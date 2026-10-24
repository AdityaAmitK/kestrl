//! `kestrl db`: query databases and browse tables.

use std::io::{self, Write};

use anyhow::Context as _;
use clap::{Args, Subcommand};
use kestrl_core::db::{ConnectionConfig, ConnectionInfo, QueryOptions, QueryResult};
use kestrl_core::history::{EventMeta, History, NewDbEvent, RetentionOptions};
use kestrl_core::secrets::Redactor;
use kestrl_core::workspace::{self, WorkspaceConfig};

use crate::context::Context;

#[derive(Debug, Args)]
pub struct DbArgs {
    /// Database operation to run.
    #[command(subcommand)]
    pub command: DbCommand,
}

#[derive(Debug, Subcommand)]
pub enum DbCommand {
    /// Run a SQL query
    Query {
        /// Connection name from `.kestrl/connections`.
        connection: String,
        /// SQL statement to run.
        sql: String,
        /// Environment to use.
        #[arg(long, short)]
        env: Option<String>,
    },
    /// List or describe tables
    Tables {
        /// Connection name from `.kestrl/connections`.
        connection: String,
        /// Environment to use.
        #[arg(long, short)]
        env: Option<String>,
        /// Optional table name, optionally prefixed by a schema.
        table: Option<String>,
    },
    /// List connection names
    Connections,
}

pub fn run(ctx: &Context, args: DbArgs) -> anyhow::Result<()> {
    match args.command {
        DbCommand::Connections => {
            let root = find_workspace()?;
            let names = ConnectionConfig::names(&root)?;
            if ctx.is_json {
                writeln!(
                    io::stdout().lock(),
                    "{}",
                    serde_json::to_string_pretty(&names)?
                )?;
            } else {
                let mut out = io::stdout().lock();
                for name in names {
                    writeln!(out, "{name}")?;
                }
            }
            Ok(())
        }
        DbCommand::Query {
            connection,
            sql,
            env,
        } => query_command(ctx, &connection, &sql, env.as_deref()),
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
) -> anyhow::Result<()> {
    let (root, info, redactor, retention) = resolve(connection, env)?;
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
    let mut event = NewDbEvent {
        meta: EventMeta {
            source: "cli".to_owned(),
            actor: ctx.actor.as_str().to_owned(),
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
    let history_result = record(&mut event, &retention);
    if let Err(error) = history_result {
        crate::commands::output::warn(&format!("could not record database query: {error}"));
    }
    let mut result = result?;
    redact_result(&mut result, &redactor);
    if ctx.is_json {
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
    let (_, info, redactor, _) = resolve(connection, env)?;
    if let Some(table) = table {
        let (schema, name) = table.split_once('.').unwrap_or(("public", table));
        let columns = kestrl_core::db::describe_table(&info, schema, name, &redactor)?;
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
        let tables = kestrl_core::db::list_tables(&info, &redactor)?;
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
) -> anyhow::Result<(
    std::path::PathBuf,
    ConnectionInfo,
    Redactor,
    RetentionOptions,
)> {
    let root = find_workspace()?;
    let workspace_config = WorkspaceConfig::load(&root)?;
    let (env, mut variables) = workspace::load_variables(&root, env)?;
    let config = ConnectionConfig::load(&root, connection)?;
    let info = config.resolve(connection, &env, &mut variables)?;
    let retention = workspace_config.retention;
    Ok((root, info, variables.redactor, retention))
}

fn find_workspace() -> anyhow::Result<std::path::PathBuf> {
    let cwd = std::env::current_dir()?;
    workspace::find_root(&cwd).context("could not find a Kestrl workspace")
}

fn record(event: &mut NewDbEvent, retention: &RetentionOptions) -> kestrl_core::Result<String> {
    let mut history = History::open_default()?;
    event.meta.session_id = history.current_session()?.map(|session| session.id);
    history.record_db(event, retention)
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
    for row in &mut result.rows {
        for value in row {
            if let serde_json::Value::String(text) = value {
                *text = redactor.redact(text);
            }
        }
    }
}
