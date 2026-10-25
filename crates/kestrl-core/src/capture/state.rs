//! Private runtime files for device capture: the run directory, atomic writes, and the lock that
//! gives one device capture session ownership of device proxy settings.

use std::fs::{File, OpenOptions, TryLockError};
use std::io::Write;
use std::path::{Path, PathBuf};

use directories::ProjectDirs;

use crate::{Error, Result};

/// Lock file that serializes device capture sessions across processes.
const LOCK_FILE: &str = "device-capture.lock";

/// Return the private run directory, honoring `KESTRL_RUN_DIR`.
pub(crate) fn run_dir() -> Result<PathBuf> {
    let dir = match std::env::var_os("KESTRL_RUN_DIR") {
        Some(path) => PathBuf::from(path),
        None => {
            let project = ProjectDirs::from("", "", "kestrl").ok_or(Error::NoDataDir)?;
            project
                .runtime_dir()
                .map(PathBuf::from)
                .unwrap_or_else(|| project.data_dir().join("run"))
        }
    };
    std::fs::create_dir_all(&dir)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700))?;
    }
    Ok(dir)
}

/// Write `bytes` to `path` with owner-only permissions. The file is replaced in one rename, so a
/// crash leaves either the old contents or the new ones.
pub(crate) fn write_private(path: &Path, bytes: &[u8]) -> Result<()> {
    let temporary = path.with_extension("json.tmp");
    let _ = std::fs::remove_file(&temporary);
    let mut options = OpenOptions::new();
    options.create_new(true).write(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(&temporary)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    drop(file);
    std::fs::rename(&temporary, path)?;
    Ok(())
}

/// Exclusive ownership of device proxy settings. Held for the whole session, including cleanup,
/// and released when dropped or when the process exits. The lock file is never deleted, so a
/// crashed owner cannot leave a stale lock behind.
pub(crate) struct DeviceLock {
    _file: File,
}

impl DeviceLock {
    /// Take the lock without waiting. Fails when another device capture holds it.
    pub(crate) fn acquire() -> Result<Self> {
        let path = run_dir()?.join(LOCK_FILE);
        let file = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .truncate(false)
            .open(&path)?;
        match file.try_lock() {
            Ok(()) => Ok(Self { _file: file }),
            Err(TryLockError::WouldBlock) => Err(Error::Capture(
                "another device capture is already running; stop it before starting another"
                    .to_owned(),
            )),
            Err(TryLockError::Error(error)) => Err(error.into()),
        }
    }
}
