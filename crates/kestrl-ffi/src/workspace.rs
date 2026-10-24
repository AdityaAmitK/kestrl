//! Opening a workspace: its root, environments, profiles and request files.

use std::path::Path;

use kestrl_core::env::Environment;
use kestrl_core::profile;
use kestrl_core::workspace::{self, WorkspaceConfig};

use crate::error::KestrlError;

/// What the app shows for an open workspace.
#[derive(Debug, uniffi::Record)]
pub struct WorkspaceInfo {
    pub root: String,
    pub default_env: Option<String>,
    pub envs: Vec<String>,
    pub profiles: Vec<String>,
    /// Request files, relative to `root`
    pub request_files: Vec<String>,
}

/// Open the workspace that contains `path`.
#[uniffi::export]
pub fn open_workspace(path: String) -> Result<WorkspaceInfo, KestrlError> {
    let root = workspace::find_root(Path::new(&path))?;
    let config = WorkspaceConfig::load(&root)?;
    Ok(WorkspaceInfo {
        root: root.display().to_string(),
        default_env: config.default_env,
        envs: Environment::names(&root)?,
        profiles: profile::names(&root)?,
        request_files: workspace::request_files(&root)?,
    })
}
