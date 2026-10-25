//! Browser capture for the app. Each completed exchange is redacted, streamed to a sink, and
//! optionally saved to history as actor `me`, source `browser`.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

use kestrl_core::actor::Actor;
use kestrl_core::capture::browser::{self, Browser, BrowserOptions};
use kestrl_core::capture::{self, CapturedExchange as CoreExchange};
use kestrl_core::history::{EventMeta, History, NewCapture, RetentionOptions};
use kestrl_core::secrets::{Redactor, workspace_redactor};
use kestrl_core::workspace::WorkspaceConfig;

use crate::types::KeyValue;

/// The browsers Kestrl can launch for capture.
#[derive(Debug, Clone, Copy, uniffi::Enum)]
pub enum CaptureBrowser {
    Chrome,
    Arc,
    Brave,
    Edge,
    Chromium,
    Helium,
}

/// What to capture. Without a browser, the first installed one is used.
#[derive(Debug, uniffi::Record)]
pub struct BrowserCaptureOptions {
    /// The open workspace, used for secret masking and history settings
    pub workspace_root: Option<String>,
    pub browser: Option<CaptureBrowser>,
    /// The page the browser opens first
    pub start_url: Option<String>,
    /// Host glob filters such as `*.example.com`. Empty captures every host.
    pub hosts: Vec<String>,
    /// Record each exchange in history as it completes
    pub save_to_history: bool,
}

/// One completed request and response, with secrets masked.
#[derive(Debug, Clone, uniffi::Record)]
pub struct CapturedExchange {
    /// Counts up from 1 for each capture, so the app can identify rows
    pub seq: u64,
    pub ts_ms: i64,
    pub method: String,
    pub url: String,
    pub status: Option<u16>,
    pub duration_ms: u64,
    pub request_headers: Vec<KeyValue>,
    pub request_body: Option<String>,
    pub response_headers: Vec<KeyValue>,
    pub response_body: Option<String>,
    /// History event id, when the exchange was saved
    pub event_id: Option<String>,
    /// Why saving failed, when it did; the capture continues
    pub history_error: Option<String>,
}

/// Receives exchanges from the capture thread. Calls come from a background thread.
#[uniffi::export(with_foreign)]
pub trait CaptureSink: Send + Sync {
    fn on_exchange(&self, exchange: CapturedExchange);
    /// Called once when capture stops. `error` is set when it failed to start or stopped unexpectedly.
    fn on_end(&self, error: Option<String>);
}

/// A running browser capture. `stop` ends it, closes the browser and removes its profile.
#[derive(uniffi::Object)]
pub struct BrowserCapture {
    stop: Arc<AtomicBool>,
}

#[uniffi::export]
impl BrowserCapture {
    pub fn stop(&self) {
        self.stop.store(true, Ordering::Release);
    }
}

/// Launch a separate browser and stream its traffic to `sink` until it is stopped or closed.
#[uniffi::export]
pub fn start_browser_capture(
    options: BrowserCaptureOptions,
    sink: Arc<dyn CaptureSink>,
) -> Arc<BrowserCapture> {
    let stop = Arc::new(AtomicBool::new(false));
    let handle = Arc::new(BrowserCapture {
        stop: Arc::clone(&stop),
    });
    thread::spawn(move || {
        let result = run_capture(options, Arc::clone(&stop), Arc::clone(&sink));
        sink.on_end(result.err().map(|error| error.to_string()));
    });
    handle
}

fn run_capture(
    options: BrowserCaptureOptions,
    stop: Arc<AtomicBool>,
    sink: Arc<dyn CaptureSink>,
) -> kestrl_core::Result<()> {
    let root = options.workspace_root.map(PathBuf::from);
    let config = root.as_deref().map(WorkspaceConfig::load).transpose()?;
    let redactor = match &root {
        Some(root) => workspace_redactor(root)?,
        None => Redactor::default(),
    };
    let retention = config
        .as_ref()
        .map(|config| config.retention)
        .unwrap_or_default();
    let env = config
        .as_ref()
        .and_then(|config| config.default_env.clone());
    let workspace = root.map(|root| root.to_string_lossy().into_owned());
    let history = if options.save_to_history {
        Some(Mutex::new(History::open_default()?))
    } else {
        None
    };
    let seq = AtomicU64::new(0);
    let callback = Arc::new(move |exchange: CoreExchange| {
        let exchange = capture::redact(exchange, &redactor);
        let (event_id, history_error) = match &history {
            Some(history) => match record(history, &exchange, &workspace, &env, &retention) {
                Ok(id) => (Some(id), None),
                Err(error) => (None, Some(error.to_string())),
            },
            None => (None, None),
        };
        sink.on_exchange(CapturedExchange {
            seq: seq.fetch_add(1, Ordering::Relaxed) + 1,
            ts_ms: exchange.ts,
            method: exchange.method,
            url: exchange.url,
            status: exchange.status,
            duration_ms: exchange.duration_ms,
            request_headers: KeyValue::from_pairs(exchange.req_headers),
            request_body: exchange.req_body,
            response_headers: KeyValue::from_pairs(exchange.res_headers),
            response_body: exchange.res_body,
            event_id,
            history_error,
        });
    });
    let browser = options.browser.map(|browser| match browser {
        CaptureBrowser::Chrome => Browser::Chrome,
        CaptureBrowser::Arc => Browser::Arc,
        CaptureBrowser::Brave => Browser::Brave,
        CaptureBrowser::Edge => Browser::Edge,
        CaptureBrowser::Chromium => Browser::Chromium,
        CaptureBrowser::Helium => Browser::Helium,
    });
    browser::run(BrowserOptions {
        browser,
        browser_path: None,
        url: options.start_url,
        hosts: options.hosts,
        callback,
        profile_callback: Arc::new(|_| {}),
        stop,
    })?;
    Ok(())
}

fn record(
    history: &Mutex<History>,
    exchange: &CoreExchange,
    workspace: &Option<String>,
    env: &Option<String>,
    retention: &RetentionOptions,
) -> kestrl_core::Result<String> {
    let mut history = history
        .lock()
        .map_err(|_| kestrl_core::Error::Capture("history lock poisoned".to_owned()))?;
    let session_id = history.current_session()?.map(|session| session.id);
    let capture = NewCapture {
        meta: EventMeta {
            source: "browser".to_owned(),
            actor: Actor::Me.as_str().to_owned(),
            session_id,
            workspace: workspace.clone(),
            env: env.clone(),
            request_ref: None,
        },
        method: exchange.method.clone(),
        url: exchange.url.clone(),
        req_headers: exchange.req_headers.clone(),
        req_body: exchange.req_body.clone(),
        status: exchange.status,
        res_headers: exchange.res_headers.clone(),
        res_body: exchange.res_body.clone(),
        duration_ms: exchange.duration_ms,
    };
    history.record_capture(&capture, retention)
}
