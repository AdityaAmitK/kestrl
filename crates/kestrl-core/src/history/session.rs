//! Sessions: named working stretches that group events.

use rusqlite::{OptionalExtension, Row, params};
use serde::{Deserialize, Serialize};
use ulid::Ulid;

use super::{History, now_ms};
use crate::{Error, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub name: Option<String>,
    pub actor: String,
    pub started_at: i64,
}

impl History {
    /// Open a new session and return its id.
    pub fn start_session(&self, name: Option<&str>, actor: &str) -> Result<String> {
        let id = Ulid::new().to_string();
        self.conn.execute(
            "INSERT INTO sessions (id, name, actor, started_at) VALUES (?1, ?2, ?3, ?4)",
            params![id, name, actor, now_ms()],
        )?;
        Ok(id)
    }

    /// Close the current session and return it.
    pub fn end_session(&self) -> Result<Session> {
        let session = self.current_session()?.ok_or(Error::NoOpenSession)?;
        self.conn.execute(
            "UPDATE sessions SET ended_at = ?2 WHERE id = ?1",
            params![session.id, now_ms()],
        )?;
        Ok(session)
    }

    /// The open session that was started most recently, if any.
    pub fn current_session(&self) -> Result<Option<Session>> {
        Ok(self
            .conn
            .query_row(
                "SELECT id, name, actor, started_at FROM sessions
                 WHERE ended_at IS NULL ORDER BY started_at DESC, id DESC LIMIT 1",
                [],
                session_row,
            )
            .optional()?)
    }
}

fn session_row(row: &Row) -> rusqlite::Result<Session> {
    Ok(Session {
        id: row.get(0)?,
        name: row.get(1)?,
        actor: row.get(2)?,
        started_at: row.get(3)?,
    })
}
