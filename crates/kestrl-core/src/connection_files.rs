//! Creating and editing database connection files (`.kestrl/connections/<name>.toml`):
//! the kind, a URL per environment, and the environments where writes are allowed.
//!
//! Edits keep comments and ordering and write atomically (see `toml_files`).

use std::path::Path;
use std::path::PathBuf;

use toml_edit::Array;
use toml_edit::Item;
use toml_edit::Table;
use toml_edit::value;

use crate::Error;
use crate::Result;
use crate::toml_files;

/// Connection kinds Kestrl can query.
pub const SUPPORTED_KINDS: [&str; 1] = ["postgres"];

/// A connection as written in its file. URLs are unresolved (`{{secret:…}}` stays as is).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ConnectionSpec {
    pub kind: String,
    /// `(environment, url)` in file order
    pub urls: Vec<(String, String)>,
    pub allow_writes: Vec<String>,
}

/// Read a connection exactly as written.
pub fn read(root: &Path, name: &str) -> Result<ConnectionSpec> {
    let path = existing_path(root, name)?;
    let document = toml_files::read_document(&path)?;
    let urls = document
        .get("env")
        .and_then(Item::as_table)
        .map(|table| {
            table
                .iter()
                .filter_map(|(env, item)| {
                    item.as_table()
                        .and_then(|entry| entry.get("url"))
                        .and_then(Item::as_str)
                        .map(|url| (env.to_owned(), url.to_owned()))
                })
                .collect()
        })
        .unwrap_or_default();
    Ok(ConnectionSpec {
        kind: document
            .get("kind")
            .and_then(Item::as_str)
            .unwrap_or("postgres")
            .to_owned(),
        urls,
        allow_writes: document
            .get("allow_writes")
            .and_then(Item::as_array)
            .map(|array| {
                array
                    .iter()
                    .filter_map(|value| value.as_str().map(str::to_owned))
                    .collect()
            })
            .unwrap_or_default(),
    })
}

/// Create or update a connection. `previous` renames an existing connection first.
pub fn save(root: &Path, name: &str, previous: Option<&str>, spec: &ConnectionSpec) -> Result<()> {
    if !SUPPORTED_KINDS.contains(&spec.kind.as_str()) {
        return Err(Error::UnsupportedConnectionKind(spec.kind.clone()));
    }
    for (env, url) in &spec.urls {
        toml_files::validate_file_name("environment", env)?;
        if url.trim().is_empty() {
            return Err(Error::InvalidName {
                kind: "connection URL",
                name: env.clone(),
                rule: "each environment needs a URL",
            });
        }
    }
    toml_files::reject_duplicates("environment", &spec.urls)?;
    let target = path(root, name)?;
    match previous {
        Some(old) if old != name => {
            if target.exists() {
                return Err(taken(name));
            }
            let source = existing_path(root, old)?;
            std::fs::rename(&source, &target).map_err(|source| Error::Write {
                path: target.clone(),
                source,
            })?;
        }
        Some(_) => {}
        None if target.exists() => return Err(taken(name)),
        None => {}
    }
    let mut document = toml_files::read_document_or_new(&target)?;
    document["kind"] = value(spec.kind.as_str());
    document["allow_writes"] = value(
        spec.allow_writes
            .iter()
            .map(String::as_str)
            .collect::<Array>(),
    );
    let envs = toml_files::table_mut(&mut document, "env", &target)?;
    envs.set_implicit(true);
    let removed: Vec<String> = envs
        .iter()
        .map(|(env, _)| env.to_owned())
        .filter(|env| !spec.urls.iter().any(|(name, _)| name == env))
        .collect();
    for env in removed {
        envs.remove(&env);
    }
    for (env, url) in &spec.urls {
        if !envs.get(env).is_some_and(Item::is_table) {
            envs.insert(env, Item::Table(Table::new()));
        }
        if let Some(entry) = envs.get_mut(env).and_then(Item::as_table_mut) {
            entry["url"] = value(url.as_str());
        }
    }
    toml_files::write_atomic(&target, &document.to_string())
}

/// Delete a connection file.
pub fn delete(root: &Path, name: &str) -> Result<()> {
    let path = existing_path(root, name)?;
    std::fs::remove_file(&path).map_err(|source| Error::Write { path, source })
}

fn path(root: &Path, name: &str) -> Result<PathBuf> {
    toml_files::validate_file_name("connection", name)?;
    Ok(root
        .join(".kestrl/connections")
        .join(format!("{name}.toml")))
}

fn existing_path(root: &Path, name: &str) -> Result<PathBuf> {
    let path = path(root, name)?;
    if path.exists() {
        Ok(path)
    } else {
        Err(Error::UnknownConnection(name.to_owned()))
    }
}

fn taken(name: &str) -> Error {
    Error::NameTaken {
        kind: "connection",
        name: name.to_owned(),
    }
}
