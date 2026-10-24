//! Recording captured HTTP exchanges as capture history events.

use rusqlite::params;

use super::{EventMeta, History, RetentionOptions, now_ms};
use crate::Result;

/// One captured exchange to save in history.
#[derive(Debug, Clone)]
pub struct NewCapture {
    /// History metadata shared with sent HTTP exchanges.
    pub meta: EventMeta,
    /// HTTP method.
    pub method: String,
    /// Full request URL.
    pub url: String,
    /// Request headers.
    pub req_headers: Vec<(String, String)>,
    /// Request body, if present.
    pub req_body: Option<String>,
    /// HTTP status.
    pub status: Option<u16>,
    /// Response headers.
    pub res_headers: Vec<(String, String)>,
    /// Response body, if present.
    pub res_body: Option<String>,
    /// Exchange duration in milliseconds.
    pub duration_ms: u64,
}

impl History {
    /// Store a captured exchange as one capture event.
    pub fn record_capture(
        &mut self,
        capture: &NewCapture,
        retention: &RetentionOptions,
    ) -> Result<String> {
        let id = ulid::Ulid::new().to_string();
        let ts = now_ms();
        let summary = format!(
            "{} {} {}ms",
            capture.method,
            path_of(&capture.url),
            capture.duration_ms
        );
        let req_headers = serde_json::to_string(&capture.req_headers)?;
        let res_headers = serde_json::to_string(&capture.res_headers)?;
        let tx = self.conn.transaction()?;
        tx.execute(
            "INSERT INTO events (id, ts, kind, source, actor, session_id, workspace, env, request_ref, correlation, summary) VALUES (?1, ?2, 'capture', ?3, ?4, ?5, ?6, ?7, ?8, NULL, ?9)",
            params![id, ts, capture.meta.source, capture.meta.actor, capture.meta.session_id, capture.meta.workspace, capture.meta.env, capture.meta.request_ref, summary],
        )?;
        tx.execute(
            "INSERT INTO http_exchanges (event_id, method, url, req_headers, req_body, status, res_headers, res_body, duration_ms, redacted)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, '[]')",
            params![id, capture.method, capture.url, req_headers, capture.req_body, capture.status, res_headers, capture.res_body, capture.duration_ms],
        )?;
        tx.execute(
            "INSERT INTO events_fts (event_id, summary, url, req_body, res_body) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![id, summary, capture.url, capture.req_body, capture.res_body],
        )?;
        tx.commit()?;
        self.prune(retention)?;
        Ok(id)
    }
}

fn path_of(url: &str) -> &str {
    let rest = url.split_once("://").map_or(url, |(_, rest)| rest);
    let path = rest.find('/').map_or("/", |index| &rest[index..]);
    path.split_once('?').map_or(path, |(path, _)| path)
}
