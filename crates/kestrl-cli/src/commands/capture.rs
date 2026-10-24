//! Device capture commands.

use std::io::Write;
use std::sync::{Arc, Mutex};

use anyhow::Context as _;
use clap::{Args, Subcommand};
use kestrl_core::capture::proxy::ProxyOptions;
use kestrl_core::capture::{self, CapturedExchange};
use kestrl_core::history::{EventMeta, History, NewCapture};
use kestrl_core::workspace::{self, WorkspaceConfig};

use crate::commands::output;
use crate::context::Context;

/// Device capture subcommands.
#[derive(Debug, Args)]
pub struct CaptureArgs {
    #[command(subcommand)]
    command: DeviceCommand,
}

#[derive(Debug, Subcommand)]
enum DeviceCommand {
    /// Capture traffic from a booted iOS Simulator
    Sim(CaptureOptions),
    /// Capture traffic from a booted Android emulator
    Android(CaptureOptions),
}

#[derive(Debug, Args)]
struct CaptureOptions {
    /// Proxy listening port
    #[arg(long, default_value_t = 9090)]
    port: u16,
    /// Save each captured exchange to history
    #[arg(long)]
    save: bool,
    /// Limit captured hosts
    #[arg(long)]
    host: Vec<String>,
}

/// Start device capture and stream each completed exchange.
pub fn run(ctx: &Context, args: CaptureArgs) -> anyhow::Result<()> {
    let (source, options, simulator) = match args.command {
        DeviceCommand::Sim(options) => ("simulator", options, true),
        DeviceCommand::Android(options) => ("android", options, false),
    };
    let cwd = std::env::current_dir()?;
    let root = workspace::find_root(&cwd)?;
    let workspace_config = WorkspaceConfig::load(&root)?;
    let paths = capture::ca::ensure().context("create Kestrl root CA")?;
    capture::system_proxy::restore().context("restore stale system proxy state")?;
    let history = if options.save {
        Some(Mutex::new(History::open_default()?))
    } else {
        None
    };
    if simulator {
        capture::system_proxy::enable(options.port).context("enable macOS system proxy")?;
    } else {
        capture::device::start_android(options.port).context("configure Android emulator proxy")?;
    }
    let proxy_guard = if simulator {
        Some(capture::system_proxy::SystemProxyGuard::new())
    } else {
        None
    };
    if simulator {
        eprintln!("Install the Kestrl root certificate with `kestrl proxy trust --sim`.");
    } else {
        eprintln!("Install the Kestrl root certificate with `kestrl proxy trust --android`.");
    }
    let redactor = Arc::new(kestrl_core::secrets::workspace_redactor(&root)?);
    let actor = ctx.actor.as_str().to_owned();
    let json = ctx.is_json;
    let offset = ctx.utc_offset;
    let root_string = root.to_string_lossy().into_owned();
    let retention = workspace_config.retention;
    let callback = Arc::new(move |exchange: CapturedExchange| {
        let exchange = capture::redact(exchange, &redactor);
        if let Some(history) = &history {
            let result = (|| -> kestrl_core::Result<String> {
                let mut history = history
                    .lock()
                    .map_err(|_| kestrl_core::Error::Capture("history lock poisoned".to_owned()))?;
                let session_id = history.current_session()?.map(|session| session.id);
                let capture = NewCapture {
                    meta: EventMeta {
                        source: source.to_owned(),
                        actor: actor.clone(),
                        session_id,
                        workspace: Some(root_string.clone()),
                        env: workspace_config.default_env.clone(),
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
                history.record_capture(&capture, &retention)
            })();
            if let Err(error) = result {
                output::warn(&format!("capture event not recorded: {error}"));
            }
        }
        if json {
            if let Ok(line) =
                serde_json::to_string(&serde_json::json!({"ts": exchange.ts, "exchange": exchange}))
            {
                let _ = writeln!(std::io::stdout().lock(), "{line}");
            }
        } else {
            let path = exchange
                .url
                .split_once("://")
                .map_or(exchange.url.as_str(), |(_, rest)| rest);
            let status = if exchange.tunneled {
                "-".to_owned()
            } else {
                exchange
                    .status
                    .map_or_else(|| "-".to_owned(), |value| value.to_string())
            };
            let tunnel = if exchange.tunneled { "  tunneled" } else { "" };
            let _ = writeln!(
                std::io::stdout().lock(),
                "{}  {} {}  {}ms  {}{}",
                output::clock(exchange.ts, offset),
                exchange.method,
                status,
                exchange.duration_ms,
                path,
                tunnel
            );
        }
    });
    eprintln!(
        "Kestrl capture proxy listening on 127.0.0.1:{} (CA: {})",
        options.port,
        paths.certificate.display()
    );
    let result = capture::proxy::run(ProxyOptions {
        port: options.port,
        hosts: options.host,
        callback,
    });
    drop(proxy_guard);
    let cleanup = if simulator {
        capture::system_proxy::restore()
    } else {
        capture::device::stop_android()
    };
    result.context("run capture proxy")?;
    cleanup.context("restore device proxy settings")?;
    Ok(())
}
