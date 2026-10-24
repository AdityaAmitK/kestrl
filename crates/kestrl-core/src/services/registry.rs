//! Runtime directory and service metadata.

use std::env;
use std::fs;
use std::path::PathBuf;

use serde::Deserialize;
use serde::Serialize;

/// Metadata for a service managed by Kestrl.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Service {
    /// Service name.
    pub name: String,
    /// Supervisor child process ID.
    pub pid: u32,
    /// Command line used to start the service.
    pub command: String,
    /// Working directory for the service.
    pub cwd: String,
    /// Unix start time in milliseconds.
    pub started_at: i64,
    /// Listening TCP ports discovered for the process tree.
    pub ports: Vec<u16>,
}

/// Return the runtime directory, honoring `KESTRL_RUN_DIR`.
pub fn dir() -> std::io::Result<PathBuf> {
    if let Some(path) = env::var_os("KESTRL_RUN_DIR") {
        return Ok(path.into());
    }
    let home = env::var_os("HOME").ok_or(std::io::ErrorKind::NotFound)?;
    Ok(PathBuf::from(home).join("Library/Application Support/kestrl/run"))
}

/// Write the service metadata file.
pub fn write(service: &Service) -> std::io::Result<()> {
    let directory = dir()?;
    fs::create_dir_all(&directory)?;
    fs::write(
        directory.join(format!("{}.json", service.name)),
        serde_json::to_vec(service)?,
    )
}

/// Read all valid service metadata files.
pub fn list() -> std::io::Result<Vec<Service>> {
    let directory = dir()?;
    if !directory.exists() {
        return Ok(Vec::new());
    }
    let mut services = Vec::new();
    for entry in fs::read_dir(directory)? {
        let path = entry?.path();
        if path
            .extension()
            .is_some_and(|extension| extension == "json")
            && let Ok(service) = serde_json::from_slice(&fs::read(path)?)
        {
            services.push(service)
        }
    }
    Ok(services)
}

/// Remove a service's metadata and socket files.
pub fn remove(name: &str) {
    if let Ok(d) = dir() {
        let _ = fs::remove_file(d.join(format!("{name}.json")));
        let _ = fs::remove_file(d.join(format!("{name}.sock")));
    }
}
