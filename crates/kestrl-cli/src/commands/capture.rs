//! Device capture commands.

use std::io::Write;
use std::sync::{Arc, Mutex};

use anyhow::Context as _;
use clap::{Args, Subcommand};
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
    /// Capture traffic from a separate browser window
    Browser(BrowserOptions),
    /// Capture plain-HTTP traffic on a loopback port
    Port(PortOptions),
}

#[derive(Debug, Args)]
struct PortOptions {
    /// Existing local server port, or a setup action
    port: Option<u16>,
    /// Save each captured exchange to history
    #[arg(long)]
    save: bool,
    /// Limit captured hosts
    #[arg(long)]
    host: Vec<String>,
    #[command(subcommand)]
    action: Option<PortAction>,
}

#[derive(Debug, Subcommand)]
enum PortAction {
    /// Explain and check the one-time macOS BPF setup requirement
    Setup,
    /// Show whether port capture can run
    Status,
    /// Remove an installed port-capture helper
    Uninstall,
}

#[derive(Debug, Args)]
struct BrowserOptions {
    /// Chromium browser to launch
    #[arg(long, value_parser = ["chrome", "arc", "brave", "edge", "chromium", "helium"])]
    browser: Option<String>,
    /// Browser bundle or executable path
    #[arg(long)]
    browser_path: Option<std::path::PathBuf>,
    /// Initial URL
    #[arg(long)]
    url: Option<String>,
    /// Save each captured exchange to history
    #[arg(long)]
    save: bool,
    /// Limit captured hosts
    #[arg(long)]
    host: Vec<String>,
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
    let command = match args.command {
        DeviceCommand::Browser(options) => return run_browser(ctx, options),
        DeviceCommand::Port(options) => return run_port(ctx, options),
        command => command,
    };
    let (source, options, simulator) = match command {
        DeviceCommand::Sim(options) => ("simulator", options, true),
        DeviceCommand::Android(options) => ("android", options, false),
        DeviceCommand::Browser(_) => {
            return Err(anyhow::anyhow!("browser command dispatch failed"));
        }
        DeviceCommand::Port(_) => return Err(anyhow::anyhow!("port command dispatch failed")),
    };
    let cwd = std::env::current_dir()?;
    let root = workspace::find_root(&cwd).ok();
    let workspace_config = root
        .as_ref()
        .map(|path| WorkspaceConfig::load(path))
        .transpose()?;
    capture::ca::ensure().context("create Kestrl root CA")?;
    capture::system_proxy::restore().context("restore stale system proxy state")?;
    let history = if options.save {
        Some(Mutex::new(History::open_default()?))
    } else {
        None
    };
    let names = root
        .as_ref()
        .map(|path| kestrl_core::secrets::referenced_in_workspace(path))
        .transpose()?
        .unwrap_or_default();
    eprintln!(
        "Reading {} workspace secrets from the Keychain for masking…",
        names.len()
    );
    let redactor = Arc::new(match &root {
        Some(root) => kestrl_core::secrets::workspace_redactor(root)?,
        None => kestrl_core::secrets::Redactor::default(),
    });
    let actor = ctx.actor.as_str().to_owned();
    let json = ctx.force_json;
    let offset = ctx.utc_offset;
    let root_string = root
        .as_ref()
        .map(|path| path.to_string_lossy().into_owned());
    let retention = workspace_config.as_ref().map(|config| config.retention);
    if simulator {
        eprintln!("Install the Kestrl root certificate with `kestrl proxy trust --sim`.");
    } else {
        eprintln!("Install the Kestrl root certificate with `kestrl proxy trust --android`.");
    }
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
                        workspace: root_string.clone(),
                        env: workspace_config
                            .as_ref()
                            .and_then(|config| config.default_env.clone()),
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
                if let Some(retention) = &retention {
                    history.record_capture(&capture, retention)
                } else {
                    history.record_capture(&capture, &Default::default())
                }
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
            output::print_capture_line(&exchange, offset);
        }
    });
    capture::session::run(capture::session::SessionOptions {
        port: options.port,
        hosts: options.host,
        callback,
        simulator,
    })?;
    Ok(())
}

fn run_port(ctx: &Context, options: PortOptions) -> anyhow::Result<()> {
    match options.action {
        Some(PortAction::Setup) => anyhow::bail!("{}", kestrl_core::capture::port::setup_status()),
        Some(PortAction::Status) => {
            writeln!(
                std::io::stdout().lock(),
                "{}",
                kestrl_core::capture::port::status()
            )?;
            Ok(())
        }
        Some(PortAction::Uninstall) => {
            writeln!(
                std::io::stdout().lock(),
                "{}",
                kestrl_core::capture::port::uninstall()?
            )?;
            Ok(())
        }
        None => {
            let port = options
                .port
                .ok_or_else(|| anyhow::anyhow!("provide a port: `kestrl capture port <port>`"))?;
            let cwd = std::env::current_dir()?;
            let root = workspace::find_root(&cwd).ok();
            let config = root
                .as_ref()
                .map(|root| WorkspaceConfig::load(root))
                .transpose()?;
            let history = if options.save {
                Some(Mutex::new(History::open_default()?))
            } else {
                None
            };
            let redactor = Arc::new(match &root {
                Some(root) => kestrl_core::secrets::workspace_redactor(root)?,
                None => kestrl_core::secrets::Redactor::default(),
            });
            let actor = ctx.actor.as_str().to_owned();
            let json = ctx.force_json;
            let offset = ctx.utc_offset;
            let root_string = root
                .as_ref()
                .map(|root| root.to_string_lossy().into_owned());
            let retention = config.as_ref().map(|config| config.retention);
            let callback = Arc::new(move |exchange: CapturedExchange| {
                let exchange = capture::redact(exchange, &redactor);
                if let Some(history) = &history {
                    let result = (|| -> kestrl_core::Result<String> {
                        let mut history = history.lock().map_err(|_| {
                            kestrl_core::Error::Capture("history lock poisoned".to_owned())
                        })?;
                        let capture = NewCapture {
                            meta: EventMeta {
                                source: "port".to_owned(),
                                actor: actor.clone(),
                                session_id: history.current_session()?.map(|session| session.id),
                                workspace: root_string.clone(),
                                env: config
                                    .as_ref()
                                    .and_then(|config| config.default_env.clone()),
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
                        if let Some(retention) = &retention {
                            history.record_capture(&capture, retention)
                        } else {
                            history.record_capture(&capture, &Default::default())
                        }
                    })();
                    if let Err(error) = result {
                        output::warn(&format!("capture event not recorded: {error}"));
                    }
                }
                if json {
                    if let Ok(line) = serde_json::to_string(
                        &serde_json::json!({"ts": exchange.ts, "exchange": exchange}),
                    ) {
                        let _ = writeln!(std::io::stdout().lock(), "{line}");
                    }
                } else {
                    output::print_capture_line(&exchange, offset);
                }
            });
            eprintln!("Capturing plain HTTP on loopback port {port}; press Ctrl-C to stop.");
            capture::port::run(port, options.host, callback)?;
            Ok(())
        }
    }
}

fn run_browser(ctx: &Context, options: BrowserOptions) -> anyhow::Result<()> {
    let cwd = std::env::current_dir()?;
    let root = workspace::find_root(&cwd).ok();
    let config = root
        .as_ref()
        .map(|root| WorkspaceConfig::load(root))
        .transpose()?;
    let history = if options.save {
        Some(Mutex::new(History::open_default()?))
    } else {
        None
    };
    let redactor = Arc::new(match &root {
        Some(root) => kestrl_core::secrets::workspace_redactor(root)?,
        None => kestrl_core::secrets::Redactor::default(),
    });
    let actor = ctx.actor.as_str().to_owned();
    let json = ctx.force_json;
    let offset = ctx.utc_offset;
    let root_string = root
        .as_ref()
        .map(|path| path.to_string_lossy().into_owned());
    let retention = config.as_ref().map(|config| config.retention);
    let callback = Arc::new(move |exchange: CapturedExchange| {
        let exchange = capture::redact(exchange, &redactor);
        if let Some(history) = &history {
            let result = (|| -> kestrl_core::Result<String> {
                let mut history = history
                    .lock()
                    .map_err(|_| kestrl_core::Error::Capture("history lock poisoned".to_owned()))?;
                let session_id = history.current_session()?.map(|session| session.id);
                let saved = NewCapture {
                    meta: EventMeta {
                        source: "browser".to_owned(),
                        actor: actor.clone(),
                        session_id,
                        workspace: root_string.clone(),
                        env: config
                            .as_ref()
                            .and_then(|config| config.default_env.clone()),
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
                if let Some(retention) = &retention {
                    history.record_capture(&saved, retention)
                } else {
                    history.record_capture(&saved, &Default::default())
                }
            })();
            if let Err(error) = result {
                output::warn(&format!("capture event not recorded: {error}"));
            }
        }
        if json {
            if let Ok(line) =
                serde_json::to_string(&serde_json::json!({"ts":exchange.ts,"exchange":exchange}))
            {
                let _ = writeln!(std::io::stdout().lock(), "{line}");
            }
        } else {
            output::print_capture_line(&exchange, offset);
        }
    });
    let browser = options.browser.as_deref().map(|name| match name {
        "chrome" => kestrl_core::capture::browser::Browser::Chrome,
        "arc" => kestrl_core::capture::browser::Browser::Arc,
        "brave" => kestrl_core::capture::browser::Browser::Brave,
        "edge" => kestrl_core::capture::browser::Browser::Edge,
        "chromium" => kestrl_core::capture::browser::Browser::Chromium,
        _ => kestrl_core::capture::browser::Browser::Helium,
    });
    let profile =
        kestrl_core::capture::browser::run(kestrl_core::capture::browser::BrowserOptions {
            browser,
            browser_path: options.browser_path,
            url: options.url,
            hosts: options.host,
            callback,
            profile_callback: Arc::new(|profile| {
                eprintln!("Browser capture isolated profile: {}", profile.display());
            }),
        })?;
    eprintln!(
        "Browser capture used isolated profile: {}",
        profile.display()
    );
    Ok(())
}
