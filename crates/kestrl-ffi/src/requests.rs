//! Sending requests from the app. Each send is recorded in history as actor `me`, source `app`.

use std::path::{Path, PathBuf};

use kestrl_core::actor::Actor;
use kestrl_core::http;
use kestrl_core::send::{self as core_send, SendOptions, Target};

use crate::error::KestrlError;
use crate::types::KeyValue;

/// The outcome of one send. Server and network failures are in `error`, not thrown.
#[derive(Debug, uniffi::Record)]
pub struct SentExchange {
    /// History event id, when recording succeeded
    pub event_id: Option<String>,
    /// Why recording failed, when it did; the send itself still succeeded
    pub history_error: Option<String>,
    pub method: String,
    pub url: String,
    pub status: Option<u16>,
    pub duration_ms: u64,
    pub headers: Vec<KeyValue>,
    pub body: String,
    pub error: Option<String>,
    /// Names of the secrets that were masked in this exchange
    pub redacted: Vec<String>,
}

/// Send the request file at `path`, relative to the workspace root.
#[uniffi::export]
pub fn send_request_file(
    workspace_root: String,
    path: String,
    env: Option<String>,
    profiles: Vec<String>,
    vars: Vec<KeyValue>,
) -> Result<SentExchange, KestrlError> {
    let root = PathBuf::from(&workspace_root);
    let target = Target::File(root.join(path));
    run_send(&root, target, env, profiles, vars)
}

/// Send a method and URL that has no request file.
#[uniffi::export]
pub fn send_ad_hoc(
    workspace_root: String,
    method: String,
    url: String,
    env: Option<String>,
) -> Result<SentExchange, KestrlError> {
    let root = PathBuf::from(&workspace_root);
    run_send(
        &root,
        Target::AdHoc { method, url },
        env,
        Vec::new(),
        Vec::new(),
    )
}

fn run_send(
    root: &Path,
    target: Target,
    env: Option<String>,
    profiles: Vec<String>,
    vars: Vec<KeyValue>,
) -> Result<SentExchange, KestrlError> {
    let options = SendOptions {
        source: "app".to_owned(),
        actor: Actor::Me.as_str().to_owned(),
        env,
        profiles,
        vars: KeyValue::into_map(vars),
        correlation: None,
    };
    let client = http::client()?;
    let sent = core_send::send(root, target, &options, &client)?;
    let (event_id, history_error) = match sent.event {
        Ok(id) => (Some(id), None),
        Err(error) => (None, Some(error.to_string())),
    };
    let exchange = sent.exchange;
    Ok(SentExchange {
        event_id,
        history_error,
        method: exchange.method,
        url: exchange.url,
        status: exchange.status,
        duration_ms: u64::try_from(exchange.duration_ms).unwrap_or(u64::MAX),
        headers: KeyValue::from_map(exchange.response_headers.into_iter().collect()),
        body: exchange.response_body,
        error: exchange.error,
        redacted: exchange.redacted,
    })
}
