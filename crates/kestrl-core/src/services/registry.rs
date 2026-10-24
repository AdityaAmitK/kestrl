//! Runtime directory and service metadata.
use serde::{Deserialize, Serialize};
use std::{env, fs, path::PathBuf};
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Service {
    pub name: String,
    pub pid: u32,
    pub command: String,
    pub cwd: String,
    pub started_at: u128,
    pub ports: Vec<u16>,
}
pub fn dir() -> std::io::Result<PathBuf> {
    if let Some(p) = env::var_os("KESTRL_RUN_DIR") {
        return Ok(p.into());
    }
    let home = env::var_os("HOME").ok_or(std::io::ErrorKind::NotFound)?;
    Ok(PathBuf::from(home).join("Library/Application Support/kestrl/run"))
}
pub fn write(s: &Service) -> std::io::Result<()> {
    let d = dir()?;
    fs::create_dir_all(&d)?;
    fs::write(d.join(format!("{}.json", s.name)), serde_json::to_vec(s)?)
}
pub fn list() -> std::io::Result<Vec<Service>> {
    let d = dir()?;
    if !d.exists() {
        return Ok(Vec::new());
    }
    let mut v = Vec::new();
    for e in fs::read_dir(d)? {
        let p = e?.path();
        if p.extension().is_some_and(|x| x == "json")
            && let Ok(s) = serde_json::from_slice(&fs::read(p)?)
        {
            v.push(s)
        }
    }
    Ok(v)
}
pub fn remove(name: &str) {
    if let Ok(d) = dir() {
        let _ = fs::remove_file(d.join(format!("{name}.json")));
        let _ = fs::remove_file(d.join(format!("{name}.sock")));
    }
}
