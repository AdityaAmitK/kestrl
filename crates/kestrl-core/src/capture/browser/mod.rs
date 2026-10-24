//! Browser capture through Chrome DevTools Protocol.

mod cdp;
mod launch;
mod network;

use std::sync::Arc;

use crate::{Error, Result};

/// Browser choices supported by browser capture.
#[derive(Debug, Clone, Copy)]
pub enum Browser {
    /// Google Chrome.
    Chrome,
    /// Arc.
    Arc,
    /// Brave.
    Brave,
    /// Microsoft Edge.
    Edge,
    /// Chromium.
    Chromium,
}

/// Options for browser capture.
pub struct BrowserOptions {
    /// Browser to launch, or the first installed browser when omitted.
    pub browser: Option<Browser>,
    /// Initial URL.
    pub url: Option<String>,
    /// Optional host filters.
    pub hosts: Vec<String>,
    /// Receives captured exchanges.
    pub callback: Arc<dyn Fn(super::CapturedExchange) + Send + Sync>,
}

/// Launch a separate browser and capture its network traffic until it closes.
pub fn run(options: BrowserOptions) -> Result<std::path::PathBuf> {
    let mut process = launch::BrowserProcess::start(options.browser)?;
    let profile = process.profile.clone();
    let result = cdp::capture(
        &mut process,
        &options.hosts,
        options.url.as_deref(),
        &options.callback,
    );
    process.close();
    result.map(|()| profile)
}

impl From<tungstenite::Error> for Error {
    fn from(error: tungstenite::Error) -> Self {
        Self::Capture(format!("Chrome DevTools Protocol: {error}"))
    }
}
