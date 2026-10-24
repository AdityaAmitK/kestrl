//! History store: SQLite at `~/Library/Application Support/kestrl/history.db`.

use std::path::Path;

use rusqlite::Connection;

use crate::Result;

const MIGRATION_0001: &str = include_str!("../migrations/0001_init.sql");

pub struct History {
    #[expect(dead_code, reason = "used by history queries in phase 1")]
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
}
