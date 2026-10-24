//! Recording database queries as history events.

use rusqlite::params;
use serde::{Deserialize, Serialize};

use super::{History, now_ms};
use crate::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewDbEvent {
    pub source: String,
    pub actor: String,
    pub workspace: Option<String>,
    pub env: Option<String>,
    pub connection: String,
    pub sql: String,
    pub rows: Option<u64>,
    pub duration_ms: u64,
    pub error: Option<String>,
}

pub(super) fn record_db(history: &mut History, event: &NewDbEvent) -> Result<String> {
    let id = ulid::Ulid::new().to_string();
    let ts = now_ms();
    let sql = event.sql.chars().take(60).collect::<String>();
    let summary = format!(
        "{}: {} {} rows {}ms",
        event.connection,
        sql,
        event
            .rows
            .map_or_else(|| "ERR".to_owned(), |rows| rows.to_string()),
        event.duration_ms
    );
    let tx = history.transaction()?;
    tx.execute(
        "INSERT INTO events (id, ts, kind, source, actor, workspace, env, summary) VALUES (?1, ?2, 'db', ?3, ?4, ?5, ?6, ?7)",
        params![id, ts, event.source, event.actor, event.workspace, event.env, summary],
    )?;
    tx.execute(
        "INSERT INTO db_queries (event_id, connection, sql, rows, duration_ms, error) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![id, event.connection, event.sql, event.rows, event.duration_ms, event.error],
    )?;
    tx.execute(
        "INSERT INTO events_fts (event_id, summary) VALUES (?1, ?2)",
        params![id, summary],
    )?;
    tx.commit()?;
    history.prune_default()?;
    Ok(id)
}
