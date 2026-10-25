//! Ordered runtime and system proxy lifecycle for device capture.

use std::net::{Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use crate::{Error, Result};

/// How often a running session checks its stop flag.
const STOP_POLL: Duration = Duration::from_millis(100);

/// Device capture session settings.
pub struct SessionOptions {
    /// Proxy listening port.
    pub port: u16,
    /// Optional host glob filters.
    pub hosts: Vec<String>,
    /// Whether capture is for an iOS Simulator.
    pub simulator: bool,
    /// Receives completed HTTP exchanges.
    pub callback: Arc<dyn Fn(super::CapturedExchange) + Send + Sync>,
    /// Set to true to end the session. The device proxy is restored before `run` returns.
    pub stop: Arc<AtomicBool>,
}

/// Run device capture until `stop` is set or the proxy fails, then restore device proxy settings.
/// The caller owns signal handling: the CLI sets `stop` from Ctrl-C, the app sets it from Stop.
pub fn run(options: SessionOptions) -> Result<()> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(|error| Error::Capture(format!("prepare capture runtime: {error}")))?;
    runtime.block_on(run_session(options))
}

async fn run_session(options: SessionOptions) -> Result<()> {
    super::system_proxy::restore()?;
    let listener =
        tokio::net::TcpListener::bind(SocketAddr::from((Ipv4Addr::LOCALHOST, options.port)))
            .await?;
    let (shutdown_sender, shutdown_receiver) = tokio::sync::oneshot::channel();
    let proxy_options = super::proxy::ProxyOptions {
        port: options.port,
        hosts: options.hosts,
        callback: options.callback,
    };
    let server = super::proxy::run(proxy_options, listener, async move {
        let _ = shutdown_receiver.await;
    })?;
    let guard = super::system_proxy::SystemProxyGuard::new();
    if options.simulator {
        if let Err(error) = super::system_proxy::enable(options.port) {
            drop(guard);
            let _ = shutdown_sender.send(());
            let _ = server.await;
            return Err(error);
        }
    } else if let Err(error) = super::device::start_android(options.port) {
        let _ = shutdown_sender.send(());
        let _ = server.await;
        return Err(error);
    }
    while !options.stop.load(Ordering::Acquire) && !server.is_finished() {
        tokio::time::sleep(STOP_POLL).await;
    }
    let cleanup = if options.simulator {
        super::system_proxy::restore()
    } else {
        super::device::stop_android()
    };
    drop(guard);
    let _ = shutdown_sender.send(());
    server
        .await
        .map_err(|error| Error::Capture(format!("capture proxy task failed: {error}")))??;
    cleanup
}
