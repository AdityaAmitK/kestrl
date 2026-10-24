//! `kestrl send`: send one request (file or METHOD URL), recording it in history.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::bail;
use clap::Args;
use kestrl_core::env::Environment;
use kestrl_core::history::{EventMeta, History, NewHttpEvent, RetentionOptions};
use kestrl_core::http;
use kestrl_core::request;
use kestrl_core::secrets;
use kestrl_core::vars::Variables;
use kestrl_core::workspace::{self, WorkspaceConfig};

use crate::commands::output;
use crate::context::Context;

#[derive(Debug, Args)]
pub struct SendArgs {
    /// Request file path, or an HTTP method followed by a URL
    #[arg(required = true, num_args = 1..=2)]
    pub target: Vec<String>,
    #[command(flatten)]
    pub resolve: crate::commands::ResolveArgs,
    /// Tag to add to the recorded event
    #[arg(long)]
    pub tag: Option<String>,
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
) -> anyhow::Result<Sent> {
    let client = http::client()?;
    send_one_with_client(ctx, target, resolve, &client, None)
}

pub fn send_one_with_client(
    ctx: &Context,
    target: &[String],
    resolve: &crate::commands::ResolveArgs,
    client: &http::Client,
    correlation: Option<&str>,
) -> anyhow::Result<Sent> {
    let root = workspace_root()?;
    let config = WorkspaceConfig::load(&root)?;
    let env_name = resolve.env.as_deref().or(config.default_env.as_deref());
    let env_name = env_name.unwrap_or("local");
    let environment = match Environment::load(&root, env_name) {
        Ok(environment) => environment.vars,
        Err(kestrl_core::Error::Read { source, .. })
            if resolve.env.is_none()
                && config.default_env.is_none()
                && source.kind() == std::io::ErrorKind::NotFound =>
        {
            BTreeMap::new()
        }
        Err(kestrl_core::Error::Read { source, .. })
            if source.kind() == std::io::ErrorKind::NotFound =>
        {
            let available = Environment::names(&root)?.join(", ");
            return Err(kestrl_core::Error::EnvironmentNotFound {
                name: env_name.to_owned(),
                available,
            }
            .into());
        }
        Err(error) => return Err(error.into()),
    };
    let vars = Variables {
        overrides: parse_vars(&resolve.vars)?,
        environment,
        workspace: config.vars.clone(),
        redactor: secrets::Redactor::default(),
    };
    let request_ref = if target.len() == 2 {
        None
    } else {
        Some(request_ref(&root, &target[0])?)
    };
    let request_file = if target.len() == 2 {
        request::ad_hoc(&target[0], &target[1])
    } else {
        request::load(Path::new(&target[0]))?
    };
    let resolved = request::resolve(
        request_file,
        &root,
        &config,
        env_name,
        &resolve.profiles,
        vars,
    )?;
    let exchange = http::send(client, resolved)?;
    let meta = event_meta(
        ctx,
        &root,
        resolve.env.clone().or(config.default_env.clone()),
        request_ref,
    );
    let event_id = record(&exchange, meta, &config.retention, correlation);
    Ok(Sent { exchange, event_id })
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
    let result = History::open_default().and_then(|mut history| {
        let session_id = history.current_session()?.map(|session| session.id);
        let event = NewHttpEvent {
            correlation: correlation.map(str::to_owned),
            ..NewHttpEvent::from_exchange(exchange, EventMeta { session_id, ..meta })
        };
        history.record_http(&event, retention)
    });
    match result {
        Ok(id) => Some(id),
        Err(error) => {
            output::warn(&format!("history event not recorded: {error}"));
            None
        }
    }
}

/// The request file path relative to the workspace root, or absolute if it lies outside.
fn request_ref(root: &Path, path: &str) -> anyhow::Result<String> {
    let absolute = std::env::current_dir()?.join(path);
    Ok(absolute.strip_prefix(root).map_or_else(
        |_| absolute.display().to_string(),
        |relative| relative.display().to_string(),
    ))
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
    let sent = send_one(ctx, &args.target, &args.resolve)?;
    if let (Some(tag), Some(id)) = (tag, &sent.event_id) {
        let tagged = History::open_default().and_then(|history| history.add_tag(id, tag));
        if let Err(error) = tagged {
            output::warn(&format!("could not tag #{tag}: {error}"));
        }
    }
    print_sent(ctx, &sent)
}
