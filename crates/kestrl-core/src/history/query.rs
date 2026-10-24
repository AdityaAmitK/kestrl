//! Listing and showing history events. Ids may be given as unique prefixes.

use rusqlite::types::Type;
use rusqlite::{OptionalExtension, Row, params};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use super::History;
use crate::{Error, Result};

/// Filters for `History::list`. Unset fields match everything.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryFilter {
    /// Full-text search over summary, note, url and bodies
    pub text: Option<String>,
    pub actor: Option<String>,
    pub tag: Option<String>,
    /// Only events at or after this unix ms timestamp
    pub since_ms: Option<i64>,
    pub session_id: Option<String>,
    /// Newest events first, at most this many
    pub limit: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventSummary {
    pub id: String,
    pub ts: i64,
    pub kind: String,
    pub source: String,
    pub actor: String,
    pub env: Option<String>,
    pub summary: String,
    pub tags: Vec<String>,
    /// HTTP events only
    pub method: Option<String>,
    /// HTTP events only; unset when the request failed before a response
    pub status: Option<u16>,
    /// HTTP events only
    pub duration_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventDetail {
    #[serde(flatten)]
    pub event: EventSummary,
    pub session_id: Option<String>,
    pub workspace: Option<String>,
    pub request_ref: Option<String>,
    pub correlation: Option<String>,
    pub note: Option<String>,
    pub http: Option<HttpExchange>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpExchange {
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

const SUMMARY_COLUMNS: &str = "e.id, e.ts, e.kind, e.source, e.actor, e.env, e.summary,
    (SELECT json_group_array(t.tag) FROM tags t WHERE t.event_id = e.id),
    (SELECT h.method FROM http_exchanges h WHERE h.event_id = e.id),
    (SELECT h.status FROM http_exchanges h WHERE h.event_id = e.id),
    (SELECT h.duration_ms FROM http_exchanges h WHERE h.event_id = e.id)";

impl History {
    pub fn list(&self, filter: &HistoryFilter) -> Result<Vec<EventSummary>> {
        let sql = format!(
            "SELECT {SUMMARY_COLUMNS} FROM events e
             WHERE (?1 IS NULL OR e.actor = ?1)
               AND (?2 IS NULL OR e.ts >= ?2)
               AND (?3 IS NULL OR e.session_id = ?3)
               AND (?4 IS NULL OR EXISTS (SELECT 1 FROM tags t WHERE t.event_id = e.id AND t.tag = ?4))
               AND (?5 IS NULL OR e.id IN (SELECT event_id FROM events_fts WHERE events_fts MATCH ?5))
             ORDER BY e.ts DESC, e.id DESC
             LIMIT ?6"
        );
        let text = filter.text.as_deref().and_then(fts_query);
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(
            params![
                filter.actor,
                filter.since_ms,
                filter.session_id,
                filter.tag,
                text,
                i64::from(filter.limit),
            ],
            summary_row,
        )?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    pub fn show(&self, id_prefix: &str) -> Result<EventDetail> {
        let id = self.resolve_id(id_prefix)?;
        let sql = format!(
            "SELECT {SUMMARY_COLUMNS}, e.session_id, e.workspace, e.request_ref, e.correlation, e.note
             FROM events e WHERE e.id = ?1"
        );
        let detail = self.conn.query_row(&sql, params![id], |row| {
            Ok(EventDetail {
                event: summary_row(row)?,
                session_id: row.get(11)?,
                workspace: row.get(12)?,
                request_ref: row.get(13)?,
                correlation: row.get(14)?,
                note: row.get(15)?,
                http: None,
            })
        })?;
        let http = self
            .conn
            .query_row(
                "SELECT method, url, req_headers, req_body, status, res_headers, res_body, duration_ms, error, redacted
                 FROM http_exchanges WHERE event_id = ?1",
                params![id],
                |row| {
                    Ok(HttpExchange {
                        method: row.get(0)?,
                        url: row.get(1)?,
                        req_headers: json_column(row, 2)?,
                        req_body: row.get(3)?,
                        status: row.get(4)?,
                        res_headers: json_column(row, 5)?,
                        res_body: row.get(6)?,
                        duration_ms: row.get(7)?,
                        error: row.get(8)?,
                        redacted: json_column(row, 9)?,
                    })
                },
            )
            .optional()?;
        Ok(EventDetail { http, ..detail })
    }

    /// The one event whose id starts with `prefix`.
    pub(crate) fn resolve_id(&self, prefix: &str) -> Result<String> {
        let prefix = prefix.to_ascii_uppercase();
        let mut stmt = self
            .conn
            .prepare("SELECT id FROM events WHERE substr(id, 1, length(?1)) = ?1 LIMIT 2")?;
        let ids = stmt
            .query_map(params![prefix], |row| row.get::<_, String>(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        match ids.as_slice() {
            [] => Err(Error::EventNotFound(prefix)),
            [id] => Ok(id.clone()),
            _ => Err(Error::AmbiguousId(prefix)),
        }
    }
}

fn summary_row(row: &Row) -> rusqlite::Result<EventSummary> {
    Ok(EventSummary {
        id: row.get(0)?,
        ts: row.get(1)?,
        kind: row.get(2)?,
        source: row.get(3)?,
        actor: row.get(4)?,
        env: row.get(5)?,
        summary: row.get(6)?,
        tags: json_column(row, 7)?,
        method: row.get(8)?,
        status: row.get(9)?,
        duration_ms: row.get(10)?,
    })
}

/// Read a column holding JSON text.
fn json_column<T: DeserializeOwned>(row: &Row, idx: usize) -> rusqlite::Result<T> {
    let text: String = row.get(idx)?;
    serde_json::from_str(&text)
        .map_err(|e| rusqlite::Error::FromSqlConversionFailure(idx, Type::Text, Box::new(e)))
}

/// Each word must match; quoting keeps punctuation literal.
fn fts_query(text: &str) -> Option<String> {
    let terms: Vec<String> = text
        .split_whitespace()
        .map(|word| format!("\"{}\"", word.replace('"', "\"\"")))
        .collect();
    (!terms.is_empty()).then(|| terms.join(" "))
}
