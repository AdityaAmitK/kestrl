//! Resolved environments for the app. Secret values are masked.

use std::path::Path;

use kestrl_core::env::Environment;
use kestrl_core::env_files::{self, RawVariable};

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

/// The variables of `env` exactly as written in its file, in file order.
#[uniffi::export]
pub fn environment_variables(
    workspace_root: String,
    env: String,
) -> Result<Vec<KeyValue>, KestrlError> {
    Ok(env_files::variables(Path::new(&workspace_root), &env)?
        .into_iter()
        .map(|variable| KeyValue {
            key: variable.name,
            value: variable.value,
        })
        .collect())
}

/// Replace the variables of `env`; comments on kept variables are preserved.
#[uniffi::export]
pub fn save_environment_variables(
    workspace_root: String,
    env: String,
    variables: Vec<KeyValue>,
) -> Result<(), KestrlError> {
    let variables: Vec<RawVariable> = variables
        .into_iter()
        .map(|pair| RawVariable {
            name: pair.key,
            value: pair.value,
        })
        .collect();
    Ok(env_files::save_variables(
        Path::new(&workspace_root),
        &env,
        &variables,
    )?)
}

/// Create an environment, empty or as a copy of `copy_from`.
#[uniffi::export]
pub fn create_environment(
    workspace_root: String,
    name: String,
    copy_from: Option<String>,
) -> Result<(), KestrlError> {
    Ok(env_files::create(
        Path::new(&workspace_root),
        &name,
        copy_from.as_deref(),
    )?)
}

/// Rename an environment; the default follows the rename.
#[uniffi::export]
pub fn rename_environment(
    workspace_root: String,
    from: String,
    to: String,
) -> Result<(), KestrlError> {
    Ok(env_files::rename(Path::new(&workspace_root), &from, &to)?)
}

/// Delete a non-default environment.
#[uniffi::export]
pub fn delete_environment(workspace_root: String, name: String) -> Result<(), KestrlError> {
    Ok(env_files::delete(Path::new(&workspace_root), &name)?)
}

/// Make `name` the workspace's default environment.
#[uniffi::export]
pub fn set_default_environment(workspace_root: String, name: String) -> Result<(), KestrlError> {
    Ok(env_files::set_default(Path::new(&workspace_root), &name)?)
}

/// Check an environment or variable name before saving, for inline validation.
#[uniffi::export]
pub fn validate_name(kind: String, name: String) -> Option<String> {
    let result = match kind.as_str() {
        "environment" => env_files::validate_environment_name(&name),
        _ => env_files::validate_variable_name(&name),
    };
    result.err().map(|error| error.to_string())
}
