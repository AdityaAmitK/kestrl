//! Running SQL against PostgreSQL and converting result values to JSON.

use std::time::Instant;

use postgres::types::Type;
use postgres_types::{FromSql, Json};
use serde::Serialize;
use serde_json::Value;

use super::ConnectionInfo;
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
    if options.is_read_only && is_write(sql) {
        return Err(Error::ReadOnlyConnection {
            connection: connection.name.clone(),
            env: connection.env.clone(),
        });
    }
    let started = Instant::now();
    let mut client = postgres::Client::connect(&connection.url, postgres::NoTls)
        .map_err(|error| database_error(error, redactor))?;
    if options.is_read_only {
        client
            .batch_execute("BEGIN READ ONLY")
            .map_err(|error| database_error(error, redactor))?;
    }
    let is_command = !options.is_read_only && is_write(sql);
    let result = if is_command {
        client
            .execute(sql, params)
            .map(|row_count| (Vec::new(), row_count))
            .map_err(|error| database_error(error, redactor))
    } else {
        client
            .query(sql, params)
            .map(|rows| {
                let count = u64::try_from(rows.len()).unwrap_or(u64::MAX);
                (rows, count)
            })
            .map_err(|error| database_error(error, redactor))
    };
    let rollback = if options.is_read_only {
        client
            .batch_execute("ROLLBACK")
            .map_err(|error| database_error(error, redactor))
    } else {
        Ok(())
    };
    let (rows, row_count) = result?;
    rollback?;
    let columns = rows
        .first()
        .map(|row| {
            row.columns()
                .iter()
                .map(|column| column.name().to_owned())
                .collect()
        })
        .unwrap_or_default();
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
    Ok(QueryResult {
        columns,
        row_count,
        rows: values,
        duration_ms: u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
    })
}

fn value_at(row: &postgres::Row, index: usize, kind: &Type) -> Result<Value> {
    if *kind == Type::BOOL {
        return row
            .try_get::<_, Option<bool>>(index)
            .map(|value| value.map_or(Value::Null, Value::Bool))
            .map_err(|error| Error::Database(error.to_string()));
    }
    if *kind == Type::INT2 {
        return row
            .try_get::<_, Option<i16>>(index)
            .map(|value| {
                value.map_or(Value::Null, |number| {
                    Value::Number(i64::from(number).into())
                })
            })
            .map_err(|error| Error::Database(error.to_string()));
    }
    if *kind == Type::INT4 {
        return row
            .try_get::<_, Option<i32>>(index)
            .map(|value| {
                value.map_or(Value::Null, |number| {
                    Value::Number(i64::from(number).into())
                })
            })
            .map_err(|error| Error::Database(error.to_string()));
    }
    if *kind == Type::INT8 {
        return row
            .try_get::<_, Option<i64>>(index)
            .map(|value| value.map_or(Value::Null, |number| Value::Number(number.into())))
            .map_err(|error| Error::Database(error.to_string()));
    }
    if *kind == Type::FLOAT4 || *kind == Type::FLOAT8 {
        if *kind == Type::FLOAT4 {
            return row
                .try_get::<_, Option<f32>>(index)
                .map(|value| {
                    value
                        .and_then(|number| serde_json::Number::from_f64(f64::from(number)))
                        .map_or(Value::Null, Value::Number)
                })
                .map_err(|error| Error::Database(error.to_string()));
        }
        return row
            .try_get::<_, Option<f64>>(index)
            .map(|value| {
                value
                    .and_then(serde_json::Number::from_f64)
                    .map_or(Value::Null, Value::Number)
            })
            .map_err(|error| Error::Database(error.to_string()));
    }
    if *kind == Type::BYTEA {
        return row
            .try_get::<_, Option<Vec<u8>>>(index)
            .map(|value| {
                value.map_or(Value::Null, |bytes| {
                    Value::String(bytes.iter().map(|byte| format!("{byte:02x}")).collect())
                })
            })
            .map_err(|error| Error::Database(error.to_string()));
    }
    if *kind == Type::JSON || *kind == Type::JSONB {
        return row
            .try_get::<_, Option<Json<Value>>>(index)
            .map(|value| value.map_or(Value::Null, |Json(value)| value))
            .map_err(|error| Error::Database(error.to_string()));
    }
    if *kind == Type::NUMERIC {
        return row
            .try_get::<_, Option<Numeric>>(index)
            .map(|value| {
                value.map_or(Value::Null, |Numeric(bytes)| {
                    Value::String(numeric_string(&bytes))
                })
            })
            .map_err(|error| Error::Database(error.to_string()));
    }
    if *kind == Type::UUID {
        return row
            .try_get::<_, Option<UuidBytes>>(index)
            .map(|value| {
                value.map_or(Value::Null, |UuidBytes(bytes)| {
                    let hex = bytes
                        .iter()
                        .map(|byte| format!("{byte:02x}"))
                        .collect::<String>();
                    Value::String(format!(
                        "{}-{}-{}-{}-{}",
                        &hex[..8],
                        &hex[8..12],
                        &hex[12..16],
                        &hex[16..20],
                        &hex[20..]
                    ))
                })
            })
            .map_err(|error| Error::Database(error.to_string()));
    }
    if *kind == Type::TIMESTAMPTZ || *kind == Type::TIMESTAMP {
        return row
            .try_get::<_, Option<i64>>(index)
            .map(|value| {
                value.map_or(Value::Null, |micros| {
                    let unix_micros = micros.saturating_add(946_684_800_000_000);
                    let timestamp = time::OffsetDateTime::from_unix_timestamp_nanos(
                        i128::from(unix_micros) * 1_000,
                    )
                    .unwrap_or(time::OffsetDateTime::UNIX_EPOCH);
                    Value::String(
                        timestamp
                            .format(&time::format_description::well_known::Rfc3339)
                            .unwrap_or_default(),
                    )
                })
            })
            .map_err(|error| Error::Database(error.to_string()));
    }
    let value = row
        .try_get::<_, Option<String>>(index)
        .map_err(|error| Error::Database(error.to_string()));
    match value {
        Ok(value) => Ok(value.map_or(Value::Null, Value::String)),
        Err(_) => row
            .try_get::<_, Option<Vec<u8>>>(index)
            .map(|value| {
                value.map_or(Value::Null, |bytes| {
                    Value::String(String::from_utf8_lossy(&bytes).into_owned())
                })
            })
            .map_err(|error| Error::Database(error.to_string())),
    }
}

struct Numeric(Vec<u8>);

struct UuidBytes(Vec<u8>);

impl<'a> FromSql<'a> for UuidBytes {
    fn from_sql(
        _kind: &Type,
        raw: &'a [u8],
    ) -> std::result::Result<Self, Box<dyn std::error::Error + Sync + Send>> {
        Ok(Self(raw.to_vec()))
    }

    fn accepts(kind: &Type) -> bool {
        *kind == Type::UUID
    }
}

impl<'a> FromSql<'a> for Numeric {
    fn from_sql(
        _kind: &Type,
        raw: &'a [u8],
    ) -> std::result::Result<Self, Box<dyn std::error::Error + Sync + Send>> {
        Ok(Self(raw.to_vec()))
    }

    fn accepts(kind: &Type) -> bool {
        *kind == Type::NUMERIC
    }
}

fn numeric_string(bytes: &[u8]) -> String {
    if bytes.len() < 8 {
        return String::new();
    }
    let read_i16 = |offset: usize| i16::from_be_bytes([bytes[offset], bytes[offset + 1]]);
    let count = read_i16(0).max(0) as usize;
    let weight = read_i16(2);
    let sign = read_i16(4);
    let scale = read_i16(6).max(0) as usize;
    let mut digits = Vec::new();
    for index in 0..count {
        let offset = 8 + index * 2;
        if offset + 2 > bytes.len() {
            break;
        }
        digits.push(u16::from_be_bytes([bytes[offset], bytes[offset + 1]]));
    }
    let integer_groups = usize::try_from(weight + 1).unwrap_or(0);
    let mut integer = String::new();
    for index in 0..integer_groups {
        let digit = digits.get(index).copied().unwrap_or(0);
        if index == 0 {
            integer.push_str(&digit.to_string());
        } else {
            integer.push_str(&format!("{digit:04}"));
        }
    }
    if integer.is_empty() {
        integer.push('0');
    }
    let fraction_groups = scale.div_ceil(4);
    let fraction = (0..fraction_groups)
        .map(|index| {
            format!(
                "{:04}",
                digits.get(integer_groups + index).copied().unwrap_or(0)
            )
        })
        .collect::<String>();
    let fraction = &fraction[..scale.min(fraction.len())];
    let sign = if sign == 0x4000 { "-" } else { "" };
    if scale == 0 {
        format!("{sign}{integer}")
    } else {
        format!("{sign}{integer}.{fraction}")
    }
}

fn is_write(sql: &str) -> bool {
    let statement = sql.trim_start().to_ascii_uppercase();
    !statement.starts_with("SELECT")
        && !statement.starts_with("SHOW")
        && !statement.starts_with("VALUES")
        && !statement.starts_with("EXPLAIN")
}

fn database_error(error: postgres::Error, redactor: &Redactor) -> Error {
    Error::Database(redactor.redact(&error.to_string()))
}
