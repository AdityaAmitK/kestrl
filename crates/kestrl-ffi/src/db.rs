//! Database connection and query functions for the app.

use std::path::Path;

use kestrl_core::db::run::DbQueryMeta;
use kestrl_core::db::{self, ConnectionConfig};

use crate::error::KestrlError;

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
    let (result, is_read_only, warning) = db::run::run_query(
        Path::new(&workspace_root),
        &connection,
        env.as_deref(),
        &sql,
        DbQueryMeta {
            source: "app".to_owned(),
            actor: "me".to_owned(),
        },
    )?;
    let _ = warning;
    let result = result?;
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
