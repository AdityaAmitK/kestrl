//! Browsing and editing history from the app. Ids may be unique prefixes, as in the CLI.

use kestrl_core::duration::parse_since;
use kestrl_core::history::{
    self as core_history, EventDetail as CoreDetail, History, HistoryFilter,
};

use crate::error::KestrlError;
use crate::types::KeyValue;

/// Which events to list. Unset fields match everything.
#[derive(Debug, uniffi::Record)]
pub struct EventFilter {
    /// Full-text search over summary, note, url and bodies
    pub text: Option<String>,
    pub actor: Option<String>,
    pub tag: Option<String>,
    /// Only events from this long ago: `30s`, `2m`, `1h` or `2d`
    pub since: Option<String>,
    pub session_id: Option<String>,
    /// Newest first, at most this many
    pub limit: u32,
}

#[derive(Debug, uniffi::Record)]
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

#[derive(Debug, uniffi::Record)]
pub struct EventDetail {
    pub event: EventSummary,
    pub session_id: Option<String>,
    pub workspace: Option<String>,
    pub request_ref: Option<String>,
    pub correlation: Option<String>,
    pub note: Option<String>,
    /// Present for HTTP events
    pub http: Option<HttpExchange>,
}

/// Stored request and response. Secrets were masked before storage.
#[derive(Debug, uniffi::Record)]
pub struct HttpExchange {
    pub method: String,
    pub url: String,
    pub req_headers: Vec<KeyValue>,
    pub req_body: Option<String>,
    pub status: Option<u16>,
    pub res_headers: Vec<KeyValue>,
    pub res_body: Option<String>,
    pub duration_ms: u64,
    pub error: Option<String>,
    pub redacted: Vec<String>,
}

/// Events matching `filter`, newest first.
#[uniffi::export]
pub fn list_events(filter: EventFilter) -> Result<Vec<EventSummary>, KestrlError> {
    let filter = HistoryFilter {
        text: filter.text,
        actor: filter.actor,
        tag: filter.tag,
        since_ms: filter.since.as_deref().map(parse_since).transpose()?,
        session_id: filter.session_id,
        limit: filter.limit,
    };
    let events = History::open_default()?.list(&filter)?;
    Ok(events.into_iter().map(EventSummary::from).collect())
}

/// One event in full, with its HTTP exchange when it has one.
#[uniffi::export]
pub fn show_event(id: String) -> Result<EventDetail, KestrlError> {
    Ok(EventDetail::from(History::open_default()?.show(&id)?))
}

/// Add a tag to an event. A leading `#` is optional.
#[uniffi::export]
pub fn tag_event(id: String, tag: String) -> Result<(), KestrlError> {
    let tag = tag.trim_start_matches('#');
    if tag.is_empty() {
        return Err(KestrlError::InvalidInput {
            message: "tag must not be empty".to_owned(),
        });
    }
    History::open_default()?.add_tag(&id, tag)?;
    Ok(())
}

/// Remove a tag from an event. A leading `#` is optional.
#[uniffi::export]
pub fn untag_event(id: String, tag: String) -> Result<(), KestrlError> {
    History::open_default()?.remove_tag(&id, tag.trim_start_matches('#'))?;
    Ok(())
}

/// Set an event's note. An event with a note is saved and kept.
#[uniffi::export]
pub fn set_note(id: String, note: String) -> Result<(), KestrlError> {
    History::open_default()?.set_note(&id, &note)?;
    Ok(())
}

/// Compare two HTTP events; the result is the JSON of `EventDiff`.
#[uniffi::export]
pub fn diff_events(a: String, b: String) -> Result<String, KestrlError> {
    let history = History::open_default()?;
    let left = history.show(&a)?;
    let right = history.show(&b)?;
    let (Some(left), Some(right)) = (&left.http, &right.http) else {
        return Err(KestrlError::InvalidInput {
            message: "history diff compares two HTTP events".to_owned(),
        });
    };
    let diff = core_history::diff_exchanges(left, right);
    serde_json::to_string(&diff).map_err(|error| KestrlError::History {
        message: error.to_string(),
    })
}

impl From<kestrl_core::history::EventSummary> for EventSummary {
    fn from(event: kestrl_core::history::EventSummary) -> Self {
        Self {
            id: event.id,
            ts: event.ts,
            kind: event.kind,
            source: event.source,
            actor: event.actor,
            env: event.env,
            summary: event.summary,
            tags: event.tags,
            method: event.method,
            status: event.status,
            duration_ms: event.duration_ms,
        }
    }
}

impl From<CoreDetail> for EventDetail {
    fn from(detail: CoreDetail) -> Self {
        Self {
            event: EventSummary::from(detail.event),
            session_id: detail.session_id,
            workspace: detail.workspace,
            request_ref: detail.request_ref,
            correlation: detail.correlation,
            note: detail.note,
            http: detail.http.map(HttpExchange::from),
        }
    }
}

impl From<core_history::HttpExchange> for HttpExchange {
    fn from(http: core_history::HttpExchange) -> Self {
        Self {
            method: http.method,
            url: http.url,
            req_headers: KeyValue::from_pairs(http.req_headers),
            req_body: http.req_body,
            status: http.status,
            res_headers: KeyValue::from_pairs(http.res_headers),
            res_body: http.res_body,
            duration_ms: http.duration_ms,
            error: http.error,
            redacted: http.redacted,
        }
    }
}
