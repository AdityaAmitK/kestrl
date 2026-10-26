//! Find, launch, and stop an isolated Chromium browser.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use super::Browser;
use crate::{Error, Result};

/// Running browser and its temporary profile.
pub(super) struct BrowserProcess {
    child: Child,
    pub profile: PathBuf,
    pub port: u16,
}

impl BrowserProcess {
    /// Start a browser with a fresh, isolated profile and return its DevTools port.
    pub fn start(browser: Option<Browser>, browser_path: Option<&Path>) -> Result<Self> {
        // Best effort: a leftover profile must never block a new capture.
        let _ = remove_stale_profiles(&std::env::temp_dir());
        let (application, binary) = executable(browser, browser_path)?;
        let profile = std::env::temp_dir().join(format!("kestrl-browser-{}", ulid::Ulid::new()));
        fs::create_dir_all(&profile)?;
        let child = Command::new(&binary)
            .args([
                "--remote-debugging-port=0",
                "--no-first-run",
                "--no-default-browser-check",
            ])
            .arg(format!("--user-data-dir={}", profile.display()))
            .arg("about:blank")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|error| Error::Capture(format!("could not launch {application}: {error}")))?;
        let active_port = profile.join("DevToolsActivePort");
        let deadline = Instant::now() + Duration::from_secs(10);
        let port = loop {
            if let Ok(contents) = fs::read_to_string(&active_port)
                && let Some(port) = contents
                    .lines()
                    .next()
                    .and_then(|line| line.parse::<u16>().ok())
            {
                break port;
            }
            if Instant::now() >= deadline {
                let mut child = child;
                let _ = child.kill();
                let _ = child.wait();
                let _ = fs::remove_dir_all(&profile);
                return Err(Error::Capture(format!(
                    "{application} did not create DevToolsActivePort"
                )));
            }
            thread::sleep(Duration::from_millis(100));
        };
        Ok(Self {
            child,
            profile,
            port,
        })
    }

    /// Stop the browser and remove its profile.
    pub fn close(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = fs::remove_dir_all(&self.profile);
    }

    pub fn has_exited(&mut self) -> Result<bool> {
        Ok(self.child.try_wait()?.is_some())
    }
}

/// Remove abandoned Kestrl browser profiles that no running process is using.
fn remove_stale_profiles(temp_dir: &Path) -> Result<()> {
    let entries = fs::read_dir(temp_dir)?;
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if !name.starts_with("kestrl-browser-") || !entry.file_type()?.is_dir() {
            continue;
        }
        let argument = format!("--user-data-dir={}", path.display());
        let in_use = Command::new("pgrep")
            .args(["-f", "--", &argument])
            .status()
            .is_ok_and(|status| status.success());
        if !in_use {
            fs::remove_dir_all(path)?;
        }
    }
    Ok(())
}

fn executable(
    requested: Option<Browser>,
    browser_path: Option<&Path>,
) -> Result<(String, PathBuf)> {
    if let Some(path) = browser_path {
        if path.extension().is_some_and(|extension| extension == "app") {
            return bundle_executable(path).map(|binary| (path.display().to_string(), binary));
        }
        if path.is_file() {
            return Ok((path.display().to_string(), path.to_path_buf()));
        }
        return Err(Error::Capture(format!(
            "browser path does not exist: {}",
            path.display()
        )));
    }
    installed(requested).ok_or_else(|| {
        Error::Capture(format!(
            "no supported Chromium browser found; {SEARCHED}; pass --browser-path <path>"
        ))
    })
}

fn is_wanted(requested: Option<Browser>, kind: &Browser) -> bool {
    requested.is_none_or(|wanted| std::mem::discriminant(&wanted) == std::mem::discriminant(kind))
}

#[cfg(target_os = "macos")]
const SEARCHED: &str = "searched /Applications and ~/Applications";

/// The first installed application bundle, by preference.
#[cfg(target_os = "macos")]
fn installed(requested: Option<Browser>) -> Option<(String, PathBuf)> {
    let choices: &[(Browser, &str)] = &[
        (Browser::Chrome, "Google Chrome"),
        (Browser::Arc, "Arc"),
        (Browser::Brave, "Brave Browser"),
        (Browser::Edge, "Microsoft Edge"),
        (Browser::Chromium, "Chromium"),
        (Browser::Helium, "Helium"),
    ];
    let home = std::env::var_os("HOME").map(PathBuf::from)?;
    choices
        .iter()
        .filter(|(kind, _)| is_wanted(requested, kind))
        .find_map(|(_, app)| {
            [PathBuf::from("/Applications"), home.join("Applications")]
                .into_iter()
                .map(|directory| directory.join(format!("{app}.app")))
                .find_map(|bundle| {
                    bundle_executable(&bundle)
                        .ok()
                        .map(|binary| ((*app).to_owned(), binary))
                })
        })
}

#[cfg(not(target_os = "macos"))]
const SEARCHED: &str = "searched PATH";

/// The first browser command on `PATH`, by preference. Arc and Helium have no Linux builds.
#[cfg(not(target_os = "macos"))]
fn installed(requested: Option<Browser>) -> Option<(String, PathBuf)> {
    let choices: &[(Browser, &str)] = &[
        (Browser::Chrome, "google-chrome"),
        (Browser::Chrome, "google-chrome-stable"),
        (Browser::Brave, "brave-browser"),
        (Browser::Edge, "microsoft-edge"),
        (Browser::Chromium, "chromium"),
        (Browser::Chromium, "chromium-browser"),
    ];
    let path = std::env::var_os("PATH")?;
    choices
        .iter()
        .filter(|(kind, _)| is_wanted(requested, kind))
        .find_map(|(_, command)| {
            std::env::split_paths(&path)
                .map(|directory| directory.join(command))
                .find(|binary| binary.is_file())
                .map(|binary| ((*command).to_owned(), binary))
        })
}

fn bundle_executable(bundle: &Path) -> Result<PathBuf> {
    let plist = bundle.join("Contents/Info.plist");
    let output = Command::new("/usr/libexec/PlistBuddy")
        .args(["-c", "Print :CFBundleExecutable"])
        .arg(&plist)
        .output()
        .map_err(|error| Error::Capture(format!("could not read {}: {error}", plist.display())))?;
    if !output.status.success() {
        return Err(Error::Capture(format!(
            "could not read CFBundleExecutable from {}",
            plist.display()
        )));
    }
    let name = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    let binary = bundle.join("Contents/MacOS").join(name);
    if !binary.is_file() {
        return Err(Error::Capture(format!(
            "browser executable not found: {}",
            binary.display()
        )));
    }
    Ok(binary)
}
