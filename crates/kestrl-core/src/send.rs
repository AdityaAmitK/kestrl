//! Sending one request and recording it in history. Shared by the CLI and the FFI app.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::env::Environment;
use crate::history::{EventMeta, History, NewHttpEvent, RetentionOptions};
use crate::http::{self, Client, Exchange};
use crate::request;
use crate::secrets::Redactor;
use crate::vars::Variables;
use crate::workspace::WorkspaceConfig;
use crate::{Error, Result};

/// What to send.
pub enum Target {
    /// Request file, absolute or relative to the working directory
    File(PathBuf),
    AdHoc {
        method: String,
        url: String,
    },
}

/// Who sends, and how the request resolves.
#[derive(Debug, Clone, Default)]
pub struct SendOptions {
    /// Where the send came from: `cli` or `app`
    pub source: String,
    pub actor: String,
    /// Environment; the workspace default when `None`
    pub env: Option<String>,
    pub profiles: Vec<String>,
    /// `--var` overrides, the highest-precedence variables
    pub vars: BTreeMap<String, String>,
    /// Id of the event this send repeats, for replay
    pub correlation: Option<String>,
}

/// An exchange and the history event recorded for it.
pub struct Sent {
    pub exchange: Exchange,
    /// Why recording failed, if it did. The send itself still succeeded.
    pub event: Result<String>,
}

/// Resolve, send and record one request in the workspace at `root`.
pub fn send(root: &Path, target: Target, options: &SendOptions, client: &Client) -> Result<Sent> {
    let config = WorkspaceConfig::load(root)?;
    let env_name = options
        .env
        .as_deref()
        .or(config.default_env.as_deref())
        .unwrap_or("local");
    let implicit_env = options.env.is_none() && config.default_env.is_none();
    let vars = Variables {
        overrides: options.vars.clone(),
        environment: load_environment(root, env_name, implicit_env)?,
        workspace: config.vars.clone(),
        redactor: Redactor::default(),
    };
    let (request_ref, request_file) = match target {
        Target::File(path) => (Some(request_ref(root, &path)?), request::load(&path)?),
        Target::AdHoc { method, url } => (None, request::ad_hoc(&method, &url)),
    };
    let resolved = request::resolve(
        request_file,
        root,
        &config,
        env_name,
        &options.profiles,
        vars,
    )?;
    let exchange = http::send(client, resolved)?;
    let meta = EventMeta {
        source: options.source.clone(),
        actor: options.actor.clone(),
        session_id: None,
        workspace: Some(root.display().to_string()),
        env: options.env.clone().or(config.default_env),
        request_ref,
    };
    let event = record(
        &exchange,
        meta,
        &config.retention,
        options.correlation.as_deref(),
    );
    Ok(Sent { exchange, event })
}

/// Store an exchange as a history event in the default database, then prune.
pub fn record(
    exchange: &Exchange,
    meta: EventMeta,
    retention: &RetentionOptions,
    correlation: Option<&str>,
) -> Result<String> {
    let mut history = History::open_default()?;
    let session_id = history.current_session()?.map(|session| session.id);
    let event = NewHttpEvent {
        correlation: correlation.map(str::to_owned),
        ..NewHttpEvent::from_exchange(exchange, EventMeta { session_id, ..meta })
    };
    history.record_http(&event, retention)
}

/// Variables of the environment. A missing file is an empty environment only
/// when no environment was asked for, so the default `local` may be absent.
fn load_environment(root: &Path, name: &str, implicit: bool) -> Result<BTreeMap<String, String>> {
    match Environment::load(root, name) {
        Ok(environment) => Ok(environment.vars),
        Err(Error::Read { source, .. })
            if implicit && source.kind() == std::io::ErrorKind::NotFound =>
        {
            Ok(BTreeMap::new())
        }
        Err(Error::Read { source, .. }) if source.kind() == std::io::ErrorKind::NotFound => {
            Err(Error::EnvironmentNotFound {
                name: name.to_owned(),
                available: Environment::names(root)?.join(", "),
            })
        }
        Err(error) => Err(error),
    }
}

/// The request file path relative to the workspace root, or absolute if it lies outside.
fn request_ref(root: &Path, path: &Path) -> Result<String> {
    let absolute = std::env::current_dir()?.join(path);
    Ok(absolute.strip_prefix(root).map_or_else(
        |_| absolute.display().to_string(),
        |relative| relative.display().to_string(),
    ))
}
