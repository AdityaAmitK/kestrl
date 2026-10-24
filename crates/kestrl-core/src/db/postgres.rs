//! Running SQL against PostgreSQL and converting result values to JSON.
//!
//! Unsupported PostgreSQL types are returned as `<type_name>` strings rather
//! than interpreting their binary representation as text.

use std::str::FromStr;
use std::time::Instant;

use postgres::types::{FromSql, Type};
use serde::Serialize;
use serde_json::Value;

use super::ConnectionInfo;
use super::numeric::{Numeric, numeric_string};
use crate::secrets::Redactor;
use crate::{Error, Result};

#[derive(Debug, Clone, Copy)]
pub struct QueryOptions {
    pub is_read_only: bool,
}

#[derive(Debug, Serialize)]
pub struct QueryResult {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<Value>>,
    pub row_count: u64,
    pub duration_ms: u64,
}

pub fn query(
    connection: &ConnectionInfo,
    sql: &str,
    options: QueryOptions,
    redactor: &Redactor,
) -> Result<QueryResult> {
    query_params(connection, sql, &[], options, redactor)
}

pub fn query_params(
    connection: &ConnectionInfo,
    sql: &str,
    params: &[&(dyn postgres::types::ToSql + Sync)],
    options: QueryOptions,
    redactor: &Redactor,
) -> Result<QueryResult> {
    let started = Instant::now();
    let mut client = connect(connection, redactor)?;
    if options.is_read_only {
        client
            .batch_execute("BEGIN READ ONLY")
            .map_err(|error| database_error(error, connection, redactor))?;
    }
    let result: Result<(Vec<String>, Vec<Vec<Value>>, u64)> = (|| {
        let statement = client
            .prepare(sql)
            .map_err(|error| database_error(error, connection, redactor))?;
        let columns = statement
            .columns()
            .iter()
            .map(|column| column.name().to_owned())
            .collect::<Vec<_>>();
        if statement.columns().is_empty() {
            let row_count = client
                .execute(&statement, params)
                .map_err(|error| database_error(error, connection, redactor))?;
            Ok((columns, Vec::new(), row_count))
        } else {
            let rows = client
                .query(&statement, params)
                .map_err(|error| database_error(error, connection, redactor))?;
            let row_count = u64::try_from(rows.len()).unwrap_or(u64::MAX);
            let values = rows
                .iter()
                .map(|row| {
                    row.columns()
                        .iter()
                        .enumerate()
                        .map(|(index, column)| value_at(row, index, column.type_()))
                        .collect::<Result<Vec<_>>>()
                })
                .collect::<Result<Vec<_>>>()?;
            Ok((columns, values, row_count))
        }
    })();
    let rollback = if options.is_read_only {
        client
            .batch_execute("ROLLBACK")
            .map_err(|error| database_error(error, connection, redactor))
    } else {
        Ok(())
    };
    let (columns, rows, row_count) = result?;
    rollback?;
    Ok(QueryResult {
        columns,
        rows,
        row_count,
        duration_ms: u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
    })
}

fn connect(connection: &ConnectionInfo, redactor: &Redactor) -> Result<postgres::Client> {
    let config = postgres::Config::from_str(&connection.url)
        .map_err(|error| Error::Database(redactor.redact(&error.to_string())))?;
    if connection.url.split_once('?').is_some_and(|(_, query)| {
        query
            .split('&')
            .any(|pair| pair.eq_ignore_ascii_case("sslmode=require"))
    }) {
        let tls = native_tls::TlsConnector::new()
            .map_err(|error| Error::Database(redactor.redact(&error.to_string())))?;
        let connector = postgres_native_tls::MakeTlsConnector::new(tls);
        config
            .connect(connector)
            .map_err(|error| database_error(error, connection, redactor))
    } else {
        config
            .connect(postgres::NoTls)
            .map_err(|error| database_error(error, connection, redactor))
    }
}

fn value_at(row: &postgres::Row, index: usize, kind: &Type) -> Result<Value> {
    if *kind == Type::BOOL {
        return get(row, index, Value::Bool);
    }
    if *kind == Type::INT2 {
        return get(row, index, |v: i16| Value::Number(i64::from(v).into()));
    }
    if *kind == Type::INT4 {
        return get(row, index, |v: i32| Value::Number(i64::from(v).into()));
    }
    if *kind == Type::INT8 {
        return get(row, index, |v: i64| Value::Number(v.into()));
    }
    if *kind == Type::FLOAT4 {
        return get(row, index, |v: f32| {
            serde_json::Number::from_f64(f64::from(v)).map_or(Value::Null, Value::Number)
        });
    }
    if *kind == Type::FLOAT8 {
        return get(row, index, |v: f64| {
            serde_json::Number::from_f64(v).map_or(Value::Null, Value::Number)
        });
    }
    if *kind == Type::BYTEA {
        return get(row, index, |v: Vec<u8>| {
            Value::String(v.iter().map(|b| format!("{b:02x}")).collect())
        });
    }
    if *kind == Type::DATE {
        return get::<time::Date>(row, index, |v| Value::String(v.to_string()));
    }
    if *kind == Type::TIME {
        return get::<time::Time>(row, index, |v| Value::String(v.to_string()));
    }
    if *kind == Type::TIMESTAMP {
        return get::<time::PrimitiveDateTime>(row, index, |v| Value::String(v.to_string()));
    }
    if *kind == Type::TIMESTAMPTZ {
        return get::<time::OffsetDateTime>(row, index, |v| Value::String(v.to_string()));
    }
    if *kind == Type::UUID {
        return get::<uuid::Uuid>(row, index, |v| Value::String(v.to_string()));
    }
    if *kind == Type::JSON || *kind == Type::JSONB {
        return get::<postgres::types::Json<Value>>(row, index, |v| v.0);
    }
    if *kind == Type::NUMERIC {
        return get::<Numeric>(row, index, |v| Value::String(numeric_string(&v.0)));
    }
    // Unknown PostgreSQL binary formats cannot safely be interpreted as UTF-8.
    get::<String>(row, index, Value::String)
        .or_else(|_| Ok(Value::String(format!("<{}>", kind.name()))))
}

fn get<T>(row: &postgres::Row, index: usize, map: impl Fn(T) -> Value) -> Result<Value>
where
    for<'a> T: FromSql<'a>,
{
    row.try_get::<_, Option<T>>(index)
        .map(|value| value.map_or(Value::Null, map))
        .map_err(|error| Error::Database(error.to_string()))
}

fn database_error(
    error: postgres::Error,
    connection: &ConnectionInfo,
    redactor: &Redactor,
) -> Error {
    if error.code().is_some_and(|code| code.code() == "25006") {
        Error::ReadOnlyConnection {
            connection: connection.name.clone(),
            env: connection.env.clone(),
        }
    } else {
        Error::Database(redactor.redact(&error.to_string()))
    }
}
