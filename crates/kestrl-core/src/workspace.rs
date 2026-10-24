//! Workspace discovery and `.kestrl/kestrl.toml` configuration.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::history::RetentionOptions;
use crate::{Error, Result};

#[derive(Debug, Default, Deserialize)]
pub struct WorkspaceConfig {
    pub default_env: Option<String>,
    #[serde(default)]
    pub vars: BTreeMap<String, String>,
    #[serde(default)]
    pub retention: RetentionOptions,
}

pub fn find_root(from: &Path) -> Result<PathBuf> {
    for path in from.ancestors() {
        if path.join(".kestrl").is_dir() {
            return Ok(path.to_owned());
        }
    }
    Err(Error::WorkspaceNotFound(from.to_owned()))
}

pub(crate) fn names(dir: &Path) -> Result<Vec<String>> {
    let mut names = std::fs::read_dir(dir)
        .map_err(|source| Error::Read {
            path: dir.to_owned(),
            source,
        })?
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
