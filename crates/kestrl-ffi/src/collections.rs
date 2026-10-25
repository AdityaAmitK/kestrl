//! Collections for the app: folders and request files, and per-folder settings.

use std::path::Path;

use kestrl_core::collections::{self, FolderSettings};

use crate::error::KestrlError;
use crate::types::KeyValue;

/// Variables a folder adds for one environment.
#[derive(Debug, Clone, uniffi::Record)]
pub struct FolderEnvironmentVariables {
    pub environment: String,
    pub variables: Vec<KeyValue>,
}

/// A folder's `folder.toml`, as written.
#[derive(Debug, Clone, uniffi::Record)]
pub struct FolderSettingsRecord {
    pub default_env: Option<String>,
    /// Variables for every environment
    pub variables: Vec<KeyValue>,
    pub environment_variables: Vec<FolderEnvironmentVariables>,
}

#[uniffi::export]
pub fn read_folder_settings(
    workspace_root: String,
    folder: String,
) -> Result<FolderSettingsRecord, KestrlError> {
    let settings = collections::read_settings(Path::new(&workspace_root), &folder)?;
    Ok(FolderSettingsRecord {
        default_env: settings.default_env,
        variables: KeyValue::from_pairs(settings.vars),
        environment_variables: settings
            .env_vars
            .into_iter()
            .map(|(environment, vars)| FolderEnvironmentVariables {
                environment,
                variables: KeyValue::from_pairs(vars),
            })
            .collect(),
    })
}

/// Save a folder's settings; empty settings remove `folder.toml`.
#[uniffi::export]
pub fn save_folder_settings(
    workspace_root: String,
    folder: String,
    settings: FolderSettingsRecord,
) -> Result<(), KestrlError> {
    let pairs = |values: Vec<KeyValue>| -> Vec<(String, String)> {
        values
            .into_iter()
            .map(|pair| (pair.key, pair.value))
            .collect()
    };
    let settings = FolderSettings {
        default_env: settings.default_env.filter(|env| !env.is_empty()),
        vars: pairs(settings.variables),
        env_vars: settings
            .environment_variables
            .into_iter()
            .map(|entry| (entry.environment, pairs(entry.variables)))
            .collect(),
    };
    Ok(collections::save_settings(
        Path::new(&workspace_root),
        &folder,
        &settings,
    )?)
}

/// Create a folder (collection) inside the workspace.
#[uniffi::export]
pub fn create_folder(workspace_root: String, folder: String) -> Result<(), KestrlError> {
    Ok(collections::create_folder(
        Path::new(&workspace_root),
        &folder,
    )?)
}

/// Create a request file in `folder`; returns its workspace-relative path.
#[uniffi::export]
pub fn create_request(
    workspace_root: String,
    folder: String,
    name: String,
    method: String,
) -> Result<String, KestrlError> {
    Ok(collections::create_request(
        Path::new(&workspace_root),
        &folder,
        &name,
        &method,
    )?)
}

/// Duplicate a request file next to itself; returns the new path.
#[uniffi::export]
pub fn duplicate_request(workspace_root: String, path: String) -> Result<String, KestrlError> {
    Ok(collections::duplicate_request(
        Path::new(&workspace_root),
        &path,
    )?)
}

/// Move or rename a request file or folder inside the workspace.
#[uniffi::export]
pub fn move_item(workspace_root: String, from: String, to: String) -> Result<(), KestrlError> {
    Ok(collections::move_item(
        Path::new(&workspace_root),
        &from,
        &to,
    )?)
}
