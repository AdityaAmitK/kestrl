//! `kestrl history`: search, show and edit history events.

use std::io::{self, Write};

use anyhow::{Context as _, bail};
use clap::{Args, Subcommand};
use kestrl_core::history::{EventDetail, EventSummary, History, HistoryFilter, now_ms};

use crate::context::Context;

#[derive(Debug, Args)]
#[command(args_conflicts_with_subcommands = true)]
pub struct HistoryArgs {
    #[command(subcommand)]
    pub action: Option<HistoryAction>,
    /// Full-text search query
    pub query: Option<String>,
    /// Only events by this actor (me, cursor, claude-code, codex, ...)
    #[arg(long = "by", value_name = "ACTOR")]
    pub by: Option<String>,
    /// Only events with this tag
    #[arg(long)]
    pub tag: Option<String>,
    /// Only events from the last 30m, 1h or 2d
    #[arg(long)]
    pub since: Option<String>,
    /// Only events in this session
    #[arg(long)]
    pub session: Option<String>,
    /// Maximum number of events
    #[arg(long, default_value_t = 20)]
    pub limit: u32,
}

#[derive(Debug, Subcommand)]
pub enum HistoryAction {
    /// Show one event in full
    Show {
        /// Event id or unique prefix
        id: String,
    },
    /// Change an event's actor or note
    Edit {
        /// Event id or unique prefix
        id: String,
        /// New actor, e.g. claude-code
        #[arg(long = "set-actor", value_name = "ACTOR")]
        set_actor: Option<String>,
        /// New note; makes the event saved
        #[arg(long)]
        note: Option<String>,
    },
}

pub fn run(ctx: &Context, args: HistoryArgs) -> anyhow::Result<()> {
    let history = History::open_default().context("failed to open history")?;
    match args.action.as_ref() {
        Some(HistoryAction::Show { id }) => show(ctx, &history, id),
        Some(HistoryAction::Edit {
            id,
            set_actor,
            note,
        }) => edit(&history, id, set_actor.as_deref(), note.as_deref()),
        None => list(ctx, &history, args),
    }
}

fn list(ctx: &Context, history: &History, args: HistoryArgs) -> anyhow::Result<()> {
    let filter = HistoryFilter {
        text: args.query,
        actor: args.by,
        tag: args.tag,
        since_ms: args.since.as_deref().map(since_ms).transpose()?,
        session_id: args.session,
        limit: args.limit,
    };
    let events = history.list(&filter)?;
    let mut out = io::stdout().lock();
    if ctx.is_json {
        writeln!(out, "{}", serde_json::to_string_pretty(&events)?)?;
        return Ok(());
    }
    if events.is_empty() {
        writeln!(out, "No matching events.")?;
    }
    let now = now_ms();
    for event in &events {
        write_row(&mut out, event, now)?;
    }
    Ok(())
}

fn show(ctx: &Context, history: &History, id: &str) -> anyhow::Result<()> {
    let detail = history.show(id)?;
    let mut out = io::stdout().lock();
    if ctx.is_json {
        writeln!(out, "{}", serde_json::to_string_pretty(&detail)?)?;
    } else {
        write_detail(&mut out, &detail)?;
    }
    Ok(())
}

fn edit(
    history: &History,
    id: &str,
    actor: Option<&str>,
    note: Option<&str>,
) -> anyhow::Result<()> {
    if actor.is_none() && note.is_none() {
        bail!("nothing to edit; pass --set-actor or --note");
    }
    if let Some(actor) = actor {
        history.set_actor(id, actor)?;
    }
    if let Some(note) = note {
        history.set_note(id, note)?;
    }
    writeln!(io::stdout(), "Updated {id}")?;
    Ok(())
}

fn write_row(out: &mut impl Write, event: &EventSummary, now: i64) -> io::Result<()> {
    let tags: Vec<String> = event.tags.iter().map(|tag| format!("#{tag}")).collect();
    writeln!(
        out,
        "{}  {:>4}  {:<12}  {}  {}",
        event.id,
        age(now - event.ts),
        event.actor,
        event.summary,
        tags.join(" ")
    )
}

fn write_detail(out: &mut impl Write, detail: &EventDetail) -> io::Result<()> {
    let event = &detail.event;
    writeln!(out, "{}  {}", event.id, event.summary)?;
    writeln!(out, "when:      {} ago", age(now_ms() - event.ts))?;
    writeln!(out, "actor:     {}", event.actor)?;
    writeln!(out, "source:    {}", event.source)?;
    for (label, value) in [
        ("env", &event.env),
        ("session", &detail.session_id),
        ("workspace", &detail.workspace),
        ("request", &detail.request_ref),
        ("correlation", &detail.correlation),
        ("note", &detail.note),
    ] {
        if let Some(value) = value {
            writeln!(out, "{label}:{}{value}", " ".repeat(10 - label.len()))?;
        }
    }
    if !event.tags.is_empty() {
        let tags: Vec<String> = event.tags.iter().map(|tag| format!("#{tag}")).collect();
        writeln!(out, "tags:      {}", tags.join(" "))?;
    }
    if let Some(http) = &detail.http {
        writeln!(out)?;
        writeln!(out, "{} {}", http.method, http.url)?;
        writeln!(
            out,
            "status:    {}",
            http.status.map_or("-".to_owned(), |s| s.to_string())
        )?;
        writeln!(out, "duration:  {}ms", http.duration_ms)?;
        if let Some(error) = &http.error {
            writeln!(out, "error:     {error}")?;
        }
        for (name, value) in &http.req_headers {
            writeln!(out, "> {name}: {value}")?;
        }
        for (name, value) in &http.res_headers {
            writeln!(out, "< {name}: {value}")?;
        }
        for body in [&http.req_body, &http.res_body].into_iter().flatten() {
            writeln!(out)?;
            writeln!(out, "{body}")?;
        }
    }
    Ok(())
}

/// Unix ms at the start of a window such as `30m`, `1h` or `2d`.
fn since_ms(spec: &str) -> anyhow::Result<i64> {
    let (count, unit_ms) = if let Some(count) = spec.strip_suffix('m') {
        (count, 60_000)
    } else if let Some(count) = spec.strip_suffix('h') {
        (count, 3_600_000)
    } else if let Some(count) = spec.strip_suffix('d') {
        (count, 86_400_000)
    } else {
        bail!("--since must end in m, h or d, e.g. 1h");
    };
    let count: i64 = count
        .parse()
        .with_context(|| format!("invalid --since {spec:?}"))?;
    Ok(now_ms().saturating_sub(count.saturating_mul(unit_ms)))
}

/// Compact age such as `5s`, `12m`, `3h` or `2d`.
fn age(ms: i64) -> String {
    let secs = ms.max(0) / 1000;
    match secs {
        0..60 => format!("{secs}s"),
        60..3600 => format!("{}m", secs / 60),
        3600..86400 => format!("{}h", secs / 3600),
        _ => format!("{}d", secs / 86400),
    }
}
