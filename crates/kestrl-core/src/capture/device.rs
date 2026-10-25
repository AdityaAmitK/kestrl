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

/// Check, without changing anything, that an iOS Simulator is booted for capture.
pub fn check_simulator() -> Result<()> {
    let text = run("xcrun", &["simctl", "list", "devices", "booted"])?;
    if text.contains("(Booted)") {
        return Ok(());
    }
    Err(Error::Capture(
        "No booted iOS Simulator. Boot one in Xcode or with `xcrun simctl boot <device>`."
            .to_owned(),
    ))
}

/// Check, without changing anything, that an Android device or emulator is online for capture.
pub fn check_android() -> Result<()> {
    let text = run("adb", &["devices"])?;
    let online = text.lines().skip(1).any(|line| {
        line.split_once('\t')
            .is_some_and(|(serial, state)| !serial.is_empty() && state.trim() == "device")
    });
    if online {
        return Ok(());
    }
    Err(Error::Capture(
        "No running Android emulator. Start one in Android Studio or with `emulator -avd <name>`."
            .to_owned(),
    ))
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
