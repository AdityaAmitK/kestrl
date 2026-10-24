//! Device capture commands.

use std::io::Write;
use std::path::Path;
use std::sync::{Arc, Mutex};

use anyhow::Context as _;
use clap::{Args, Subcommand};
use kestrl_core::capture::proxy::ProxyOptions;
use kestrl_core::capture::{self, CapturedExchange};
use kestrl_core::history::{History, NewCapture};
use kestrl_core::secrets::{self, MASK, Redactor};

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
    let paths = capture::ca::ensure().context("create Kestrl root CA")?;
    capture::system_proxy::restore().context("restore stale system proxy state")?;
    let history = if options.save {
        Some(Mutex::new(History::open_default()?))
    } else {
        None
    };
    if simulator {
        capture::system_proxy::enable(options.port).context("enable macOS system proxy")?;
        eprintln!("Install the Kestrl root certificate with `kestrl proxy trust --sim`.");
    } else {
        capture::device::start_android(options.port).context("configure Android emulator proxy")?;
        eprintln!("Install the Kestrl root certificate with `kestrl proxy trust --android`.");
    }
    let actor = ctx.actor.as_str().to_owned();
    let json = ctx.is_json;
    let redactor = Arc::new(capture_redactor());
    let callback = Arc::new(move |exchange: CapturedExchange| {
        let exchange = redact_exchange(exchange, &redactor);
        if let Some(history) = &history
            && let Ok(mut history) = history.lock()
        {
            let capture = NewCapture {
                source: source.to_owned(),
                method: exchange.method.clone(),
                url: exchange.url.clone(),
                req_headers: exchange.req_headers.clone(),
                req_body: exchange.req_body.clone(),
                status: exchange.status,
                res_headers: exchange.res_headers.clone(),
                res_body: exchange.res_body.clone(),
                duration_ms: exchange.duration_ms,
            };
            let _ = history.record_capture(&capture, &actor);
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
            let _ = writeln!(
                std::io::stdout().lock(),
                "{:02}:{:02}:{:02}  {} {}  {}ms  {}",
                (exchange.ts / 3_600_000) % 24,
                (exchange.ts / 60_000) % 60,
                (exchange.ts / 1_000) % 60,
                exchange.method,
                exchange.status.unwrap_or_default(),
                exchange.duration_ms,
                path
            );
        }
    });
    eprintln!(
        "Kestrl capture proxy listening on 0.0.0.0:{} (CA: {})",
        options.port,
        paths.certificate.display()
    );
    let result = capture::proxy::run(ProxyOptions {
        port: options.port,
        hosts: options.host,
        callback,
    });
    let cleanup = if simulator {
        capture::system_proxy::restore()
    } else {
        capture::device::stop_android()
    };
    result.context("run capture proxy")?;
    cleanup.context("restore device proxy settings")?;
    Ok(())
}

fn capture_redactor() -> Redactor {
    let mut redactor = Redactor::default();
    let current = std::env::current_dir().unwrap_or_default();
    let mut names = std::collections::BTreeSet::new();
    for directory in [current.join(".kestrl"), current.join("requests")] {
        collect_secret_names(&directory, &mut names);
    }
    for name in names {
        if let Ok(value) = secrets::get(&name) {
            redactor.track(&name, &value);
        }
    }
    redactor
}

fn collect_secret_names(path: &Path, names: &mut std::collections::BTreeSet<String>) {
    let Ok(entries) = std::fs::read_dir(path) else {
        return;
    };
    for entry in entries.flatten() {
        let entry_path = entry.path();
        if entry_path.is_dir() {
            collect_secret_names(&entry_path, names);
        } else if entry_path
            .extension()
            .is_some_and(|extension| extension == "toml")
            && let Ok(contents) = std::fs::read_to_string(entry_path)
        {
            names.extend(secrets::referenced_names(&contents));
        }
    }
}

fn redact_exchange(mut exchange: CapturedExchange, redactor: &Redactor) -> CapturedExchange {
    exchange.method = redactor.redact(&exchange.method);
    exchange.url = redactor.redact(&exchange.url);
    for (name, value) in &mut exchange.req_headers {
        if is_sensitive_header(name) {
            *value = MASK.to_owned();
        } else {
            *value = redactor.redact(value);
        }
        *name = redactor.redact(name);
    }
    for (name, value) in &mut exchange.res_headers {
        if is_sensitive_header(name) {
            *value = MASK.to_owned();
        } else {
            *value = redactor.redact(value);
        }
        *name = redactor.redact(name);
    }
    exchange.req_body = exchange.req_body.map(|body| redactor.redact(&body));
    exchange.res_body = exchange.res_body.map(|body| redactor.redact(&body));
    exchange
}

fn is_sensitive_header(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        "authorization" | "proxy-authorization" | "cookie" | "set-cookie"
    )
}
