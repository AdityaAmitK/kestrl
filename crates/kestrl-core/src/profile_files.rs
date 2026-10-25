//! Creating and editing profile files (`.kestrl/profiles/<name>.toml`): headers and query
//! parameters added to requests, optionally only when `[when]` matches.
//!
//! Edits keep comments and ordering and write atomically (see `toml_files`).

use std::path::Path;
use std::path::PathBuf;

use toml_edit::Array;
use toml_edit::value;

use crate::Error;
use crate::Result;
use crate::toml_files;

/// A profile as written in its file.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProfileSpec {
    pub headers: Vec<(String, String)>,
    pub query: Vec<(String, String)>,
    /// Apply automatically only in these environments (empty: any environment)
    pub when_envs: Vec<String>,
    /// Apply automatically only to URLs matching this `*` pattern
    pub when_url: Option<String>,
}

impl ProfileSpec {
    /// A profile with a `[when]` block is applied automatically when it matches.
    pub fn is_automatic(&self) -> bool {
        !self.when_envs.is_empty() || self.when_url.is_some()
    }
}

/// Read a profile exactly as written.
pub fn read(root: &Path, name: &str) -> Result<ProfileSpec> {
    let path = existing_path(root, name)?;
    let document = toml_files::read_document(&path)?;
    let when = document.get("when").and_then(|item| item.as_table());
    Ok(ProfileSpec {
        headers: toml_files::string_pairs(&document, "headers"),
        query: toml_files::string_pairs(&document, "query"),
        when_envs: when
            .and_then(|table| table.get("env"))
            .and_then(|item| item.as_array())
            .map(|array| {
                array
                    .iter()
                    .filter_map(|value| value.as_str().map(str::to_owned))
                    .collect()
            })
            .unwrap_or_default(),
        when_url: when
            .and_then(|table| table.get("url"))
            .and_then(|item| item.as_str())
            .filter(|url| !url.is_empty())
            .map(str::to_owned),
    })
}

/// Create a profile, empty or as a copy of `copy_from` (comments included).
pub fn create(root: &Path, name: &str, copy_from: Option<&str>) -> Result<()> {
    let path = path(root, name)?;
    if path.exists() {
        return Err(taken(name));
    }
    let contents = match copy_from {
        Some(source) => toml_files::read_text(&existing_path(root, source)?)?,
        None => "[headers]\n".to_owned(),
    };
    toml_files::write_atomic(&path, &contents)
}

/// Save a profile's headers, query parameters and conditions.
pub fn save(root: &Path, name: &str, spec: &ProfileSpec) -> Result<()> {
    let path = existing_path(root, name)?;
    for (header, _) in &spec.headers {
        validate_header_name(header)?;
    }
    for (parameter, _) in &spec.query {
        if parameter.trim().is_empty() {
            return Err(Error::InvalidName {
                kind: "query parameter",
                name: parameter.clone(),
                rule: "a query parameter needs a name",
            });
        }
    }
    toml_files::reject_duplicates("header", &spec.headers)?;
    toml_files::reject_duplicates("query parameter", &spec.query)?;
    let mut document = toml_files::read_document(&path)?;
    toml_files::sync_string_table(
        toml_files::table_mut(&mut document, "headers", &path)?,
        &spec.headers,
    );
    if spec.query.is_empty() {
        document.remove("query");
    } else {
        toml_files::sync_string_table(
            toml_files::table_mut(&mut document, "query", &path)?,
            &spec.query,
        );
    }
    if spec.is_automatic() {
        let when = toml_files::table_mut(&mut document, "when", &path)?;
        if spec.when_envs.is_empty() {
            when.remove("env");
        } else {
            when["env"] = value(spec.when_envs.iter().map(String::as_str).collect::<Array>());
        }
        match &spec.when_url {
            Some(url) => when["url"] = value(url.as_str()),
            None => {
                when.remove("url");
            }
        }
    } else {
        document.remove("when");
    }
    toml_files::write_atomic(&path, &document.to_string())
}

/// Rename a profile. Request files that list the old name are not rewritten.
pub fn rename(root: &Path, from: &str, to: &str) -> Result<()> {
    let source = existing_path(root, from)?;
    let target = path(root, to)?;
    if from == to {
        return Ok(());
    }
    if target.exists() {
        return Err(taken(to));
    }
    std::fs::rename(&source, &target).map_err(|source| Error::Write {
        path: target,
        source,
    })
}

/// Delete a profile file.
pub fn delete(root: &Path, name: &str) -> Result<()> {
    let path = existing_path(root, name)?;
    std::fs::remove_file(&path).map_err(|source| Error::Write { path, source })
}

/// HTTP header names: visible ASCII without separators.
fn validate_header_name(name: &str) -> Result<()> {
    let is_valid = !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "!#$%&'*+-.^_`|~".contains(c));
    if is_valid {
        Ok(())
    } else {
        Err(Error::InvalidName {
            kind: "header",
            name: name.to_owned(),
            rule: "use letters, digits and - (no spaces or colons)",
        })
    }
}

fn path(root: &Path, name: &str) -> Result<PathBuf> {
    toml_files::validate_file_name("profile", name)?;
    Ok(root.join(".kestrl/profiles").join(format!("{name}.toml")))
}

fn existing_path(root: &Path, name: &str) -> Result<PathBuf> {
    let path = path(root, name)?;
    if path.exists() {
        Ok(path)
    } else {
        Err(Error::NotFound {
            kind: "profile",
            name: name.to_owned(),
        })
    }
}

fn taken(name: &str) -> Error {
    Error::NameTaken {
        kind: "profile",
        name: name.to_owned(),
    }
}
