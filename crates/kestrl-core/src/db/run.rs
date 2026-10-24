//! Resolving and recording database queries for CLI and app callers.

use std::path::Path;
use std::time::Instant;

use super::{ConnectionConfig, ConnectionInfo, QueryOptions, QueryResult, query};
use crate::Result;
use crate::history::{EventMeta, History, NewDbEvent, RetentionOptions};
use crate::secrets::Redactor;
use crate::workspace::{self, WorkspaceConfig};

/// History attribution for a database query.
#[derive(Debug, Clone)]
pub struct DbQueryMeta {
    pub source: String,
    pub actor: String,
}

/// Resolve a connection and its secret redactor for a workspace.
pub fn resolve_connection(
    root: &Path,
    connection: &str,
    env: Option<&str>,
) -> Result<(ConnectionInfo, Redactor)> {
    let (env, mut variables) = workspace::load_variables(root, env)?;
    let config = ConnectionConfig::load(root, connection)?;
    let info = config.resolve(connection, &env, &mut variables)?;
    let mut redactor = crate::secrets::workspace_redactor(root)?;
    redactor.track("connection", &info.url);
    Ok((info, redactor))
}

/// The outcome of one recorded query.
#[derive(Debug)]
pub struct QueryRun {
    /// Redacted results, or why the query failed. Failed queries are recorded too.
    pub outcome: Result<QueryResult>,
    /// Whether the query ran in a read-only transaction.
    pub is_read_only: bool,
    /// Why recording to history failed, if it did; the query itself is unaffected.
    pub history_warning: Option<String>,
}

/// Run a database query, record it in history, and return redacted results.
pub fn run_query(
    root: &Path,
    connection: &str,
    env: Option<&str>,
    sql: &str,
    meta: DbQueryMeta,
) -> Result<QueryRun> {
    let workspace_config = WorkspaceConfig::load(root)?;
    let (info, redactor) = resolve_connection(root, connection, env)?;
    let started = Instant::now();
    let result = query(
        &info,
        sql,
        QueryOptions {
            is_read_only: info.is_read_only,
        },
        &redactor,
    );
    let elapsed = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
    let event = NewDbEvent {
        meta: EventMeta {
            source: meta.source,
            actor: meta.actor,
            session_id: None,
            workspace: Some(root.to_string_lossy().into_owned()),
            env: Some(info.env.clone()),
            request_ref: None,
        },
        connection: info.name.clone(),
        sql: redactor.redact(sql),
        rows: result.as_ref().ok().map(|result| result.row_count),
        duration_ms: elapsed,
        error: result
            .as_ref()
            .err()
            .map(|error| redactor.redact(&error.to_string())),
    };
    let history_warning = record(event, &workspace_config.retention)
        .err()
        .map(|error| error.to_string());
    let outcome = result.map(|mut result| {
        for row in &mut result.rows {
            for value in row {
                if let serde_json::Value::String(text) = value {
                    *text = redactor.redact(text);
                }
            }
        }
        result
    });
    Ok(QueryRun {
        outcome,
        is_read_only: info.is_read_only,
        history_warning,
    })
}

fn record(mut event: NewDbEvent, retention: &RetentionOptions) -> Result<String> {
    let mut history = History::open_default()?;
    event.meta.session_id = history.current_session()?.map(|session| session.id);
    history.record_db(&event, retention)
}

/// List table metadata for a resolved database connection.
pub fn tables(root: &Path, connection: &str, env: Option<&str>) -> Result<Vec<super::TableInfo>> {
    let (info, redactor) = resolve_connection(root, connection, env)?;
    super::list_tables(&info, &redactor)
}

/// Describe a table using an optional schema-qualified name.
pub fn describe(
    root: &Path,
    connection: &str,
    env: Option<&str>,
    table: &str,
) -> Result<Vec<super::ColumnInfo>> {
    let (info, redactor) = resolve_connection(root, connection, env)?;
    let (schema, name) = table.split_once('.').unwrap_or(("public", table));
    super::describe_table(&info, schema, name, &redactor)
}
