//! Environments: `.kestrl/envs/<name>.toml`, flat string variables.

use std::collections::BTreeMap;
use std::path::Path;

use serde::Deserialize;

use crate::vars::Variables;
use crate::workspace::{self, WorkspaceConfig};
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

    /// The variables with every reference expanded and secret values masked.
    /// Workspace variables are the fallback for names the environment lacks.
    pub fn resolved(workspace: &Path, name: &str) -> Result<BTreeMap<String, String>> {
        let config = WorkspaceConfig::load(workspace)?;
        let mut vars = Variables {
            workspace: config.vars,
            environment: Self::load(workspace, name)?.vars,
            ..Variables::default()
        };
        let raw = vars.environment.clone();
        raw.into_iter()
            .map(|(key, value)| {
                let value = vars.resolve(&value)?;
                Ok((key, vars.redactor.redact(&value)))
            })
            .collect()
    }
}
