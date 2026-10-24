//! Bounded in-memory log retention for one service.

use std::collections::VecDeque;

use serde::Deserialize;
use serde::Serialize;

use crate::history::now_ms;

/// Output stream that produced a log line.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Stream {
    /// Standard output.
    Stdout,
    /// Standard error.
    Stderr,
}

/// One line captured from a service process.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LogLine {
    /// Monotonically increasing sequence number within this service run.
    pub seq: u64,
    /// Unix timestamp in milliseconds.
    pub ts_ms: i64,
    /// Child output stream.
    pub stream: Stream,
    /// Text captured for this line.
    pub text: String,
}

/// Bounded in-memory log buffer for one service.
#[derive(Default)]
pub struct Buffer {
    lines: VecDeque<LogLine>,
    next: u64,
}
impl Buffer {
    /// Add a line, retaining only the newest 10,000 lines.
    pub fn push(&mut self, stream: Stream, text: String) {
        self.next += 1;
        self.lines.push_back(LogLine {
            seq: self.next,
            ts_ms: now_ms(),
            stream,
            text,
        });
        if self.lines.len() > 10_000 {
            self.lines.pop_front();
        }
    }
    /// Return matching lines, retaining the newest `limit` matches.
    pub fn query(
        &self,
        since_ms: Option<i64>,
        grep: Option<&regex::Regex>,
        limit: Option<usize>,
    ) -> Vec<LogLine> {
        self.lines
            .iter()
            .filter(|line| since_ms.is_none_or(|since| line.ts_ms >= since))
            .filter(|line| grep.is_none_or(|pattern| pattern.is_match(&line.text)))
            .rev()
            .take(limit.unwrap_or(usize::MAX))
            .cloned()
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect()
    }
    /// Return the newest assigned sequence number.
    pub fn latest(&self) -> u64 {
        self.next
    }
    /// Return lines newer than `seq`.
    pub fn after(&self, seq: u64) -> Vec<LogLine> {
        self.lines
            .iter()
            .filter(|line| line.seq > seq)
            .cloned()
            .collect()
    }
}
