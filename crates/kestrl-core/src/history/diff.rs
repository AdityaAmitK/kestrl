//! Comparing two stored HTTP exchanges: status, duration, response headers and body.

use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;
use serde_json::Value;

use super::HttpExchange;

/// Above this many table cells the line diff falls back to remove-all, add-all.
const MAX_LCS_CELLS: usize = 4_000_000;

/// What changed from exchange `a` to exchange `b`.
#[derive(Debug, Serialize)]
pub struct EventDiff {
    pub status: (Option<u16>, Option<u16>),
    pub duration_ms: (u64, u64),
    pub headers: HeaderDiff,
    pub body: BodyDiff,
}

#[derive(Debug, Default, Serialize)]
pub struct HeaderDiff {
    /// Name and value present only in `b`
    pub added: Vec<(String, String)>,
    /// Name and value present only in `a`
    pub removed: Vec<(String, String)>,
    pub changed: Vec<HeaderChange>,
}

#[derive(Debug, Serialize)]
pub struct HeaderChange {
    pub name: String,
    pub from: String,
    pub to: String,
}

#[derive(Debug, Serialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum BodyDiff {
    Identical,
    /// Both bodies are JSON: the paths that differ
    Json {
        changes: Vec<JsonChange>,
    },
    /// Otherwise: the lines that differ
    Lines {
        ops: Vec<LineOp>,
    },
}

/// One JSON path that differs. `None` means the path is missing on that side.
#[derive(Debug, Serialize)]
pub struct JsonChange {
    pub path: String,
    pub from: Option<Value>,
    pub to: Option<Value>,
}

#[derive(Debug, Serialize)]
#[serde(tag = "op", rename_all = "lowercase")]
pub enum LineOp {
    Added { text: String },
    Removed { text: String },
}

impl HeaderDiff {
    pub fn is_empty(&self) -> bool {
        self.added.is_empty() && self.removed.is_empty() && self.changed.is_empty()
    }
}

/// Compare `a` (the older or left side) with `b`.
pub fn diff_exchanges(a: &HttpExchange, b: &HttpExchange) -> EventDiff {
    EventDiff {
        status: (a.status, b.status),
        duration_ms: (a.duration_ms, b.duration_ms),
        headers: diff_headers(&a.res_headers, &b.res_headers),
        body: diff_bodies(
            a.res_body.as_deref().unwrap_or_default(),
            b.res_body.as_deref().unwrap_or_default(),
        ),
    }
}

fn diff_headers(a: &[(String, String)], b: &[(String, String)]) -> HeaderDiff {
    let a = join_headers(a);
    let b = join_headers(b);
    let names: BTreeSet<&str> = a.keys().chain(b.keys()).copied().collect();
    let mut diff = HeaderDiff::default();
    for name in names {
        match (a.get(name), b.get(name)) {
            (None, Some(value)) => diff.added.push((name.to_owned(), value.clone())),
            (Some(value), None) => diff.removed.push((name.to_owned(), value.clone())),
            (Some(from), Some(to)) if from != to => diff.changed.push(HeaderChange {
                name: name.to_owned(),
                from: from.clone(),
                to: to.clone(),
            }),
            _ => {}
        }
    }
    diff
}

/// Header names are already lowercase; repeated headers are joined with `, `.
fn join_headers(headers: &[(String, String)]) -> BTreeMap<&str, String> {
    let mut joined: BTreeMap<&str, String> = BTreeMap::new();
    for (name, value) in headers {
        joined
            .entry(name.as_str())
            .and_modify(|all| {
                all.push_str(", ");
                all.push_str(value);
            })
            .or_insert_with(|| value.clone());
    }
    joined
}

fn diff_bodies(a: &str, b: &str) -> BodyDiff {
    if a == b {
        return BodyDiff::Identical;
    }
    if let (Ok(a_json), Ok(b_json)) = (
        serde_json::from_str::<Value>(a),
        serde_json::from_str::<Value>(b),
    ) {
        let mut changes = Vec::new();
        json_changes("$", Some(&a_json), Some(&b_json), &mut changes);
        return if changes.is_empty() {
            BodyDiff::Identical
        } else {
            BodyDiff::Json { changes }
        };
    }
    let ops = line_ops(
        &a.lines().collect::<Vec<_>>(),
        &b.lines().collect::<Vec<_>>(),
    );
    if ops.is_empty() {
        BodyDiff::Identical
    } else {
        BodyDiff::Lines { ops }
    }
}

fn json_changes(path: &str, a: Option<&Value>, b: Option<&Value>, out: &mut Vec<JsonChange>) {
    match (a, b) {
        (Some(Value::Object(a)), Some(Value::Object(b))) => {
            let keys: BTreeSet<&String> = a.keys().chain(b.keys()).collect();
            for key in keys {
                json_changes(&format!("{path}.{key}"), a.get(key), b.get(key), out);
            }
        }
        (Some(Value::Array(a)), Some(Value::Array(b))) => {
            for index in 0..a.len().max(b.len()) {
                json_changes(&format!("{path}[{index}]"), a.get(index), b.get(index), out);
            }
        }
        (a, b) if a == b => {}
        (a, b) => out.push(JsonChange {
            path: path.to_owned(),
            from: a.cloned(),
            to: b.cloned(),
        }),
    }
}

/// Only the changed lines, found by longest common subsequence after trimming
/// the common head and tail.
fn line_ops(a: &[&str], b: &[&str]) -> Vec<LineOp> {
    let head = a.iter().zip(b).take_while(|(x, y)| x == y).count();
    let tail = a[head..]
        .iter()
        .rev()
        .zip(b[head..].iter().rev())
        .take_while(|(x, y)| x == y)
        .count();
    let a = &a[head..a.len() - tail];
    let b = &b[head..b.len() - tail];
    let (n, m) = (a.len(), b.len());

    if (n + 1).saturating_mul(m + 1) > MAX_LCS_CELLS {
        return a
            .iter()
            .map(|text| LineOp::Removed {
                text: (*text).to_owned(),
            })
            .chain(b.iter().map(|text| LineOp::Added {
                text: (*text).to_owned(),
            }))
            .collect();
    }

    let mut lcs = vec![vec![0u32; m + 1]; n + 1];
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            lcs[i][j] = if a[i] == b[j] {
                lcs[i + 1][j + 1] + 1
            } else {
                lcs[i + 1][j].max(lcs[i][j + 1])
            };
        }
    }

    let mut ops = Vec::new();
    let (mut i, mut j) = (0, 0);
    while i < n || j < m {
        if i < n && j < m && a[i] == b[j] {
            i += 1;
            j += 1;
        } else if j < m && (i == n || lcs[i][j + 1] >= lcs[i + 1][j]) {
            ops.push(LineOp::Added {
                text: b[j].to_owned(),
            });
            j += 1;
        } else {
            ops.push(LineOp::Removed {
                text: a[i].to_owned(),
            });
            i += 1;
        }
    }
    ops
}
