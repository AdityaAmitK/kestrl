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

/// One editor completion. Secret values never enter this record.
#[derive(Debug)]
pub struct VariableInfo {
    pub name: String,
    pub source: String,
    pub value: String,
    pub is_secret: bool,
    pub is_set: bool,
}

/// Effective environment/workspace variables and referenced secret names.
/// Preview expansion masks secrets before reading them; unresolved values remain templates.
/// With `request`, variables from that request's collection folders are included and win.
pub fn variable_catalog(
    root: &Path,
    env: Option<&str>,
    request: Option<&str>,
) -> Result<Vec<VariableInfo>> {
    let (env_name, mut vars) = workspace::load_variables(root, env)?;
    if let Some(request) = request {
        vars.collection = crate::collections::variables_for(root, request, &env_name)?;
    }
    let mut effective = vars.workspace.clone();
    effective.extend(vars.environment.clone());
    effective.extend(vars.collection.clone());
    let stored = crate::secrets::stored_names()?;
    let statuses: BTreeMap<String, bool> = crate::secrets::referenced_in_workspace(root)?
        .into_iter()
        .map(|name| {
            let is_set = stored.contains(&name);
            (name, is_set)
        })
        .collect();
    let mut catalog = Vec::new();
    for (name, raw) in effective {
        let template = vars.preview_template(&raw).unwrap_or(raw);
        let secret_names = crate::secrets::referenced_names(&template);
        let mut redactor = crate::secrets::Redactor::default();
        for secret in &secret_names {
            redactor.track(secret, &format!("{{{{secret:{secret}}}}}"));
        }
        let value = redactor.redact(&template);
        catalog.push(VariableInfo {
            source: if vars.collection.contains_key(&name) {
                "folder"
            } else if vars.environment.contains_key(&name) {
                "environment"
            } else {
                "workspace"
            }
            .to_owned(),
            name,
            is_secret: !secret_names.is_empty(),
            value,
            is_set: secret_names
                .iter()
                .all(|name| statuses.get(name) == Some(&true)),
        });
    }
    for (name, is_set) in statuses {
        catalog.push(VariableInfo {
            is_set,
            name: format!("secret:{name}"),
            source: "secrets".to_owned(),
            value: crate::secrets::MASK.to_owned(),
            is_secret: true,
        });
    }
    Ok(catalog)
}
