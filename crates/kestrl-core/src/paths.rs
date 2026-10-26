//! Kestrl's per-user data directory, shared by the CLI and the app.

use std::path::PathBuf;

use directories::ProjectDirs;

use crate::{Error, Result};

/// Return the data directory, honoring `KESTRL_DATA_DIR`. It is not created here.
pub fn data_dir() -> Result<PathBuf> {
    match std::env::var_os("KESTRL_DATA_DIR") {
        Some(path) => Ok(PathBuf::from(path)),
        None => Ok(ProjectDirs::from("", "", "kestrl")
            .ok_or(Error::NoDataDir)?
            .data_dir()
            .to_path_buf()),
    }
}
