//! Recording HTTP exchanges as history events.

use rusqlite::params;
use serde::{Deserialize, Serialize};

use super::{History, RetentionOptions, now_ms};
use crate::Result;
use crate::http::Exchange;

/// Who and where an exchange happened. `session_id` is filled in when recording.
#[derive(Debug, Clone, Default)]
pub struct EventMeta {
    pub source: String,
    pub actor: String,
    pub session_id: Option<String>,
    pub workspace: Option<String>,
    pub env: Option<String>,
    /// Request file path relative to the workspace root; `None` for ad-hoc sends
    pub request_ref: Option<String>,
}

/// One HTTP exchange to record. Values must already be redacted.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewHttpEvent {
    pub source: String,
    pub actor: String,
    pub session_id: Option<String>,
    pub workspace: Option<String>,
    pub env: Option<String>,
    pub request_ref: Option<String>,
    pub correlation: Option<String>,
    pub method: String,
    pub url: String,
    pub req_headers: Vec<(String, String)>,
    pub req_body: Option<String>,
    pub status: Option<u16>,
    pub res_headers: Vec<(String, String)>,
    pub res_body: Option<String>,
    pub duration_ms: u64,
    pub error: Option<String>,
    pub redacted: Vec<String>,
}

impl NewHttpEvent {
    /// Map a sent exchange (already redacted) to an event.
    pub fn from_exchange(exchange: &Exchange, meta: EventMeta) -> Self {
        Self {
            source: meta.source,
            actor: meta.actor,
            session_id: meta.session_id,
            workspace: meta.workspace,
            env: meta.env,
            request_ref: meta.request_ref,
            correlation: None,
            method: exchange.method.clone(),
            url: exchange.url.clone(),
            req_headers: exchange
                .request_headers
                .iter()
                .map(|(name, value)| (name.clone(), value.clone()))
                .collect(),
            req_body: exchange.request_body.clone(),
            status: exchange.status,
            res_headers: exchange
                .response_headers
                .iter()
                .map(|(name, value)| (name.clone(), value.clone()))
                .collect(),
            res_body: (!exchange.response_body.is_empty()).then(|| exchange.response_body.clone()),
            duration_ms: u64::try_from(exchange.duration_ms).unwrap_or(u64::MAX),
            error: exchange.error.clone(),
            redacted: exchange.redacted.clone(),
        }
    }
}

impl History {
    /// Store an HTTP exchange as one event, then prune old history.
    pub fn record_http(
        &mut self,
        event: &NewHttpEvent,
        retention: &RetentionOptions,
    ) -> Result<String> {
        let id = ulid::Ulid::new().to_string();
        let ts = now_ms();
        let summary = summarize(event);
        let req_body = event
            .req_body
            .as_deref()
            .map(|body| cut_body(body, retention.max_body_bytes));
        let res_body = event
            .res_body
            .as_deref()
            .map(|body| cut_body(body, retention.max_body_bytes));
        let req_headers = serde_json::to_string(&event.req_headers)?;
        let res_headers = serde_json::to_string(&event.res_headers)?;
        let redacted = serde_json::to_string(&event.redacted)?;

        let tx = self.conn.transaction()?;
        tx.execute(
            "INSERT INTO events (id, ts, kind, source, actor, session_id, workspace, env, request_ref, correlation, summary)
             VALUES (?1, ?2, 'http', ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                id,
                ts,
                event.source,
                event.actor,
                event.session_id,
                event.workspace,
                event.env,
                event.request_ref,
                event.correlation,
                summary,
            ],
        )?;
        tx.execute(
            "INSERT INTO http_exchanges (event_id, method, url, req_headers, req_body, status, res_headers, res_body, duration_ms, error, redacted)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                id,
                event.method,
                event.url,
                req_headers,
                req_body,
                event.status,
                res_headers,
                res_body,
                event.duration_ms,
                event.error,
                redacted,
            ],
        )?;
        tx.execute(
            "INSERT INTO events_fts (event_id, summary, url, req_body, res_body) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![id, summary, event.url, req_body, res_body],
        )?;
        tx.commit()?;

        self.prune(retention)?;
        Ok(id)
    }
}

/// `GET /users 200 34ms`
fn summarize(event: &NewHttpEvent) -> String {
    let status = event
        .status
        .map_or_else(|| "ERR".to_owned(), |s| s.to_string());
    format!(
        "{} {} {} {}ms",
        event.method,
        path_of(&event.url),
        status,
        event.duration_ms
    )
}

fn path_of(url: &str) -> &str {
    let rest = url.split_once("://").map_or(url, |(_, rest)| rest);
    let path = rest.find('/').map_or("/", |i| &rest[i..]);
    path.split_once('?').map_or(path, |(path, _)| path)
}

fn cut_body(body: &str, max_bytes: usize) -> &str {
    if body.len() <= max_bytes {
        return body;
    }
    let mut end = max_bytes;
    while !body.is_char_boundary(end) {
        end -= 1;
    }
    &body[..end]
}
