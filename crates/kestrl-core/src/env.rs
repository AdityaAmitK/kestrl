//! Environments: `.kestrl/envs/<name>.toml`, flat string variables.

use std::collections::BTreeMap;
use std::path::Path;

use serde::Deserialize;

use crate::workspace;
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
        workspace::names(&workspace.join(".kestrl/envs"))
    }
}
