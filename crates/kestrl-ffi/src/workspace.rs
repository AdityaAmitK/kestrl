//! Opening a workspace: its root, environments, profiles, request files and settings.

use std::path::Path;

use kestrl_core::env::Environment;
use kestrl_core::history;
use kestrl_core::profile;
use kestrl_core::request;
use kestrl_core::workspace::{self, WorkspaceConfig};

use crate::error::KestrlError;

/// What the app shows for an open workspace.
#[derive(Debug, uniffi::Record)]
pub struct WorkspaceInfo {
    pub root: String,
    pub default_env: Option<String>,
    pub envs: Vec<String>,
    pub profiles: Vec<String>,
    pub requests: Vec<RequestEntry>,
    /// Collection folders, workspace-relative, including empty ones under `requests/`
    pub folders: Vec<String>,
    pub settings: WorkspaceSettings,
}

/// One request file. `method` and `name` are unset when the file does not parse.
#[derive(Debug, uniffi::Record)]
pub struct RequestEntry {
    /// Relative to the workspace root
    pub path: String,
    pub method: Option<String>,
    pub name: Option<String>,
}

/// Read-only settings shown in the Settings window.
#[derive(Debug, uniffi::Record)]
pub struct WorkspaceSettings {
    pub history_db: String,
    pub max_age_days: u32,
    pub max_events: u32,
    pub max_body_bytes: u64,
}

/// Open the workspace that contains `path`.
#[uniffi::export]
pub fn open_workspace(path: String) -> Result<WorkspaceInfo, KestrlError> {
    let root = workspace::find_root(Path::new(&path))?;
    let config = WorkspaceConfig::load(&root)?;
    let retention = config.retention;
    let request_paths = workspace::request_files(&root)?;
    let folders = kestrl_core::collections::folders(&root, &request_paths)?;
    let requests = request_paths
        .into_iter()
        .map(|path| {
            let loaded = request::load(&root.join(&path)).ok();
            RequestEntry {
                method: loaded.as_ref().map(|file| file.method.clone()),
                name: loaded.and_then(|file| file.name),
                path,
            }
        })
        .collect();
    Ok(WorkspaceInfo {
        root: root.display().to_string(),
        default_env: config.default_env,
        envs: Environment::names(&root)?,
        profiles: profile::names(&root)?,
        requests,
        folders,
        settings: WorkspaceSettings {
            history_db: history::default_path()?.display().to_string(),
            max_age_days: retention.max_age_days,
            max_events: retention.max_events,
            max_body_bytes: u64::try_from(retention.max_body_bytes).unwrap_or(u64::MAX),
        },
    })
}

/// Create a starter workspace at exactly `path`, then open it.
#[uniffi::export]
pub fn create_workspace(path: String) -> Result<WorkspaceInfo, KestrlError> {
    workspace::init(Path::new(&path))?;
    open_workspace(path)
}
