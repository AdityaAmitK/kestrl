//! Simulator and Android emulator proxy configuration. The emulator's previous proxy setting is
//! saved to a private record before Kestrl changes it, so an interrupted capture can restore it.

use std::path::PathBuf;
use std::process::Command;

use serde::{Deserialize, Serialize};

use super::state;
use crate::{Error, Result};

/// Record file for the Android emulator proxy setting Kestrl changed.
const ANDROID_RECORD: &str = "android-proxy.json";
/// The emulator's address for the host's loopback interface.
const EMULATOR_HOST: &str = "10.0.2.2";
/// The global Android setting that holds the HTTP proxy.
const PROXY_SETTING: &str = "http_proxy";

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

/// Proxy setting saved before Kestrl changed it. Never printed: the value can hold credentials.
#[derive(Serialize, Deserialize)]
struct AndroidRecord {
    serial: String,
    /// The setting before capture, or `None` when it was unset.
    previous: Option<String>,
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

/// Check, without changing anything, that exactly one Android emulator is booted for capture.
pub fn check_android() -> Result<()> {
    booted_emulator().map(drop)
}

/// Every `emulator-*` device that `adb devices` lists, with its state. Physical devices are
/// skipped: the emulator's `10.0.2.2` address reaches only the host from inside an emulator.
fn android_emulators() -> Result<Vec<(String, String)>> {
    let text = run("adb", &["devices"])?;
    Ok(text
        .lines()
        .filter_map(|line| line.split_once('\t'))
        .filter(|(serial, _)| serial.starts_with("emulator-"))
        .map(|(serial, state)| (serial.to_owned(), state.trim().to_owned()))
        .collect())
}

/// The serial of the one booted Android emulator. Zero or several is an error, never a guess.
fn booted_emulator() -> Result<String> {
    let emulators = android_emulators()?;
    let booted = emulators
        .iter()
        .filter(|(_, state)| state == "device")
        .map(|(serial, _)| serial.as_str())
        .collect::<Vec<_>>();
    match booted.as_slice() {
        [serial] => Ok((*serial).to_owned()),
        [] => match emulators.first() {
            Some((serial, state)) => Err(Error::Capture(format!(
                "Android emulator {serial} is {state}. Wait for it to finish booting, or restart it."
            ))),
            None => Err(Error::Capture(
                "No running Android emulator. Start one in Android Studio or with `emulator -avd <name>`. Physical devices are not supported."
                    .to_owned(),
            )),
        },
        many => Err(Error::Capture(format!(
            "More than one Android emulator is running ({}). Close all but one, then retry.",
            many.join(", ")
        ))),
    }
}

/// Run `adb -s <serial> shell settings ...`. Failure output is dropped because it can echo the
/// proxy value; the exit status is reported instead.
fn settings(serial: &str, arguments: &[&str]) -> Result<String> {
    let output = Command::new("adb")
        .args(["-s", serial, "shell", "settings"])
        .args(arguments)
        .output()
        .map_err(|error| {
            Error::Capture(format!("adb is required but could not be started: {error}"))
        })?;
    if !output.status.success() {
        return Err(Error::Capture(format!(
            "adb settings command failed on {serial} ({})",
            output.status
        )));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

/// Quote a value for the device shell, which receives `adb shell` arguments joined by spaces.
fn quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', r"'\''"))
}

fn read_proxy(serial: &str) -> Result<Option<String>> {
    let value = settings(serial, &["get", "global", PROXY_SETTING])?;
    Ok((!value.is_empty() && value != "null").then_some(value))
}

fn record_path() -> Result<PathBuf> {
    Ok(state::run_dir()?.join(ANDROID_RECORD))
}

/// Route the booted Android emulator through Kestrl's local proxy. The previous setting is saved
/// first. If the change fails, the previous setting is put back.
pub(crate) fn start_android(port: u16) -> Result<()> {
    restore_android()?;
    let serial = booted_emulator()?;
    let previous = read_proxy(&serial)?;
    let record = AndroidRecord {
        serial: serial.clone(),
        previous,
    };
    state::write_private(&record_path()?, &serde_json::to_vec(&record)?)?;
    let proxy = format!("{EMULATOR_HOST}:{port}");
    if let Err(error) = settings(&serial, &["put", "global", PROXY_SETTING, &quote(&proxy)]) {
        // The record stays if restoring fails, so the next device capture start retries it.
        let _ = restore_android();
        return Err(error);
    }
    Ok(())
}

/// Restore the Android emulator proxy setting saved by `start_android`, then remove the record.
/// The record stays when the emulator is not running, so no state is lost.
pub fn restore_android() -> Result<()> {
    let path = record_path()?;
    if !path.exists() {
        return Ok(());
    }
    let record: AndroidRecord = serde_json::from_slice(&std::fs::read(&path)?).map_err(|_| {
        Error::Capture(format!(
            "Android proxy record {} is unreadable. Restore the emulator's {PROXY_SETTING} setting, then delete the record.",
            path.display()
        ))
    })?;
    let online = android_emulators()?
        .iter()
        .any(|(serial, state)| serial == &record.serial && state == "device");
    if !online {
        return Err(Error::Capture(format!(
            "Android emulator {} must be running to restore the proxy setting left by an interrupted capture. Start it and retry; Kestrl keeps the record until then.",
            record.serial
        )));
    }
    match &record.previous {
        Some(value) => {
            settings(
                &record.serial,
                &["put", "global", PROXY_SETTING, &quote(value)],
            )?;
        }
        None => {
            settings(&record.serial, &["delete", "global", PROXY_SETTING])?;
        }
    }
    std::fs::remove_file(path)?;
    Ok(())
}

/// Add the Kestrl root certificate to the booted iOS Simulator.
pub fn trust_simulator(certificate: &str) -> Result<()> {
    run(
        "xcrun",
        &["simctl", "keychain", "booted", "add-root-cert", certificate],
    )?;
    Ok(())
}

/// Push the root certificate to the booted Android emulator.
pub fn trust_android(certificate: &str) -> Result<()> {
    let serial = booted_emulator()?;
    run(
        "adb",
        &[
            "-s",
            &serial,
            "push",
            certificate,
            "/sdcard/Download/kestrl-ca.pem",
        ],
    )?;
    Ok(())
}
