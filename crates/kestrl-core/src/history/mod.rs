//! History store: SQLite at `~/Library/Application Support/kestrl/history.db`.
//!
//! `record` writes events, `query` reads them, `edit` changes saved ones,
//! `retention` prunes unsaved ones and `session` groups events.

mod edit;
mod query;
mod record;
mod retention;
mod session;

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use directories::ProjectDirs;
use rusqlite::Connection;

use crate::{Error, Result};

pub use query::{EventDetail, EventSummary, HistoryFilter, HttpExchange};
pub use record::NewHttpEvent;
pub use retention::RetentionOptions;
pub use session::Session;

const MIGRATION_0001: &str = include_str!("../../migrations/0001_init.sql");

/// Handle to the history database.
pub struct History {
    conn: Connection,
}

impl History {
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let conn = Connection::open(path)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        let version: i64 = conn.pragma_query_value(None, "user_version", |r| r.get(0))?;
        if version < 1 {
            conn.execute_batch(MIGRATION_0001)?;
            conn.pragma_update(None, "user_version", 1)?;
        }
        Ok(Self { conn })
    }

    /// Open the default database. `KESTRL_HISTORY_DB` overrides the path.
    pub fn open_default() -> Result<Self> {
        let path = match std::env::var_os("KESTRL_HISTORY_DB") {
            Some(path) => PathBuf::from(path),
            None => ProjectDirs::from("", "", "kestrl")
                .ok_or(Error::NoDataDir)?
                .data_dir()
                .join("history.db"),
        };
        Self::open(&path)
    }
}

pub(crate) fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_millis()).unwrap_or(i64::MAX))
}
