//! Editing events: tags, notes and actor. Notes and tags make an event saved.

use rusqlite::params;

use super::History;
use crate::Result;

impl History {
    pub fn add_tag(&self, id_prefix: &str, tag: &str) -> Result<()> {
        let id = self.resolve_id(id_prefix)?;
        self.conn.execute(
            "INSERT OR IGNORE INTO tags (event_id, tag) VALUES (?1, ?2)",
            params![id, tag],
        )?;
        Ok(())
    }

    pub fn remove_tag(&self, id_prefix: &str, tag: &str) -> Result<()> {
        let id = self.resolve_id(id_prefix)?;
        self.conn.execute(
            "DELETE FROM tags WHERE event_id = ?1 AND tag = ?2",
            params![id, tag],
        )?;
        Ok(())
    }

    /// Set the note. An empty note does not make the event saved.
    pub fn set_note(&self, id_prefix: &str, note: &str) -> Result<()> {
        let id = self.resolve_id(id_prefix)?;
        let tx = self.conn.unchecked_transaction()?;
        tx.execute(
            "UPDATE events SET note = ?2 WHERE id = ?1",
            params![id, note],
        )?;
        tx.execute(
            "UPDATE events_fts SET note = ?2 WHERE event_id = ?1",
            params![id, note],
        )?;
        tx.commit()?;
        Ok(())
    }

    pub fn set_actor(&self, id_prefix: &str, actor: &str) -> Result<()> {
        let id = self.resolve_id(id_prefix)?;
        self.conn.execute(
            "UPDATE events SET actor = ?2 WHERE id = ?1",
            params![id, actor],
        )?;
        Ok(())
    }
}
