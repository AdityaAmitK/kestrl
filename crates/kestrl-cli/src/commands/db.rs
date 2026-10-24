//! `kestrl db`: query databases and browse tables.

use std::collections::BTreeMap;
use std::io::{self, Write};

use anyhow::Context as _;
use clap::{Args, Subcommand};
use kestrl_core::db::{ConnectionConfig, ConnectionInfo, QueryOptions, QueryResult};
use kestrl_core::env::Environment;
use kestrl_core::history::{History, NewDbEvent};
use kestrl_core::secrets::Redactor;
use kestrl_core::vars::Variables;
use kestrl_core::workspace::{self, WorkspaceConfig};

use crate::context::Context;

#[derive(Debug, Args)]
pub struct DbArgs {
    #[command(subcommand)]
    pub command: DbCommand,
}

#[derive(Debug, Subcommand)]
pub enum DbCommand {
    /// Run a SQL query
    Query {
        connection: String,
        sql: String,
        #[arg(long, short)]
        env: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// List or describe tables
    Tables {
        connection: String,
        #[arg(long, short)]
        env: Option<String>,
        table: Option<String>,
    },
    /// List connection names
    Connections,
}

pub fn run(ctx: &Context, args: DbArgs) -> anyhow::Result<()> {
    match args.command {
        DbCommand::Connections => {
            let root = find_workspace()?;
            let mut out = io::stdout().lock();
            for name in ConnectionConfig::names(&root)? {
                writeln!(out, "{name}")?;
            }
            Ok(())
        }
        DbCommand::Query {
            connection,
            sql,
            env,
            json,
        } => query_command(ctx, &connection, &sql, env.as_deref(), json),
        DbCommand::Tables {
            connection,
            env,
            table,
        } => tables_command(ctx, &connection, env.as_deref(), table.as_deref()),
    }
}

fn query_command(
    ctx: &Context,
    connection: &str,
    sql: &str,
    env: Option<&str>,
    json: bool,
) -> anyhow::Result<()> {
    let (root, info, redactor) = resolve(connection, env)?;
    let started = std::time::Instant::now();
    let result = kestrl_core::db::query(
        &info,
        sql,
        QueryOptions {
            is_read_only: info.is_read_only,
        },
        &redactor,
    );
    let elapsed = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
    let history_result = record(
        ctx,
        &root,
        &info,
        &redactor.redact(sql),
        result.as_ref().ok().map(|r| r.row_count),
        elapsed,
        result.as_ref().err().map(|e| e.to_string()),
    );
    if let Err(error) = history_result {
        eprintln!("warning: could not record database query: {error}");
    }
    let mut result = result?;
    redact_result(&mut result, &redactor);
    if ctx.is_json || json {
        let output = serde_json::json!({
            "columns": result.columns,
            "rows": result.rows,
            "row_count": result.row_count,
            "ms": result.duration_ms,
        });
        writeln!(
            io::stdout().lock(),
            "{}",
            serde_json::to_string_pretty(&output)?
        )?;
    } else {
        print_table(&result)?;
    }
    Ok(())
}

fn tables_command(
    ctx: &Context,
    connection: &str,
    env: Option<&str>,
    table: Option<&str>,
) -> anyhow::Result<()> {
    let (_, info, redactor) = resolve(connection, env)?;
    if let Some(table) = table {
        let (schema, name) = table.split_once('.').unwrap_or(("public", table));
        let mut columns = kestrl_core::db::describe_table(&info, schema, name, &redactor)?;
        for column in &mut columns {
            column.name = redactor.redact(&column.name);
            column.data_type = redactor.redact(&column.data_type);
            column.default = column.default.as_ref().map(|value| redactor.redact(value));
        }
        if ctx.is_json {
            writeln!(
                io::stdout().lock(),
                "{}",
                serde_json::to_string_pretty(&columns)?
            )?;
        } else {
            for column in columns {
                writeln!(
                    io::stdout().lock(),
                    "{}  {}  {}  {}",
                    column.name,
                    column.data_type,
                    if column.is_nullable {
                        "nullable"
                    } else {
                        "not null"
                    },
                    column.default.unwrap_or_default()
                )?;
            }
        }
    } else {
        let mut tables = kestrl_core::db::list_tables(&info, &redactor)?;
        for table in &mut tables {
            table.schema = redactor.redact(&table.schema);
            table.name = redactor.redact(&table.name);
        }
        if ctx.is_json {
            writeln!(
                io::stdout().lock(),
                "{}",
                serde_json::to_string_pretty(&tables)?
            )?;
        } else {
            for table in tables {
                writeln!(
                    io::stdout().lock(),
                    "{}.{}  {} rows",
                    table.schema,
                    table.name,
                    table.approx_rows
                )?;
            }
        }
    }
    Ok(())
}

fn resolve(
    connection: &str,
    env: Option<&str>,
) -> anyhow::Result<(std::path::PathBuf, ConnectionInfo, Redactor)> {
    let root = find_workspace()?;
    let config = WorkspaceConfig::load(&root)?;
    let env = env.or(config.default_env.as_deref()).unwrap_or("local");
    let environment = match Environment::load(&root, env) {
        Ok(value) => value.vars,
        Err(kestrl_core::Error::Read { source, .. })
            if source.kind() == std::io::ErrorKind::NotFound =>
        {
            BTreeMap::new()
        }
        Err(error) => return Err(error.into()),
    };
    let mut variables = Variables {
        environment,
        workspace: config.vars,
        redactor: Redactor::default(),
        ..Variables::default()
    };
    let config = ConnectionConfig::load(&root, connection)?;
    let info = config.resolve(connection, env, &mut variables)?;
    Ok((root, info, variables.redactor))
}

fn find_workspace() -> anyhow::Result<std::path::PathBuf> {
    let cwd = std::env::current_dir()?;
    workspace::find_root(&cwd).context("could not find a Kestrl workspace")
}

fn record(
    ctx: &Context,
    root: &std::path::Path,
    info: &ConnectionInfo,
    sql: &str,
    rows: Option<u64>,
    duration_ms: u64,
    error: Option<String>,
) -> kestrl_core::Result<String> {
    let mut history = History::open_default()?;
    history.record_db(&NewDbEvent {
        source: "cli".to_owned(),
        actor: ctx.actor.as_str().to_owned(),
        workspace: Some(root.to_string_lossy().into_owned()),
        env: Some(info.env.clone()),
        connection: info.name.clone(),
        sql: sql.to_owned(),
        rows,
        duration_ms,
        error,
    })
}

fn print_table(result: &QueryResult) -> anyhow::Result<()> {
    if result.columns.is_empty() {
        writeln!(
            io::stdout().lock(),
            "{} rows ({}ms)",
            result.row_count,
            result.duration_ms
        )?;
        return Ok(());
    }
    let widths = result
        .columns
        .iter()
        .enumerate()
        .map(|(index, name)| {
            result
                .rows
                .iter()
                .map(|row| format_cell(row.get(index)))
                .fold(name.chars().count().min(60), |width, cell| {
                    width.max(cell.chars().count().min(60))
                })
        })
        .collect::<Vec<_>>();
    let mut out = io::stdout().lock();
    for (index, name) in result.columns.iter().enumerate() {
        write!(
            out,
            "{:<width$}  ",
            truncate(name, widths[index]),
            width = widths[index]
        )?;
    }
    writeln!(out)?;
    for width in &widths {
        write!(out, "{}  ", "-".repeat(*width))?;
    }
    writeln!(out)?;
    for row in &result.rows {
        for (index, value) in row.iter().enumerate() {
            write!(
                out,
                "{:<width$}  ",
                truncate(&format_cell(Some(value)), widths[index]),
                width = widths[index]
            )?;
        }
        writeln!(out)?;
    }
    writeln!(out, "{} rows ({}ms)", result.row_count, result.duration_ms)?;
    Ok(())
}

fn format_cell(value: Option<&serde_json::Value>) -> String {
    value.map_or_else(
        || "NULL".to_owned(),
        |value| match value {
            serde_json::Value::String(text) => text.clone(),
            _ => value.to_string(),
        },
    )
}

fn truncate(value: &str, max: usize) -> String {
    if value.chars().count() <= max {
        return value.to_owned();
    }
    let mut result = value
        .chars()
        .take(max.saturating_sub(1))
        .collect::<String>();
    result.push('…');
    result
}

fn redact_result(result: &mut QueryResult, redactor: &Redactor) {
    for column in &mut result.columns {
        *column = redactor.redact(column);
    }
    for row in &mut result.rows {
        for value in row {
            if let serde_json::Value::String(text) = value {
                *text = redactor.redact(text);
            }
        }
    }
}
