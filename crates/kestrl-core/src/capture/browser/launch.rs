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
    pub fn start(browser: Option<Browser>) -> Result<Self> {
        let (application, binary) = executable(browser)?;
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

fn executable(requested: Option<Browser>) -> Result<(String, PathBuf)> {
    let choices: &[(Browser, &str, &str)] = &[
        (Browser::Chrome, "Google Chrome", "Google Chrome"),
        (Browser::Arc, "Arc", "Arc"),
        (Browser::Brave, "Brave Browser", "Brave Browser"),
        (Browser::Edge, "Microsoft Edge", "Microsoft Edge"),
        (Browser::Chromium, "Chromium", "Chromium"),
    ];
    let selected = choices
        .iter()
        .filter(|(kind, _, _)| {
            requested.is_none_or(|wanted| {
                std::mem::discriminant(&wanted) == std::mem::discriminant(kind)
            })
        })
        .find_map(|(_, app, binary)| {
            let path =
                Path::new("/Applications").join(format!("{app}.app/Contents/MacOS/{binary}"));
            path.is_file().then_some(((*app).to_owned(), path))
        });
    selected.ok_or_else(|| {
        Error::Capture("no supported Chromium browser found in /Applications".to_owned())
    })
}
