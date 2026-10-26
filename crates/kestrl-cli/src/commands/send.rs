//! `kestrl send`: send one request (file or METHOD URL), recording it in history.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{Context as _, bail};
use clap::Args;
use kestrl_core::history::{EventMeta, History, RetentionOptions};
use kestrl_core::http;
use kestrl_core::request::Body;
use kestrl_core::send::{self as core_send, SendOptions, Target};
use kestrl_core::workspace;

use crate::commands::output;
use crate::context::Context;

#[derive(Debug, Args)]
pub struct SendArgs {
    /// Request file path, or an HTTP method followed by a URL
    #[arg(required = true, num_args = 1..=2)]
    pub target: Vec<String>,
    #[command(flatten)]
    pub resolve: crate::commands::ResolveArgs,
    #[command(flatten)]
    pub content: ContentArgs,
    /// Tag to add to the recorded event
    #[arg(long)]
    pub tag: Option<String>,
}

/// Headers and body given on the command line, applied on top of the request.
#[derive(Debug, Args, Default)]
pub struct ContentArgs {
    /// Header `Name: value`; repeatable. Overrides the request's and its profiles' headers.
    /// Values may use {{var}}, {{secret:name}} and {{env:NAME}}
    #[arg(short = 'H', long = "header", value_name = "HEADER")]
    pub headers: Vec<String>,
    /// Request body as text; `@path` reads a file and `@-` reads stdin
    #[arg(short = 'd', long, value_name = "BODY", conflicts_with = "json_body")]
    pub data: Option<String>,
    /// JSON request body (`@path` or `@-` to read it); sets Content-Type and Accept to
    /// application/json unless -H sets them
    #[arg(long, value_name = "JSON")]
    pub json_body: Option<String>,
}

impl ContentArgs {
    /// The header overrides and body these arguments describe.
    pub fn resolve(&self) -> anyhow::Result<(BTreeMap<String, String>, Option<Body>)> {
        let mut headers = BTreeMap::new();
        for header in &self.headers {
            let Some((name, value)) = header.split_once(':') else {
                bail!("header {header:?} must be `Name: value`");
            };
            let name = name.trim();
            if name.is_empty() {
                bail!("header {header:?} has no name");
            }
            // The last -H for a name wins, whatever its letter case.
            headers.retain(|existing: &String, _| !existing.eq_ignore_ascii_case(name));
            headers.insert(name.to_owned(), value.trim().to_owned());
        }
        let body = match (&self.data, &self.json_body) {
            (Some(data), _) => Some(Body::Text {
                content: read_body(data)?,
            }),
            (None, Some(json)) => {
                for name in ["Content-Type", "Accept"] {
                    if !headers.keys().any(|key| key.eq_ignore_ascii_case(name)) {
                        headers.insert(name.to_owned(), "application/json".to_owned());
                    }
                }
                Some(Body::Json {
                    content: read_body(json)?,
                })
            }
            (None, None) => None,
        };
        Ok((headers, body))
    }
}

/// A body argument: literal text, `@path` for a file, or `@-` for stdin.
fn read_body(value: &str) -> anyhow::Result<String> {
    match value.strip_prefix('@') {
        Some("-") => {
            let mut text = String::new();
            std::io::Read::read_to_string(&mut std::io::stdin(), &mut text)
                .context("cannot read the body from stdin")?;
            Ok(text)
        }
        Some(path) => std::fs::read_to_string(path)
            .with_context(|| format!("cannot read the body from {path}")),
        None => Ok(value.to_owned()),
    }
}

/// An exchange and the history event recorded for it.
pub struct Sent {
    pub exchange: http::Exchange,
    /// `None` when recording failed; a warning has already been printed.
    pub event_id: Option<String>,
}

pub fn send_one(
    ctx: &Context,
    target: &[String],
    resolve: &crate::commands::ResolveArgs,
    content: &ContentArgs,
) -> anyhow::Result<Sent> {
    let client = http::client()?;
    send_one_with_client(ctx, target, resolve, content, &client, None)
}

pub fn send_one_with_client(
    ctx: &Context,
    target: &[String],
    resolve: &crate::commands::ResolveArgs,
    content: &ContentArgs,
    client: &http::Client,
    correlation: Option<&str>,
) -> anyhow::Result<Sent> {
    let root = workspace_root()?;
    let (headers, body) = content.resolve()?;
    let target = if target.len() == 2 {
        Target::AdHoc {
            method: target[0].clone(),
            url: target[1].clone(),
        }
    } else {
        Target::File(PathBuf::from(&target[0]))
    };
    let options = SendOptions {
        source: "cli".to_owned(),
        actor: ctx.actor.as_str().to_owned(),
        env: resolve.env.clone(),
        profiles: resolve.profiles.clone(),
        vars: parse_vars(&resolve.vars)?,
        correlation: correlation.map(str::to_owned),
        headers,
        body,
    };
    let sent = core_send::send(&root, target, &options, client)?;
    let event_id = match sent.event {
        Ok(id) => Some(id),
        Err(error) => {
            output::warn(&format!("history event not recorded: {error}"));
            None
        }
    };
    Ok(Sent {
        exchange: sent.exchange,
        event_id,
    })
}

/// The workspace root, or the current directory when there is no workspace.
pub fn workspace_root() -> anyhow::Result<PathBuf> {
    let cwd = std::env::current_dir()?;
    Ok(workspace::find_root(&cwd).unwrap_or(cwd))
}

/// Origin of an exchange sent from this process.
pub fn event_meta(
    ctx: &Context,
    root: &Path,
    env: Option<String>,
    request_ref: Option<String>,
) -> EventMeta {
    EventMeta {
        source: "cli".to_owned(),
        actor: ctx.actor.as_str().to_owned(),
        session_id: None,
        workspace: Some(root.display().to_string()),
        env,
        request_ref,
    }
}

/// Store an exchange as a history event. Failure is a warning, not an error,
/// so the send still succeeds.
pub fn record(
    exchange: &http::Exchange,
    meta: EventMeta,
    retention: &RetentionOptions,
    correlation: Option<&str>,
) -> Option<String> {
    match core_send::record(exchange, meta, retention, correlation) {
        Ok(id) => Some(id),
        Err(error) => {
            output::warn(&format!("history event not recorded: {error}"));
            None
        }
    }
}

fn parse_vars(values: &[String]) -> anyhow::Result<BTreeMap<String, String>> {
    values
        .iter()
        .map(|value| {
            let (name, content) = value
                .split_once('=')
                .ok_or_else(|| anyhow::anyhow!("--var must use key=value"))?;
            Ok((name.to_owned(), content.to_owned()))
        })
        .collect()
}

/// Print a sent exchange in the output mode of `ctx`, then fail if it errored.
pub fn print_sent(ctx: &Context, sent: &Sent) -> anyhow::Result<()> {
    if ctx.is_json {
        output::print_exchange_json(&sent.exchange, sent.event_id.as_deref())?;
    } else {
        output::print_human(&sent.exchange, sent.event_id.as_deref());
    }
    if let Some(error) = &sent.exchange.error {
        bail!("{error}");
    }
    Ok(())
}

pub fn run(ctx: &Context, args: SendArgs) -> anyhow::Result<()> {
    let tag = args.tag.as_deref().map(|tag| tag.trim_start_matches('#'));
    if tag.is_some_and(str::is_empty) {
        bail!("tag must not be empty");
    }
    let sent = send_one(ctx, &args.target, &args.resolve, &args.content)?;
    if let (Some(tag), Some(id)) = (tag, &sent.event_id) {
        let tagged = History::open_default().and_then(|history| history.add_tag(id, tag));
        if let Err(error) = tagged {
            output::warn(&format!("could not tag #{tag}: {error}"));
        }
    }
    print_sent(ctx, &sent)
}
