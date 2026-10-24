//! Ordered runtime and system proxy lifecycle for device capture.

use std::net::{Ipv4Addr, SocketAddr};
use std::sync::Arc;

use crate::{Error, Result};

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
}

/// Start device capture and restore device proxy settings on exit.
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
    let address = listener.local_addr()?;
    let (shutdown_sender, shutdown_receiver) = tokio::sync::oneshot::channel();
    let proxy_options = super::proxy::ProxyOptions {
        port: options.port,
        hosts: options.hosts,
        callback: options.callback,
    };
    let server = super::proxy::run(proxy_options, listener, async move {
        let _ = shutdown_receiver.await;
    })?;
    let mut interrupt =
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt())
            .map_err(|error| Error::Capture(format!("install SIGINT handler: {error}")))?;
    let mut terminate =
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .map_err(|error| Error::Capture(format!("install SIGTERM handler: {error}")))?;
    let guard = super::system_proxy::SystemProxyGuard::new();
    if options.simulator {
        if let Err(error) = super::system_proxy::enable(options.port) {
            drop(guard);
            return Err(error);
        }
        eprintln!(
            "Capturing on {address} — system proxy enabled for active network services. Ctrl-C to stop."
        );
    } else {
        if let Err(error) = super::device::start_android(options.port) {
            let _ = shutdown_sender.send(());
            let _ = server.await;
            return Err(error);
        }
        eprintln!("Capturing Android emulator traffic on {address}. Ctrl-C to stop.");
    }
    tokio::select! {
        _ = interrupt.recv() => (),
        _ = terminate.recv() => (),
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
