//! Database connection and query functions for the app.

use std::path::Path;

use kestrl_core::db::run::DbQueryMeta;
use kestrl_core::db::{self, ConnectionConfig};

use crate::error::KestrlError;
use crate::types::KeyValue;

#[derive(Debug, uniffi::Record)]
pub struct QueryResultRecord {
    pub columns: Vec<String>,
    /// Cells are JSON-encoded so the app can render and copy their exact values.
    pub rows: Vec<Vec<String>>,
    pub row_count: u64,
    pub duration_ms: u64,
    pub is_read_only: bool,
}

#[derive(Debug, uniffi::Record)]
pub struct TableInfo {
    pub schema: String,
    pub name: String,
    pub approx_rows: i64,
}

#[derive(Debug, uniffi::Record)]
pub struct ColumnInfo {
    pub name: String,
    pub data_type: String,
    pub is_nullable: bool,
    pub default: Option<String>,
}

#[uniffi::export]
pub fn db_connections(workspace_root: String) -> Result<Vec<String>, KestrlError> {
    Ok(ConnectionConfig::names(Path::new(&workspace_root))?)
}

#[uniffi::export]
pub fn db_query(
    workspace_root: String,
    connection: String,
    env: Option<String>,
    sql: String,
) -> Result<QueryResultRecord, KestrlError> {
    let run = db::run_query(
        Path::new(&workspace_root),
        &connection,
        env.as_deref(),
        &sql,
        DbQueryMeta {
            source: "app".to_owned(),
            actor: "me".to_owned(),
        },
    )?;
    let is_read_only = run.is_read_only;
    let result = run.outcome?;
    let rows = result
        .rows
        .into_iter()
        .map(|row| {
            row.into_iter()
                .map(|value| {
                    serde_json::to_string(&value).map_err(|error| KestrlError::History {
                        message: error.to_string(),
                    })
                })
                .collect()
        })
        .collect::<Result<Vec<Vec<String>>, KestrlError>>()?;
    Ok(QueryResultRecord {
        columns: result.columns,
        rows,
        row_count: result.row_count,
        duration_ms: result.duration_ms,
        is_read_only,
    })
}

#[uniffi::export]
pub fn db_tables(
    workspace_root: String,
    connection: String,
    env: Option<String>,
) -> Result<Vec<TableInfo>, KestrlError> {
    Ok(
        db::run::tables(Path::new(&workspace_root), &connection, env.as_deref())?
            .into_iter()
            .map(|table| TableInfo {
                schema: table.schema,
                name: table.name,
                approx_rows: table.approx_rows,
            })
            .collect(),
    )
}

#[uniffi::export]
pub fn db_describe(
    workspace_root: String,
    connection: String,
    env: Option<String>,
    table: String,
) -> Result<Vec<ColumnInfo>, KestrlError> {
    Ok(db::run::describe(
        Path::new(&workspace_root),
        &connection,
        env.as_deref(),
        &table,
    )?
    .into_iter()
    .map(|column| ColumnInfo {
        name: column.name,
        data_type: column.data_type,
        is_nullable: column.is_nullable,
        default: column.default,
    })
    .collect())
}

/// A connection as written in its file; URLs stay unresolved.
#[derive(Debug, Clone, uniffi::Record)]
pub struct ConnectionRecord {
    pub kind: String,
    /// One URL per environment: key is the environment name
    pub urls: Vec<KeyValue>,
    pub allow_writes: Vec<String>,
}

/// Connection kinds the app can offer.
#[uniffi::export]
pub fn connection_kinds() -> Vec<String> {
    kestrl_core::connection_files::SUPPORTED_KINDS
        .iter()
        .map(|kind| (*kind).to_owned())
        .collect()
}

#[uniffi::export]
pub fn read_connection(
    workspace_root: String,
    name: String,
) -> Result<ConnectionRecord, KestrlError> {
    let spec = kestrl_core::connection_files::read(Path::new(&workspace_root), &name)?;
    Ok(ConnectionRecord {
        kind: spec.kind,
        urls: KeyValue::from_pairs(spec.urls),
        allow_writes: spec.allow_writes,
    })
}

/// Create (`previous` = None) or update a connection; a different `previous` renames it.
#[uniffi::export]
pub fn save_connection(
    workspace_root: String,
    name: String,
    previous: Option<String>,
    connection: ConnectionRecord,
) -> Result<(), KestrlError> {
    let spec = kestrl_core::connection_files::ConnectionSpec {
        kind: connection.kind,
        urls: connection
            .urls
            .into_iter()
            .map(|pair| (pair.key, pair.value))
            .collect(),
        allow_writes: connection.allow_writes,
    };
    Ok(kestrl_core::connection_files::save(
        Path::new(&workspace_root),
        &name,
        previous.as_deref(),
        &spec,
    )?)
}

#[uniffi::export]
pub fn delete_connection(workspace_root: String, name: String) -> Result<(), KestrlError> {
    Ok(kestrl_core::connection_files::delete(
        Path::new(&workspace_root),
        &name,
    )?)
}
