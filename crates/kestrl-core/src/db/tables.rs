//! Listing PostgreSQL tables and describing their columns.

use serde::Serialize;

use super::{ConnectionInfo, QueryOptions, query, query_params};
use crate::Result;
use crate::secrets::Redactor;

#[derive(Debug, Serialize)]
pub struct TableInfo {
    pub schema: String,
    pub name: String,
    pub approx_rows: i64,
}

#[derive(Debug, Serialize)]
pub struct ColumnInfo {
    pub name: String,
    pub data_type: String,
    pub is_nullable: bool,
    pub default: Option<String>,
}

pub fn list_tables(connection: &ConnectionInfo, redactor: &Redactor) -> Result<Vec<TableInfo>> {
    let result = query(
        connection,
        "SELECT schemaname::text, relname::text, COALESCE(n_live_tup, 0)::bigint AS approx_rows FROM pg_stat_user_tables ORDER BY schemaname, relname",
        QueryOptions {
            is_read_only: connection.is_read_only,
        },
        redactor,
    )?;
    result
        .rows
        .iter()
        .map(|row| {
            Ok(TableInfo {
                schema: row[0].as_str().unwrap_or_default().to_owned(),
                name: row[1].as_str().unwrap_or_default().to_owned(),
                approx_rows: row[2].as_i64().unwrap_or_default(),
            })
        })
        .collect()
}

pub fn describe_table(
    connection: &ConnectionInfo,
    schema: &str,
    table: &str,
    redactor: &Redactor,
) -> Result<Vec<ColumnInfo>> {
    let schema_filter = schema.to_owned();
    let table_filter = table.to_owned();
    let params: [&(dyn postgres::types::ToSql + Sync); 2] = [&schema_filter, &table_filter];
    let result = query_params(
        connection,
        "SELECT column_name, data_type, is_nullable = 'YES', column_default FROM information_schema.columns WHERE table_schema = $1 AND table_name = $2 ORDER BY ordinal_position",
        &params,
        QueryOptions {
            is_read_only: connection.is_read_only,
        },
        redactor,
    )?;
    result
        .rows
        .iter()
        .map(|row| {
            Ok(ColumnInfo {
                name: row[0].as_str().unwrap_or_default().to_owned(),
                data_type: row[1].as_str().unwrap_or_default().to_owned(),
                is_nullable: row[2].as_bool().unwrap_or(false),
                default: row[3].as_str().map(str::to_owned),
            })
        })
        .collect()
}
