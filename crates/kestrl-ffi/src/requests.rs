//! Sending requests from the app. Each send is recorded in history as actor `me`, source `app`.

use std::path::{Path, PathBuf};

use kestrl_core::actor::Actor;
use kestrl_core::http;
use kestrl_core::request::{self, Body};
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

/// A request file as written, with `{{variables}}` left unresolved.
#[derive(Debug, uniffi::Record)]
pub struct RequestDetail {
    /// Original file text used to reject saving over external edits. Empty for a new draft.
    pub source_text: String,
    pub name: Option<String>,
    pub method: String,
    pub url: String,
    pub profiles: Vec<String>,
    pub headers: Vec<KeyValue>,
    pub query: Vec<KeyValue>,
    /// `json`, `text` or `form`; unset when the request has no body
    pub body_kind: Option<String>,
    /// Form fields are shown as `key = value` lines
    pub body: Option<String>,
}

/// Read the request file at `path` without sending it.
#[uniffi::export]
pub fn read_request_file(
    workspace_root: String,
    path: String,
) -> Result<RequestDetail, KestrlError> {
    let path = PathBuf::from(workspace_root).join(path);
    let source_text = std::fs::read_to_string(&path).map_err(kestrl_core::Error::from)?;
    let file: request::RequestFile =
        toml::from_str(&source_text).map_err(|source| kestrl_core::Error::Toml { path, source })?;
    let (body_kind, body) = match file.body {
        None => (None, None),
        Some(Body::Json { content }) => (Some("json"), Some(content)),
        Some(Body::Text { content }) => (Some("text"), Some(content)),
        Some(Body::Form { fields }) => (
            Some("form"),
            Some(
                fields
                    .iter()
                    .map(|(key, value)| format!("{key}={value}"))
                    .collect::<Vec<_>>()
                    .join("\n"),
            ),
        ),
    };
    Ok(RequestDetail {
        source_text,
        name: file.name,
        method: file.method,
        url: file.url,
        profiles: file.profiles,
        headers: KeyValue::from_map(file.headers),
        query: KeyValue::from_map(file.query),
        body_kind: body_kind.map(str::to_owned),
        body,
    })
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

/// Send an editable draft through the same resolution, redaction and history path as the CLI.
#[uniffi::export]
pub fn send_request_draft(
    workspace_root: String,
    draft: RequestDetail,
    env: Option<String>,
    request_path: Option<String>,
) -> Result<SentExchange, KestrlError> {
    let root = PathBuf::from(workspace_root);
    let request_path = request_path.and_then(|path| {
        let full = root.join(path);
        (std::fs::read_to_string(&full).ok().as_deref() == Some(draft.source_text.as_str()))
            .then_some(full)
    });
    run_send(
        &root,
        Target::Draft {
            request: draft_file(draft)?,
            request_path,
        },
        env,
        Vec::new(),
        Vec::new(),
    )
}

/// Save unresolved draft values; never overwrite a file changed since the app read it.
#[uniffi::export]
pub fn save_request_draft(
    workspace_root: String,
    path: String,
    draft: RequestDetail,
) -> Result<RequestDetail, KestrlError> {
    let root = std::fs::canonicalize(&workspace_root).map_err(kestrl_core::Error::from)?;
    let target = root.join(&path);
    let parent = target.parent().ok_or_else(|| KestrlError::InvalidInput {
        message: "Choose a request file inside the workspace".to_owned(),
    })?;
    let parent = std::fs::canonicalize(parent).map_err(kestrl_core::Error::from)?;
    if !parent.starts_with(&root) || !path.ends_with(".req.toml") || target.is_symlink() {
        return Err(KestrlError::InvalidInput {
            message: "Request files must stay inside the workspace and end with .req.toml"
                .to_owned(),
        });
    }
    let original = draft.source_text.clone();
    let file = draft_file(draft)?;
    let text = toml::to_string_pretty(&file).map_err(|_| KestrlError::File {
        message: "Could not encode the request file".to_owned(),
    })?;
    use std::io::Write;
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|_| KestrlError::File {
            message: "Could not create a temporary request file".to_owned(),
        })?
        .as_nanos();
    let temporary = parent.join(format!(".kestrl-{}-{stamp}.tmp", std::process::id()));
    let result = (|| -> Result<(), KestrlError> {
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut output = options.open(&temporary).map_err(kestrl_core::Error::from)?;
        output
            .write_all(text.as_bytes())
            .map_err(kestrl_core::Error::from)?;
        output.sync_all().map_err(kestrl_core::Error::from)?;
        if original.is_empty() {
            // A hard link publishes the complete new file without replacing an existing one.
            std::fs::hard_link(&temporary, &target).map_err(kestrl_core::Error::from)?;
        } else {
            let current = std::fs::read_to_string(&target).map_err(kestrl_core::Error::from)?;
            if current != original {
                return Err(KestrlError::File { message: "This request changed outside Kestrl. Discard draft changes to reload it, or save a copy.".to_owned() });
            }
            // Move the actual disk version aside, then publish without clobbering another editor's replacement.
            let previous = parent.join(format!(".kestrl-previous-{}-{stamp}", std::process::id()));
            std::fs::rename(&target, &previous).map_err(kestrl_core::Error::from)?;
            let replaced = std::fs::read_to_string(&previous).map_err(kestrl_core::Error::from)?;
            if replaced != original {
                let _ = std::fs::hard_link(&previous, &target);
                return Err(KestrlError::File {
                    message: format!(
                        "The file changed while saving. Its external version is preserved at {}. Review it before saving again.",
                        previous.display()
                    ),
                });
            }
            if std::fs::hard_link(&temporary, &target).is_err() {
                let _ = std::fs::hard_link(&previous, &target);
                return Err(KestrlError::File {
                    message: format!(
                        "Could not publish the request without replacing another file. The previous version is preserved at {}.",
                        previous.display()
                    ),
                });
            }
            let _ = std::fs::remove_file(previous);
        }
        Ok(())
    })();
    let _ = std::fs::remove_file(&temporary);
    result?;
    read_request_file(workspace_root, path)
}

fn draft_file(draft: RequestDetail) -> Result<request::RequestFile, KestrlError> {
    let mut headers = KeyValue::into_map(draft.headers);
    let content_type = match draft.body_kind.as_deref() {
        Some("json") => Some("application/json"),
        Some("form") => Some("application/x-www-form-urlencoded"),
        Some("text") => Some("text/plain"),
        _ => None,
    };
    if !headers
        .keys()
        .any(|key| key.eq_ignore_ascii_case("content-type"))
        && let Some(content_type) = content_type
    {
        headers.insert("Content-Type".to_owned(), content_type.to_owned());
    }
    let body = match draft.body_kind.as_deref() {
        None | Some("none") => None,
        Some("json") => Some(Body::Json {
            content: draft.body.unwrap_or_default(),
        }),
        Some("text") => Some(Body::Text {
            content: draft.body.unwrap_or_default(),
        }),
        Some("form") => {
            let mut fields = std::collections::BTreeMap::new();
            for line in draft
                .body
                .unwrap_or_default()
                .lines()
                .filter(|line| !line.trim().is_empty())
            {
                let (key, value) =
                    line.split_once('=')
                        .ok_or_else(|| KestrlError::InvalidInput {
                            message: "Form body lines must use key=value".to_owned(),
                        })?;
                fields.insert(key.trim().to_owned(), value.to_owned());
            }
            Some(Body::Form { fields })
        }
        Some(_) => {
            return Err(KestrlError::InvalidInput {
                message: "Choose JSON, text, form or no body".to_owned(),
            });
        }
    };
    Ok(request::RequestFile {
        name: draft.name,
        method: draft.method,
        url: draft.url,
        profiles: draft.profiles,
        headers,
        query: KeyValue::into_map(draft.query),
        body,
    })
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
