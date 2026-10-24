//! Transactional macOS system proxy changes with crash recovery.

use std::path::PathBuf;
use std::process::Command;

use directories::ProjectDirs;
use serde::{Deserialize, Serialize};

use crate::{Error, Result};

#[derive(Debug, Serialize, Deserialize)]
struct ProxyState {
    settings: Vec<ServiceSettings>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ServiceSettings {
    service: String,
    web_enabled: String,
    web_server: String,
    web_port: String,
    secure_enabled: String,
    secure_server: String,
    secure_port: String,
}

fn state_path() -> Result<PathBuf> {
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
    Ok(dir.join("system-proxy.json"))
}

fn output(arguments: &[&str]) -> Result<String> {
    let result = Command::new("networksetup").args(arguments).output()?;
    if !result.status.success() {
        return Err(Error::Capture(
            String::from_utf8_lossy(&result.stderr).trim().to_owned(),
        ));
    }
    Ok(String::from_utf8_lossy(&result.stdout).trim().to_owned())
}

fn parse_enabled(value: &str) -> String {
    if value.eq_ignore_ascii_case("yes") || value.eq_ignore_ascii_case("on") {
        "on"
    } else {
        "off"
    }
    .to_owned()
}

fn services() -> Result<Vec<String>> {
    let text = output(&["-listnetworkserviceorder"])?;
    Ok(text
        .lines()
        .filter_map(|line| {
            if line.trim_start().starts_with('*') {
                return None;
            }
            let line = line.trim();
            let (index, name) = line.strip_prefix('(')?.split_once(')')?;
            index.parse::<u32>().ok().map(|_| name.trim().to_owned())
        })
        .filter(|service| !service.is_empty())
        .collect())
}

fn proxy_address(service: &str, kind: &str) -> Result<(String, String, String)> {
    let text = output(&[kind, service])?;
    let fields = text
        .lines()
        .filter_map(|line| line.split_once(':'))
        .map(|(key, value)| (key.trim(), value.trim()))
        .collect::<std::collections::HashMap<_, _>>();
    Ok((
        parse_enabled(fields.get("Enabled").copied().unwrap_or("No")),
        fields.get("Server").copied().unwrap_or("").to_owned(),
        fields.get("Port").copied().unwrap_or("0").to_owned(),
    ))
}

/// Enable HTTP and HTTPS system proxies and persist the old state first.
pub fn enable(port: u16) -> Result<()> {
    restore()?;
    let settings = services()?
        .into_iter()
        .map(|service| {
            let (web_enabled, web_server, web_port) = proxy_address(&service, "-getwebproxy")?;
            let (secure_enabled, secure_server, secure_port) =
                proxy_address(&service, "-getsecurewebproxy")?;
            Ok(ServiceSettings {
                service,
                web_enabled,
                web_server,
                web_port,
                secure_enabled,
                secure_server,
                secure_port,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let path = state_path()?;
    std::fs::write(&path, serde_json::to_vec_pretty(&ProxyState { settings })?)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
    }
    let state: ProxyState = serde_json::from_slice(&std::fs::read(&path)?)?;
    for entry in state.settings {
        let change = (|| {
            output(&[
                "-setwebproxy",
                &entry.service,
                "127.0.0.1",
                &port.to_string(),
            ])?;
            output(&["-setwebproxystate", &entry.service, "on"])?;
            output(&[
                "-setsecurewebproxy",
                &entry.service,
                "127.0.0.1",
                &port.to_string(),
            ])?;
            output(&["-setsecurewebproxystate", &entry.service, "on"])?;
            Ok::<(), Error>(())
        })();
        if let Err(error) = change {
            let _ = restore();
            return Err(error);
        }
    }
    Ok(())
}

/// Restore saved system proxy settings, including settings left by a crash.
pub fn restore() -> Result<()> {
    let path = state_path()?;
    if !path.exists() {
        return Ok(());
    }
    let state: ProxyState = serde_json::from_slice(&std::fs::read(&path)?)?;
    for entry in state.settings {
        output(&[
            "-setwebproxy",
            &entry.service,
            &entry.web_server,
            &entry.web_port,
        ])?;
        output(&["-setwebproxystate", &entry.service, &entry.web_enabled])?;
        output(&[
            "-setsecurewebproxy",
            &entry.service,
            &entry.secure_server,
            &entry.secure_port,
        ])?;
        output(&[
            "-setsecurewebproxystate",
            &entry.service,
            &entry.secure_enabled,
        ])?;
    }
    std::fs::remove_file(path)?;
    Ok(())
}
