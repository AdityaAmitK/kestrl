//! Environments: `.kestrl/envs/<name>.toml`, flat string variables.

use std::collections::BTreeMap;
use std::path::Path;

use serde::Deserialize;

use crate::{Error, Result};

#[derive(Debug, Default, Deserialize)]
pub struct Environment {
    #[serde(default)]
    pub vars: BTreeMap<String, String>,
}

impl Environment {
    pub fn load(workspace: &Path, name: &str) -> Result<Self> {
        let path = workspace.join(".kestrl/envs").join(format!("{name}.toml"));
        let text = std::fs::read_to_string(&path).map_err(|source| Error::Read {
            path: path.clone(),
            source,
        })?;
        toml::from_str(&text).map_err(|source| Error::Toml { path, source })
    }

    pub fn names(workspace: &Path) -> Result<Vec<String>> {
        let dir = workspace.join(".kestrl/envs");
        let mut names = std::fs::read_dir(&dir)
            .map_err(|source| Error::Read {
                path: dir.clone(),
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
}
