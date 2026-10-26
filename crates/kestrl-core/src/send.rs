//! Sending one request and recording it in history. Shared by the CLI and the FFI app.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::collections;
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
    /// An unsaved request from the app, resolved and redacted like a file.
    Draft {
        request: request::RequestFile,
        request_path: Option<PathBuf>,
    },
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
    /// Headers that override the request's and its profiles' (`-H`); names match ignoring case
    pub headers: BTreeMap<String, String>,
    /// Replaces the request body (`--data`, `--json`)
    pub body: Option<request::Body>,
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
    let (request_ref, request_file) = match target {
        Target::Draft {
            request,
            request_path,
        } => (
            request_path
                .as_deref()
                .map(|path| request_ref(root, path))
                .transpose()?,
            request,
        ),
        Target::File(path) => (Some(request_ref(root, &path)?), request::load(&path)?),
        Target::AdHoc { method, url } => (None, request::ad_hoc(&method, &url)),
    };
    let mut request_file = request_file;
    if let Some(body) = options.body.clone() {
        request_file.body = Some(body);
    }
    // A request inside the workspace picks up its collection folders' settings.
    let collection_request = request_ref
        .as_deref()
        .filter(|path| !Path::new(path).is_absolute());
    let folder_env = match collection_request {
        Some(path) => collections::default_env_for(root, path)?,
        None => None,
    };
    let env_name = options
        .env
        .clone()
        .or(folder_env)
        .or(config.default_env.clone())
        .unwrap_or_else(|| "local".to_owned());
    let implicit_env = options.env.is_none() && config.default_env.is_none();
    let vars = Variables {
        overrides: options.vars.clone(),
        collection: match collection_request {
            Some(path) => collections::variables_for(root, path, &env_name)?,
            None => BTreeMap::new(),
        },
        environment: load_environment(root, &env_name, implicit_env)?,
        workspace: config.vars.clone(),
        redactor: Redactor::default(),
    };
    let resolved = request::resolve(
        request_file,
        root,
        &config,
        &env_name,
        &options.profiles,
        &options.headers,
        vars,
    )?;
    let exchange = http::send(client, resolved)?;
    let meta = EventMeta {
        source: options.source.clone(),
        actor: options.actor.clone(),
        session_id: None,
        workspace: Some(root.display().to_string()),
        env: Some(env_name.clone()),
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
