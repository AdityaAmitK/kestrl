//! Transactional macOS system proxy changes with crash recovery.

use std::path::PathBuf;
use std::process::Command;

use serde::{Deserialize, Serialize};

use super::state;
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
    Ok(state::run_dir()?.join("system-proxy.json"))
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
    let mut services = Vec::new();
    let mut candidate: Option<String> = None;
    let mut has_device = false;
    let mut enabled = false;
    for line in text.lines() {
        let trimmed = line.trim();
        let heading = trimmed
            .strip_prefix('(')
            .and_then(|rest| rest.split_once(')'))
            .and_then(|(index, name)| index.parse::<u32>().ok().map(|_| name));
        if let Some(name) = heading {
            if let Some(previous) = candidate.take()
                && has_device
                && enabled
            {
                services.push(previous);
            }
            let disabled = name.trim_start().starts_with('*');
            let name = name.trim().trim_start_matches('*').trim().to_owned();
            candidate = Some(name);
            has_device = false;
            enabled = !disabled;
        } else if candidate.is_some()
            && let Some((_, device)) = trimmed.split_once("Device:")
        {
            has_device = !device.trim().trim_end_matches(')').trim().is_empty();
        }
    }
    if let Some(name) = candidate
        && has_device
        && enabled
    {
        services.push(name);
    }
    Ok(services)
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
    let bytes = serde_json::to_vec_pretty(&ProxyState { settings })?;
    state::write_private(&path, &bytes)?;
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
    let mut failed = Vec::new();
    for entry in state.settings {
        let mut service_failures = Vec::new();
        if !entry.web_server.is_empty()
            && let Err(error) = output(&[
                "-setwebproxy",
                &entry.service,
                &entry.web_server,
                &entry.web_port,
            ])
        {
            service_failures.push(format!("web proxy: {error}"));
        }
        if let Err(error) = output(&["-setwebproxystate", &entry.service, &entry.web_enabled]) {
            service_failures.push(format!("web proxy state: {error}"));
        }
        if !entry.secure_server.is_empty()
            && let Err(error) = output(&[
                "-setsecurewebproxy",
                &entry.service,
                &entry.secure_server,
                &entry.secure_port,
            ])
        {
            service_failures.push(format!("secure proxy: {error}"));
        }
        if let Err(error) = output(&[
            "-setsecurewebproxystate",
            &entry.service,
            &entry.secure_enabled,
        ]) {
            service_failures.push(format!("secure proxy state: {error}"));
        }
        if !service_failures.is_empty() {
            failed.push(format!(
                "{} ({})",
                entry.service,
                service_failures.join("; ")
            ));
        }
    }
    if !failed.is_empty() {
        return Err(Error::Capture(format!(
            "failed to restore system proxy for: {}",
            failed.join(", ")
        )));
    }
    std::fs::remove_file(path)?;
    Ok(())
}

/// Restores system proxy settings if a capture exits early or unwinds.
pub struct SystemProxyGuard;

impl SystemProxyGuard {
    /// Create a guard after the system proxy has been enabled.
    pub fn new() -> Self {
        Self
    }
}

impl Default for SystemProxyGuard {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for SystemProxyGuard {
    fn drop(&mut self) {
        let _ = restore();
    }
}
