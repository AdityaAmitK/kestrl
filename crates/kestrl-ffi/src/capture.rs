//! Capture for the app: browser, device and loopback port capture. Each completed exchange is
//! redacted, streamed to a sink, and optionally saved to history as actor `me`.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

use kestrl_core::actor::Actor;
use kestrl_core::capture::browser::{self, Browser, BrowserOptions};
use kestrl_core::capture::session::{self, SessionOptions};
use kestrl_core::capture::{self, CapturedExchange as CoreExchange, device, port};
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

/// Which booted device the proxy routes.
#[derive(Debug, Clone, Copy, uniffi::Enum)]
pub enum DeviceTarget {
    Simulator,
    Android,
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

/// A booted simulator or emulator, with the local proxy port it should use.
#[derive(Debug, uniffi::Record)]
pub struct DeviceCaptureOptions {
    pub workspace_root: Option<String>,
    pub target: DeviceTarget,
    /// Local proxy port on 127.0.0.1
    pub port: u16,
    pub hosts: Vec<String>,
    pub save_to_history: bool,
}

/// Plain-HTTP traffic on a loopback port. macOS BPF access is required.
#[derive(Debug, uniffi::Record)]
pub struct PortCaptureOptions {
    pub workspace_root: Option<String>,
    /// The loopback port the local server listens on
    pub port: u16,
    pub hosts: Vec<String>,
    pub save_to_history: bool,
}

/// Whether port capture can run here, and how to set it up. Setup is run by the user in Terminal.
#[derive(Debug, uniffi::Record)]
pub struct PortCaptureSetup {
    /// This process can open a BPF device
    pub accessible: bool,
    /// The one-time Kestrl BPF setup is installed
    pub installed: bool,
    /// The command the user runs from a Kestrl checkout. Kestrl never runs it.
    pub setup_command: String,
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

/// A running capture. `stop` ends it, then `on_end` is called once cleanup has finished:
/// the browser is closed, the device proxy is restored, or the port listener is released.
/// Dropping the handle also stops the capture, so no capture outlives its owner.
#[derive(uniffi::Object)]
pub struct CaptureHandle {
    stop: Arc<AtomicBool>,
}

#[uniffi::export]
impl CaptureHandle {
    pub fn stop(&self) {
        self.stop.store(true, Ordering::Release);
    }
}

impl Drop for CaptureHandle {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
    }
}

/// Launch a separate browser and stream its traffic to `sink` until it is stopped or closed.
#[uniffi::export]
pub fn start_browser_capture(
    options: BrowserCaptureOptions,
    sink: Arc<dyn CaptureSink>,
) -> Arc<CaptureHandle> {
    spawn_capture(Arc::clone(&sink), move |stop| {
        let recorder = Recorder::new(
            options.workspace_root,
            options.save_to_history,
            "browser",
            sink,
        )?;
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
            callback: recorder.callback(),
            profile_callback: Arc::new(|_| {}),
            stop,
        })?;
        Ok(())
    })
}

/// Route a booted simulator or emulator through a local proxy and stream its traffic. The
/// device's proxy is restored when capture stops, and on the next start after a crash.
#[uniffi::export]
pub fn start_device_capture(
    options: DeviceCaptureOptions,
    sink: Arc<dyn CaptureSink>,
) -> Arc<CaptureHandle> {
    let (source, simulator) = match options.target {
        DeviceTarget::Simulator => ("simulator", true),
        DeviceTarget::Android => ("android", false),
    };
    spawn_capture(Arc::clone(&sink), move |stop| {
        let recorder = Recorder::new(
            options.workspace_root,
            options.save_to_history,
            source,
            sink,
        )?;
        capture::ca::ensure()?;
        session::run(SessionOptions {
            port: options.port,
            hosts: options.hosts,
            callback: recorder.callback(),
            simulator,
            stop,
        })
    })
}

/// Read booted-simulator readiness without changing it. Returns the reason capture cannot start.
#[uniffi::export]
pub fn simulator_capture_problem() -> Option<String> {
    device::check_simulator()
        .err()
        .map(|error| error.to_string())
}

/// Read Android emulator readiness without changing it. Returns the reason capture cannot start.
#[uniffi::export]
pub fn android_capture_problem() -> Option<String> {
    device::check_android().err().map(|error| error.to_string())
}

/// Report BPF access and setup state. Nothing is installed or changed.
#[uniffi::export]
pub fn port_capture_setup() -> PortCaptureSetup {
    PortCaptureSetup {
        accessible: port::bpf_accessible(),
        installed: port::setup_installed(),
        setup_command: port::SETUP_COMMAND.to_owned(),
    }
}

/// Capture plain HTTP on a loopback port until it is stopped.
#[uniffi::export]
pub fn start_port_capture(
    options: PortCaptureOptions,
    sink: Arc<dyn CaptureSink>,
) -> Arc<CaptureHandle> {
    spawn_capture(Arc::clone(&sink), move |stop| {
        port::validate_port(options.port)?;
        let recorder = Recorder::new(
            options.workspace_root,
            options.save_to_history,
            "port",
            sink,
        )?;
        port::run_cancellable(options.port, options.hosts, recorder.callback(), stop)
    })
}

/// Run `run` on a background thread and report its end to `sink`. Errors from setup and capture
/// both arrive through `on_end`.
fn spawn_capture<F>(sink: Arc<dyn CaptureSink>, run: F) -> Arc<CaptureHandle>
where
    F: FnOnce(Arc<AtomicBool>) -> kestrl_core::Result<()> + Send + 'static,
{
    let stop = Arc::new(AtomicBool::new(false));
    let handle = Arc::new(CaptureHandle {
        stop: Arc::clone(&stop),
    });
    thread::spawn(move || {
        let result = run(stop);
        sink.on_end(result.err().map(|error| error.to_string()));
    });
    handle
}

/// Masks, saves and numbers exchanges for one capture. Shared by every capture kind.
struct Recorder {
    redactor: Redactor,
    history: Option<Mutex<History>>,
    workspace: Option<String>,
    env: Option<String>,
    retention: RetentionOptions,
    source: &'static str,
    seq: AtomicU64,
    sink: Arc<dyn CaptureSink>,
}

impl Recorder {
    fn new(
        workspace_root: Option<String>,
        save_to_history: bool,
        source: &'static str,
        sink: Arc<dyn CaptureSink>,
    ) -> kestrl_core::Result<Self> {
        let root = workspace_root.map(PathBuf::from);
        let config = root.as_deref().map(WorkspaceConfig::load).transpose()?;
        let redactor = match &root {
            Some(root) => workspace_redactor(root)?,
            None => Redactor::default(),
        };
        let history = if save_to_history {
            Some(Mutex::new(History::open_default()?))
        } else {
            None
        };
        Ok(Self {
            redactor,
            history,
            workspace: root.map(|root| root.to_string_lossy().into_owned()),
            env: config
                .as_ref()
                .and_then(|config| config.default_env.clone()),
            retention: config
                .as_ref()
                .map(|config| config.retention)
                .unwrap_or_default(),
            source,
            seq: AtomicU64::new(0),
            sink,
        })
    }

    fn callback(self) -> Arc<dyn Fn(CoreExchange) + Send + Sync> {
        let recorder = Arc::new(self);
        Arc::new(move |exchange: CoreExchange| recorder.deliver(exchange))
    }

    fn deliver(&self, exchange: CoreExchange) {
        let exchange = capture::redact(exchange, &self.redactor);
        let (event_id, history_error) = match &self.history {
            Some(history) => match self.record(history, &exchange) {
                Ok(id) => (Some(id), None),
                Err(error) => (None, Some(error.to_string())),
            },
            None => (None, None),
        };
        self.sink.on_exchange(CapturedExchange {
            seq: self.seq.fetch_add(1, Ordering::Relaxed) + 1,
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
    }

    fn record(
        &self,
        history: &Mutex<History>,
        exchange: &CoreExchange,
    ) -> kestrl_core::Result<String> {
        let mut history = history
            .lock()
            .map_err(|_| kestrl_core::Error::Capture("history lock poisoned".to_owned()))?;
        let session_id = history.current_session()?.map(|session| session.id);
        let capture = NewCapture {
            meta: EventMeta {
                source: self.source.to_owned(),
                actor: Actor::Me.as_str().to_owned(),
                session_id,
                workspace: self.workspace.clone(),
                env: self.env.clone(),
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
        history.record_capture(&capture, &self.retention)
    }
}
