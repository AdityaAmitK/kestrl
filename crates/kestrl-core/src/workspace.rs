//! Workspace discovery and `.kestrl/kestrl.toml` configuration.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::env::Environment;
use crate::history::RetentionOptions;
use crate::secrets::Redactor;
use crate::vars::Variables;
use crate::{Error, Result};

#[derive(Debug, Default, Deserialize)]
pub struct WorkspaceConfig {
    pub default_env: Option<String>,
    #[serde(default)]
    pub vars: BTreeMap<String, String>,
    #[serde(default)]
    pub retention: RetentionOptions,
}

/// The workspace containing the current directory.
pub fn find_current() -> Result<PathBuf> {
    find_root(&std::env::current_dir()?)
}

pub fn find_root(from: &Path) -> Result<PathBuf> {
    for path in from.ancestors() {
        if path.join(".kestrl").is_dir() {
            return Ok(path.to_owned());
        }
    }
    Err(Error::WorkspaceNotFound(from.to_owned()))
}

/// Load the selected environment and workspace variables for a command.
pub fn load_variables(root: &Path, env: Option<&str>) -> Result<(String, Variables)> {
    let config = WorkspaceConfig::load(root)?;
    let env_name = env
        .or(config.default_env.as_deref())
        .unwrap_or("local")
        .to_owned();
    let environment = match Environment::load(root, &env_name) {
        Ok(environment) => environment.vars,
        Err(Error::Read { source, .. }) if source.kind() == std::io::ErrorKind::NotFound => {
            BTreeMap::new()
        }
        Err(error) => return Err(error),
    };
    Ok((
        env_name,
        Variables {
            environment,
            workspace: config.vars,
            redactor: Redactor::default(),
            ..Variables::default()
        },
    ))
}

/// Directories that never hold request files.
const SKIPPED_DIRS: [&str; 3] = ["target", "node_modules", ".git"];

/// Every `*.req.toml` under `root`, as paths relative to it, sorted.
pub fn request_files(root: &Path) -> Result<Vec<String>> {
    let mut found = Vec::new();
    collect_request_files(root, root, &mut found)?;
    found.sort();
    Ok(found)
}

fn collect_request_files(root: &Path, dir: &Path, found: &mut Vec<String>) -> Result<()> {
    let entries = std::fs::read_dir(dir).map_err(|source| Error::Read {
        path: dir.to_owned(),
        source,
    })?;
    for entry in entries {
        let path = entry?.path();
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default();
        if path.is_dir() {
            if !SKIPPED_DIRS.contains(&name) {
                collect_request_files(root, &path, found)?;
            }
        } else if name.ends_with(".req.toml") {
            found.push(
                path.strip_prefix(root)
                    .unwrap_or(&path)
                    .display()
                    .to_string(),
            );
        }
    }
    Ok(())
}

/// Names of the `.toml` files in `dir`. A missing directory has none.
pub(crate) fn names(dir: &Path) -> Result<Vec<String>> {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(source) => {
            return Err(Error::Read {
                path: dir.to_owned(),
                source,
            });
        }
    };
    let mut names = entries
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "toml")
        })
        .filter_map(|path| {
            path.file_stem()
                .and_then(|stem| stem.to_str())
                .map(str::to_owned)
        })
        .collect::<Vec<_>>();
    names.sort();
    Ok(names)
}

impl WorkspaceConfig {
    pub fn load(root: &Path) -> Result<Self> {
        let path = root.join(".kestrl/kestrl.toml");
        match std::fs::read_to_string(&path) {
            Ok(text) => toml::from_str(&text).map_err(|source| Error::Toml { path, source }),
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(source) => Err(Error::Read { path, source }),
        }
    }
}
