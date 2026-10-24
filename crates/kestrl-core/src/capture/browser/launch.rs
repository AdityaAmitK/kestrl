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
    let choices: &[(Browser, &str)] = &[
        (Browser::Chrome, "Google Chrome"),
        (Browser::Arc, "Arc"),
        (Browser::Brave, "Brave Browser"),
        (Browser::Edge, "Microsoft Edge"),
        (Browser::Chromium, "Chromium"),
        (Browser::Helium, "Helium"),
    ];
    let selected = choices
        .iter()
        .filter(|(kind, _)| {
            requested.is_none_or(|wanted| {
                std::mem::discriminant(&wanted) == std::mem::discriminant(kind)
            })
        })
        .find_map(|(_, app)| {
            let home = std::env::var_os("HOME").map(PathBuf::from)?;
            [PathBuf::from("/Applications"), home.join("Applications")]
                .into_iter()
                .map(|directory| directory.join(format!("{app}.app")))
                .find_map(|bundle| {
                    bundle_executable(&bundle)
                        .ok()
                        .map(|binary| ((*app).to_owned(), binary))
                })
        });
    selected.ok_or_else(|| {
        let home = std::env::var("HOME").unwrap_or_else(|_| "~".to_owned());
        Error::Capture(format!("no supported Chromium browser found; searched /Applications and {home}/Applications; pass --browser-path <path>"))
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
