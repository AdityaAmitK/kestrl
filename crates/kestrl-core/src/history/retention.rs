//! Pruning unsaved history. Saved events (tagged or noted) are never pruned.

use rusqlite::params;
use serde::{Deserialize, Serialize};

use super::{History, now_ms};
use crate::Result;

const DAY_MS: i64 = 24 * 60 * 60 * 1000;

/// How much unsaved history to keep, and how much of each body to store.
/// Missing keys in `[retention]` take the defaults.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(default)]
pub struct RetentionOptions {
    pub max_age_days: u32,
    pub max_events: u32,
    /// Bodies are cut to this many bytes before storage.
    pub max_body_bytes: usize,
}

impl Default for RetentionOptions {
    fn default() -> Self {
        Self {
            max_age_days: 7,
            max_events: 500,
            max_body_bytes: 1024 * 1024,
        }
    }
}

impl History {
    /// Delete unsaved events older than the max age or beyond the newest
    /// `max_events`, and their search rows. Returns the number of events removed.
    pub(crate) fn prune(&self, options: &RetentionOptions) -> Result<usize> {
        let cutoff = now_ms() - i64::from(options.max_age_days) * DAY_MS;
        let tx = self.conn.unchecked_transaction()?;
        let removed = tx.execute(
            "DELETE FROM events
             WHERE (ts < ?1 OR id NOT IN (SELECT id FROM events ORDER BY ts DESC, id DESC LIMIT ?2))
               AND NOT EXISTS (SELECT 1 FROM tags WHERE tags.event_id = events.id)
               AND COALESCE(note, '') = ''",
            params![cutoff, i64::from(options.max_events)],
        )?;
        tx.execute(
            "DELETE FROM events_fts WHERE event_id NOT IN (SELECT id FROM events)",
            [],
        )?;
        tx.commit()?;
        Ok(removed)
    }
}
