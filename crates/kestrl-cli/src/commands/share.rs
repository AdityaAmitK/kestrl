//! `kestrl share`: expose one local port through a Cloudflare tunnel for a bounded time.

use std::io::Write;
use std::process::{Command, Stdio};

use clap::{ArgGroup, Args, Subcommand};
use kestrl_core::history::now_ms;
use kestrl_core::share::{self, Access, Share, ShareOptions};

use crate::commands::output::{age, print_json};
use crate::context::Context;

#[derive(Debug, Args)]
pub struct ShareArgs {
    #[command(subcommand)]
    pub action: ShareAction,
}

#[derive(Debug, Subcommand)]
pub enum ShareAction {
    /// Share a local port and print its URL; the share closes itself when its lifetime ends
    Open(OpenArgs),
    /// List open shares
    Ls,
    /// Show one share
    Status { id: String },
    /// Close a share, or every share with --all
    #[command(group(ArgGroup::new("target").required(true).args(["id", "all"])))]
    Close {
        id: Option<String>,
        #[arg(long)]
        all: bool,
    },
    /// Run a share's tunnel until it closes (started by `open`)
    #[command(hide = true)]
    Serve { id: String },
}

/// Arguments for `kestrl share open`.
#[derive(Debug, Args)]
#[command(group(ArgGroup::new("access").required(true).args(["public", "protected"])))]
pub struct OpenArgs {
    /// Local port of the web application to share
    pub port: u16,
    /// Anyone with the URL can reach it
    #[arg(long)]
    pub public: bool,
    /// Only people Cloudflare Access admits; needs --hostname covered by an Access application
    #[arg(long)]
    pub protected: bool,
    /// Named hostname in your Cloudflare zone instead of a temporary trycloudflare.com URL
    #[arg(long)]
    pub hostname: Option<String>,
    #[arg(long, value_name = "NAME", help = format!("Named tunnel that serves --hostname [default: {}]", share::DEFAULT_TUNNEL))]
    pub tunnel: Option<String>,
    /// The local server speaks HTTPS (its certificate is not verified)
    #[arg(long)]
    pub https: bool,
    /// Lifetime such as 30m or 2h, at most 24h
    #[arg(long, default_value = "1h")]
    pub ttl: String,
}

pub fn run(ctx: &Context, args: ShareArgs) -> anyhow::Result<()> {
    match args.action {
        ShareAction::Open(arguments) => open(ctx, arguments),
        ShareAction::Ls => {
            let shares = share::list()?;
            if ctx.is_json {
                return print_json(&serde_json::to_value(&shares)?);
            }
            let mut stdout = std::io::stdout().lock();
            writeln!(
                stdout,
                "{:<9} {:>5} {:<9} {:<9} {:>8} URL",
                "SHARE", "PORT", "ACCESS", "STATE", "EXPIRES"
            )?;
            for share in &shares {
                writeln!(stdout, "{}", row(share))?;
            }
            Ok(())
        }
        ShareAction::Status { id } => show(ctx, &share::get(&id)?),
        ShareAction::Close { id, all } => {
            let ids: Vec<String> = match id {
                Some(id) => vec![id],
                None if all => share::list()?.into_iter().map(|share| share.id).collect(),
                None => Vec::new(),
            };
            for id in &ids {
                share::close(id)?;
                eprintln!("Closed share {id}");
            }
            Ok(())
        }
        ShareAction::Serve { id } => Ok(share::serve(&id)?),
    }
}

fn open(ctx: &Context, args: OpenArgs) -> anyhow::Result<()> {
    let options = ShareOptions {
        port: args.port,
        is_https: args.https,
        access: if args.protected {
            Access::Protected
        } else {
            Access::Public
        },
        hostname: args.hostname,
        tunnel: args.tunnel,
        ttl: kestrl_core::duration::parse_duration(&args.ttl)?,
    };
    let created = share::create(&options)?;
    // The owner runs detached in its own process group, so the share outlives this command.
    let spawned = {
        use std::os::unix::process::CommandExt;
        Command::new(std::env::current_exe()?)
            .args(["share", "serve", &created.id])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .process_group(0)
            .spawn()
    };
    if let Err(error) = spawned {
        let _ = share::close(&created.id);
        anyhow::bail!("could not start the share process: {error}");
    }
    let ready = share::wait_ready(&created.id)?;
    show(ctx, &ready)
}

fn show(ctx: &Context, share: &Share) -> anyhow::Result<()> {
    if ctx.is_json {
        return print_json(&serde_json::to_value(share)?);
    }
    let access = match share.access {
        Access::Public => "public: anyone with the URL can reach it",
        Access::Protected => "protected by Cloudflare Access",
    };
    writeln!(
        std::io::stdout(),
        "{}\n  share {} → localhost:{} ({access})\n  closes in {} · kestrl share close {}",
        share.url.as_deref().unwrap_or("(starting)"),
        share.id,
        share.port,
        age(share.expires_at - now_ms()),
        share.id
    )?;
    Ok(())
}

fn row(share: &Share) -> String {
    let access = match share.access {
        Access::Public => "public",
        Access::Protected => "protected",
    };
    let state = match share.state {
        share::ShareState::Starting => "starting",
        share::ShareState::Ready => "ready",
        share::ShareState::Failed => "failed",
    };
    format!(
        "{:<9} {:>5} {:<9} {:<9} {:>8} {}",
        share.id,
        share.port,
        access,
        state,
        age(share.expires_at - now_ms()),
        share.url.as_deref().unwrap_or("-")
    )
}
