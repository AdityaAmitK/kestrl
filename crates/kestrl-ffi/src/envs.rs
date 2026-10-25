//! Resolved environments for the app. Secret values are masked.

use std::path::Path;

use kestrl_core::env::Environment;

use crate::error::KestrlError;
use crate::types::KeyValue;

/// The variables of `env` with every reference expanded and secrets masked.
#[uniffi::export]
pub fn resolved_env(workspace_root: String, env: String) -> Result<Vec<KeyValue>, KestrlError> {
    let vars = Environment::resolved(Path::new(&workspace_root), &env)?;
    Ok(KeyValue::from_map(vars))
}

/// An editor variable with a safe preview, never a resolved secret.
#[derive(Debug, uniffi::Record)]
pub struct VariableInfo {
    pub name: String,
    pub source: String,
    pub value: String,
    pub is_secret: bool,
    pub is_set: bool,
}

/// Effective variables and referenced secrets for request editing.
#[uniffi::export]
pub fn variable_catalog(
    workspace_root: String,
    env: Option<String>,
) -> Result<Vec<VariableInfo>, KestrlError> {
    Ok(
        kestrl_core::env::variable_catalog(Path::new(&workspace_root), env.as_deref())?
            .into_iter()
            .map(|info| VariableInfo {
                name: info.name,
                source: info.source,
                value: info.value,
                is_secret: info.is_secret,
                is_set: info.is_set,
            })
            .collect(),
    )
}
