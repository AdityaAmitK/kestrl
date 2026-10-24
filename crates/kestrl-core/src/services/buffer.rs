//! Bounded in-memory log retention for one service.

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LogLine {
    pub seq: u64,
    pub ts_ms: u128,
    pub stream: String,
    pub text: String,
}
#[derive(Default)]
pub struct Buffer {
    lines: VecDeque<LogLine>,
    next: u64,
}
impl Buffer {
    pub fn push(&mut self, stream: &str, text: String) {
        self.next += 1;
        self.lines.push_back(LogLine {
            seq: self.next,
            ts_ms: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis(),
            stream: stream.into(),
            text,
        });
        if self.lines.len() > 10_000 {
            self.lines.pop_front();
        }
    }
    pub fn query(
        &self,
        since_ms: Option<u128>,
        grep: Option<&regex::Regex>,
        limit: Option<usize>,
    ) -> Vec<LogLine> {
        self.lines
            .iter()
            .filter(|l| since_ms.is_none_or(|s| l.ts_ms >= s))
            .filter(|l| grep.is_none_or(|r| r.is_match(&l.text)))
            .take(limit.unwrap_or(usize::MAX))
            .cloned()
            .collect()
    }
    pub fn latest(&self) -> u64 {
        self.next
    }
    pub fn after(&self, seq: u64) -> Vec<LogLine> {
        self.lines.iter().filter(|l| l.seq > seq).cloned().collect()
    }
}
