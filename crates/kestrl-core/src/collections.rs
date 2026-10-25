//! Collections: folders of request files, and per-folder settings in `folder.toml`.
//!
//! A folder can add variables for every request inside it (and its subfolders) and pick a
//! default environment. `folder.toml`:
//!
//! ```toml
//! default_env = "local"          # used when no environment is chosen
//!
//! [vars]                         # any environment
//! api_url = "http://localhost:4100"
//!
//! [env.production.vars]          # only in production
//! api_url = "https://payments.example.com"
//! ```
//!
//! Precedence when sending: `--var` > folder variables (nearest folder first, environment-specific
//! before general) > environment > workspace.

use std::collections::BTreeMap;
use std::path::Component;
use std::path::Path;
use std::path::PathBuf;

use toml_edit::Item;
use toml_edit::Table;
use toml_edit::value;

use crate::Error;
use crate::Result;
use crate::toml_files;

/// The settings file inside a collection folder.
pub const SETTINGS_FILE: &str = "folder.toml";

/// A folder's settings as written.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FolderSettings {
    pub default_env: Option<String>,
    /// Variables for every environment, in file order
    pub vars: Vec<(String, String)>,
    /// Environment-specific variables: `(environment, variables)`
    pub env_vars: Vec<(String, Vec<(String, String)>)>,
}

impl FolderSettings {
    fn is_empty(&self) -> bool {
        self.default_env.is_none()
            && self.vars.is_empty()
            && self.env_vars.iter().all(|(_, vars)| vars.is_empty())
    }
}

/// Read a folder's settings; empty when it has no `folder.toml`.
pub fn read_settings(root: &Path, folder: &str) -> Result<FolderSettings> {
    let path = folder_path(root, folder)?.join(SETTINGS_FILE);
    if !path.exists() {
        return Ok(FolderSettings::default());
    }
    let document = toml_files::read_document(&path)?;
    let env_vars = document
        .get("env")
        .and_then(Item::as_table)
        .map(|envs| {
            envs.iter()
                .filter_map(|(env, item)| {
                    let vars = item.as_table()?.get("vars")?.as_table()?;
                    Some((
                        env.to_owned(),
                        vars.iter()
                            .filter_map(|(key, item)| {
                                item.as_str().map(|text| (key.to_owned(), text.to_owned()))
                            })
                            .collect(),
                    ))
                })
                .collect()
        })
        .unwrap_or_default();
    Ok(FolderSettings {
        default_env: document
            .get("default_env")
            .and_then(Item::as_str)
            .filter(|env| !env.is_empty())
            .map(str::to_owned),
        vars: toml_files::string_pairs(&document, "vars"),
        env_vars,
    })
}

/// Save a folder's settings. Empty settings remove `folder.toml`.
pub fn save_settings(root: &Path, folder: &str, settings: &FolderSettings) -> Result<()> {
    let directory = existing_folder(root, folder)?;
    let path = directory.join(SETTINGS_FILE);
    for (name, _) in &settings.vars {
        crate::env_files::validate_variable_name(name)?;
    }
    toml_files::reject_duplicates("variable", &settings.vars)?;
    for (env, vars) in &settings.env_vars {
        toml_files::validate_file_name("environment", env)?;
        for (name, _) in vars {
            crate::env_files::validate_variable_name(name)?;
        }
        toml_files::reject_duplicates("variable", vars)?;
    }
    if settings.is_empty() {
        if path.exists() {
            std::fs::remove_file(&path).map_err(|source| Error::Write { path, source })?;
        }
        return Ok(());
    }
    let mut document = toml_files::read_document_or_new(&path)?;
    match &settings.default_env {
        Some(env) => document["default_env"] = value(env.as_str()),
        None => {
            document.remove("default_env");
        }
    }
    if settings.vars.is_empty() {
        document.remove("vars");
    } else {
        toml_files::sync_string_table(
            toml_files::table_mut(&mut document, "vars", &path)?,
            &settings.vars,
        );
    }
    let kept: Vec<&(String, Vec<(String, String)>)> = settings
        .env_vars
        .iter()
        .filter(|(_, vars)| !vars.is_empty())
        .collect();
    if kept.is_empty() {
        document.remove("env");
    } else {
        let envs = toml_files::table_mut(&mut document, "env", &path)?;
        envs.set_implicit(true);
        let removed: Vec<String> = envs
            .iter()
            .map(|(env, _)| env.to_owned())
            .filter(|env| !kept.iter().any(|(name, _)| name == env))
            .collect();
        for env in removed {
            envs.remove(&env);
        }
        for (env, vars) in kept {
            if !envs.get(env).is_some_and(Item::is_table) {
                let mut entry = Table::new();
                entry.set_implicit(true);
                envs.insert(env, Item::Table(entry));
            }
            if let Some(entry) = envs.get_mut(env).and_then(Item::as_table_mut) {
                if !entry.contains_key("vars") {
                    entry.insert("vars", Item::Table(Table::new()));
                }
                if let Some(table) = entry.get_mut("vars").and_then(Item::as_table_mut) {
                    toml_files::sync_string_table(table, vars);
                }
            }
        }
    }
    toml_files::write_atomic(&path, &document.to_string())
}

/// Variables the folders around `request` add for `env`, nearest folder winning.
pub fn variables_for(root: &Path, request: &str, env: &str) -> Result<BTreeMap<String, String>> {
    let mut merged = BTreeMap::new();
    for folder in ancestor_folders(request) {
        let settings = read_settings(root, &folder)?;
        merged.extend(settings.vars);
        if let Some((_, vars)) = settings.env_vars.into_iter().find(|(name, _)| name == env) {
            merged.extend(vars);
        }
    }
    Ok(merged)
}

/// The nearest folder `default_env` around `request`, if any folder sets one.
pub fn default_env_for(root: &Path, request: &str) -> Result<Option<String>> {
    let mut found = None;
    for folder in ancestor_folders(request) {
        if let Some(env) = read_settings(root, &folder)?.default_env {
            found = Some(env);
        }
    }
    Ok(found)
}

/// Create a folder (and its parents) inside the workspace.
pub fn create_folder(root: &Path, folder: &str) -> Result<()> {
    let path = folder_path(root, folder)?;
    if path.exists() {
        return Err(Error::NameTaken {
            kind: "folder",
            name: folder.to_owned(),
        });
    }
    std::fs::create_dir_all(&path).map_err(|source| Error::Write { path, source })
}

/// Create a request file named after `name` in `folder`; returns its workspace-relative path.
pub fn create_request(root: &Path, folder: &str, name: &str, method: &str) -> Result<String> {
    let name = name.trim();
    if name.is_empty() {
        return Err(Error::InvalidName {
            kind: "request",
            name: name.to_owned(),
            rule: "a request needs a name",
        });
    }
    let directory = folder_path(root, folder)?;
    let stem = slug(name);
    let relative = unused_request_path(root, folder, &stem);
    let contents = format!(
        "name = {}\nmethod = \"{}\"\nurl = \"{{{{api_url}}}}/\"\n",
        toml_edit::Value::from(name),
        method.to_ascii_uppercase()
    );
    std::fs::create_dir_all(&directory).map_err(|source| Error::Write {
        path: directory.clone(),
        source,
    })?;
    toml_files::write_atomic(&root.join(&relative), &contents)?;
    Ok(relative)
}

/// Copy a request file next to itself as "<name> copy"; returns the new path.
pub fn duplicate_request(root: &Path, request: &str) -> Result<String> {
    let source = item_path(root, request)?;
    let folder = parent_folder(request);
    let stem = request
        .rsplit('/')
        .next()
        .unwrap_or(request)
        .trim_end_matches(".req.toml");
    let relative = unused_request_path(root, &folder, &format!("{stem}-copy"));
    let mut document = toml_files::read_document(&source)?;
    if let Some(name) = document
        .get("name")
        .and_then(Item::as_str)
        .map(str::to_owned)
    {
        document["name"] = value(format!("{name} copy"));
    }
    toml_files::write_atomic(&root.join(&relative), &document.to_string())?;
    Ok(relative)
}

/// Move or rename a request file or folder inside the workspace.
pub fn move_item(root: &Path, from: &str, to: &str) -> Result<()> {
    let source = item_path(root, from)?;
    if !source.exists() {
        return Err(Error::NotFound {
            kind: "file",
            name: from.to_owned(),
        });
    }
    let target = item_path(root, to)?;
    if source.is_file() && !to.ends_with(".req.toml") {
        return Err(Error::InvalidName {
            kind: "request file",
            name: to.to_owned(),
            rule: "request files end in .req.toml",
        });
    }
    if target.exists() {
        return Err(Error::NameTaken {
            kind: "file",
            name: to.to_owned(),
        });
    }
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent).map_err(|source| Error::Write {
            path: parent.to_owned(),
            source,
        })?;
    }
    std::fs::rename(&source, &target).map_err(|source| Error::Write {
        path: target,
        source,
    })
}

/// Folders the outline should show: every folder holding request files, and every folder under
/// `requests/`, so new empty collections appear. Workspace-relative, sorted.
pub fn folders(root: &Path, request_files: &[String]) -> Result<Vec<String>> {
    let mut found = std::collections::BTreeSet::new();
    for request in request_files {
        found.extend(ancestor_folders(request));
    }
    let requests_dir = root.join("requests");
    if requests_dir.is_dir() {
        found.insert("requests".to_owned());
        collect_folders(root, &requests_dir, &mut found)?;
    }
    Ok(found.into_iter().collect())
}

fn collect_folders(
    root: &Path,
    directory: &Path,
    found: &mut std::collections::BTreeSet<String>,
) -> Result<()> {
    let entries = std::fs::read_dir(directory).map_err(|source| Error::Read {
        path: directory.to_owned(),
        source,
    })?;
    for entry in entries.flatten() {
        let path = entry.path();
        let is_hidden = entry.file_name().to_string_lossy().starts_with('.');
        if path.is_dir() && !is_hidden {
            if let Ok(relative) = path.strip_prefix(root) {
                found.insert(relative.to_string_lossy().replace('\\', "/"));
            }
            collect_folders(root, &path, found)?;
        }
    }
    Ok(())
}

/// `a/b/c.req.toml` → `["a", "a/b"]`, outermost first.
fn ancestor_folders(request: &str) -> Vec<String> {
    let parts: Vec<&str> = request.split('/').collect();
    (1..parts.len())
        .map(|count| parts[..count].join("/"))
        .collect()
}

fn parent_folder(path: &str) -> String {
    path.rsplit_once('/')
        .map(|(folder, _)| folder.to_owned())
        .unwrap_or_default()
}

fn unused_request_path(root: &Path, folder: &str, stem: &str) -> String {
    let prefix = if folder.is_empty() {
        String::new()
    } else {
        format!("{folder}/")
    };
    let mut candidate = format!("{prefix}{stem}.req.toml");
    let mut number = 2;
    while root.join(&candidate).exists() {
        candidate = format!("{prefix}{stem}-{number}.req.toml");
        number += 1;
    }
    candidate
}

/// `Create user` → `create-user`
fn slug(name: &str) -> String {
    let mut slug = String::new();
    for character in name.chars() {
        if character.is_ascii_alphanumeric() {
            slug.push(character.to_ascii_lowercase());
        } else if !slug.ends_with('-') && !slug.is_empty() {
            slug.push('-');
        }
    }
    let slug = slug.trim_end_matches('-').to_owned();
    if slug.is_empty() {
        "request".to_owned()
    } else {
        slug
    }
}

/// A workspace-relative path that stays inside the workspace and outside `.kestrl/`.
fn item_path(root: &Path, relative: &str) -> Result<PathBuf> {
    let candidate = Path::new(relative);
    let is_safe = !relative.is_empty()
        && candidate
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
        && !relative.starts_with(".kestrl")
        && !relative
            .split('/')
            .any(|part| part.is_empty() || part.starts_with('.'));
    if is_safe {
        Ok(root.join(candidate))
    } else {
        Err(Error::InvalidName {
            kind: "path",
            name: relative.to_owned(),
            rule: "use a path inside the workspace, without .. or hidden folders",
        })
    }
}

fn folder_path(root: &Path, folder: &str) -> Result<PathBuf> {
    if folder.is_empty() {
        return Ok(root.to_owned());
    }
    item_path(root, folder)
}

fn existing_folder(root: &Path, folder: &str) -> Result<PathBuf> {
    let path = folder_path(root, folder)?;
    if path.is_dir() {
        Ok(path)
    } else {
        Err(Error::NotFound {
            kind: "folder",
            name: folder.to_owned(),
        })
    }
}
