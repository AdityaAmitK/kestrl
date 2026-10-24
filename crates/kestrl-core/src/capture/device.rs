//! Simulator and Android emulator proxy configuration.

use std::process::Command;

use crate::{Error, Result};

fn run(program: &str, args: &[&str]) -> Result<String> {
    let output = Command::new(program).args(args).output().map_err(|error| {
        Error::Capture(format!(
            "{program} is required but could not be started: {error}"
        ))
    })?;
    if !output.status.success() {
        return Err(Error::Capture(
            String::from_utf8_lossy(&output.stderr).trim().to_owned(),
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

/// Add the Kestrl root certificate to the booted iOS Simulator.
pub fn trust_simulator(certificate: &str) -> Result<()> {
    run(
        "xcrun",
        &["simctl", "keychain", "booted", "add-root-cert", certificate],
    )?;
    Ok(())
}

/// Route the Android emulator through Kestrl's local proxy.
pub fn start_android(port: u16) -> Result<()> {
    run(
        "adb",
        &[
            "shell",
            "settings",
            "put",
            "global",
            "http_proxy",
            &format!("10.0.2.2:{port}"),
        ],
    )?;
    Ok(())
}

/// Disable the Android emulator's global proxy.
pub fn stop_android() -> Result<()> {
    run(
        "adb",
        &["shell", "settings", "put", "global", "http_proxy", ":0"],
    )?;
    Ok(())
}

/// Push the root certificate to a booted Android emulator.
pub fn trust_android(certificate: &str) -> Result<()> {
    run(
        "adb",
        &["push", certificate, "/sdcard/Download/kestrl-ca.pem"],
    )?;
    Ok(())
}
