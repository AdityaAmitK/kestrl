//! Recording database queries as history events.

use super::{EventMeta, History, RetentionOptions, now_ms};
use crate::Result;
use rusqlite::params;

/// One database query to record. SQL and errors must already be redacted.
#[derive(Debug, Clone)]
pub struct NewDbEvent {
    /// Source, actor, session, workspace and environment for the query.
    pub meta: EventMeta,
    /// Configured connection name.
    pub connection: String,
    /// Redacted SQL statement.
    pub sql: String,
    /// Number of result rows or affected rows, when available.
    pub rows: Option<u64>,
    /// Query duration in milliseconds.
    pub duration_ms: u64,
    /// Redacted database error, if the query failed.
    pub error: Option<String>,
}

impl History {
    /// Store a database query as one event, then prune old history.
    pub fn record_db(
        &mut self,
        event: &NewDbEvent,
        retention: &RetentionOptions,
    ) -> Result<String> {
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
        let tx = self.conn.transaction()?;
        tx.execute(
            "INSERT INTO events (id, ts, kind, source, actor, session_id, workspace, env, summary) VALUES (?1, ?2, 'db', ?3, ?4, ?5, ?6, ?7, ?8)",
            params![id, ts, event.meta.source, event.meta.actor, event.meta.session_id, event.meta.workspace, event.meta.env, summary],
        )?;
        tx.execute(
            "INSERT INTO db_queries (event_id, connection, sql, rows, duration_ms, error) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![id, event.connection, event.sql, event.rows, event.duration_ms, event.error],
        )?;
        tx.execute(
            "INSERT INTO events_fts (event_id, summary, req_body) VALUES (?1, ?2, ?3)",
            params![id, summary, event.sql],
        )?;
        tx.commit()?;
        self.prune(retention)?;
        Ok(id)
    }
}
