//! `kestrl history`: search, show and edit history events.

use std::collections::BTreeMap;
use std::io::{self, Write};
use std::path::PathBuf;

use anyhow::{Context as _, anyhow, bail};
use clap::{Args, Subcommand};
use kestrl_core::duration::parse_since;
use kestrl_core::history::{
    BodyDiff, EventDetail, EventDiff, EventSummary, History, HistoryFilter, LineOp, now_ms,
};
use kestrl_core::http;
use kestrl_core::request::ResolvedRequest;
use kestrl_core::secrets::Redactor;
use kestrl_core::workspace::WorkspaceConfig;

use super::ResolveArgs;
use super::send::{self, Sent};
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
    /// Only events from the last 30s, 2m, 1h or 2d
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
    /// Send a past request again, as it was stored
    Replay {
        /// Event id or unique prefix
        id: String,
    },
    /// Compare two HTTP events: status, duration, headers and body
    Diff {
        /// Event id or unique prefix
        a: String,
        /// Event id or unique prefix
        b: String,
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
        Some(HistoryAction::Replay { id }) => {
            let sent = replay(ctx, &history, id)?;
            send::print_sent(ctx, &sent)
        }
        Some(HistoryAction::Diff { a, b }) => diff(ctx, &history, a, b),
        None => list(ctx, &history, args),
    }
}

fn list(ctx: &Context, history: &History, args: HistoryArgs) -> anyhow::Result<()> {
    let filter = HistoryFilter {
        text: args.query,
        actor: args.by,
        tag: args.tag,
        since_ms: args.since.as_deref().map(parse_since).transpose()?,
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

/// Send the stored request again. Secrets are never stored, so when the event
/// has redactions the request is re-resolved from its request file instead.
fn replay(ctx: &Context, history: &History, id: &str) -> anyhow::Result<Sent> {
    let detail = history.show(id)?;
    let original = detail.event.id.clone();
    let stored = detail
        .http
        .ok_or_else(|| anyhow!("{original} is not an HTTP event"))?;
    let client = http::client()?;
    if stored.redacted.is_empty() {
        let request = ResolvedRequest {
            method: stored.method,
            url: stored.url,
            headers: stored.req_headers.into_iter().collect(),
            query: BTreeMap::new(),
            body: stored.req_body,
            redactor: Redactor::default(),
        };
        let exchange = http::send(&client, request)?;
        let root = send::workspace_root()?;
        let config = WorkspaceConfig::load(&root)?;
        let meta = send::event_meta(ctx, &root, detail.event.env, detail.request_ref);
        let event_id = send::record(&exchange, meta, &config.retention, Some(&original));
        Ok(Sent { exchange, event_id })
    } else {
        let request_ref = detail.request_ref.ok_or_else(|| {
            anyhow!(
                "cannot replay {original}: it used secrets ({}) and has no request file to re-resolve",
                stored.redacted.join(", ")
            )
        })?;
        let path = PathBuf::from(detail.workspace.unwrap_or_default()).join(request_ref);
        crate::commands::output::warn(&format!(
            "{original} used secrets, so it is re-sent from {} as the file is now; profiles, --var, -H and body given on the original command line are not included",
            path.display()
        ));
        let resolve = ResolveArgs {
            env: detail.event.env,
            profiles: Vec::new(),
            vars: Vec::new(),
        };
        send::send_one_with_client(
            ctx,
            &[path.display().to_string()],
            &resolve,
            &send::ContentArgs::default(),
            &client,
            Some(&original),
        )
    }
}

fn diff(ctx: &Context, history: &History, a: &str, b: &str) -> anyhow::Result<()> {
    let left = history.show(a)?;
    let right = history.show(b)?;
    let (Some(left_http), Some(right_http)) = (&left.http, &right.http) else {
        bail!("history diff compares two HTTP events");
    };
    let diff = kestrl_core::history::diff_exchanges(left_http, right_http);
    let mut out = io::stdout().lock();
    if ctx.is_json {
        writeln!(out, "{}", serde_json::to_string_pretty(&diff)?)?;
    } else {
        write_diff(&mut out, &diff)?;
    }
    Ok(())
}

fn write_diff(out: &mut impl Write, diff: &EventDiff) -> io::Result<()> {
    writeln!(
        out,
        "status:    {} → {}",
        status_text(diff.status.0),
        status_text(diff.status.1)
    )?;
    writeln!(
        out,
        "duration:  {}ms → {}ms",
        diff.duration_ms.0, diff.duration_ms.1
    )?;
    if diff.headers.is_empty() {
        writeln!(out, "headers:   no changes")?;
    } else {
        writeln!(out, "headers:")?;
        for (name, value) in &diff.headers.added {
            writeln!(out, "  + {name}: {value}")?;
        }
        for (name, value) in &diff.headers.removed {
            writeln!(out, "  - {name}: {value}")?;
        }
        for change in &diff.headers.changed {
            writeln!(out, "  ~ {}: {} → {}", change.name, change.from, change.to)?;
        }
    }
    match &diff.body {
        BodyDiff::Identical => writeln!(out, "body:      identical")?,
        BodyDiff::Json { changes } => {
            writeln!(out, "body (json):")?;
            for change in changes {
                writeln!(
                    out,
                    "  {}: {} → {}",
                    change.path,
                    value_text(change.from.as_ref()),
                    value_text(change.to.as_ref())
                )?;
            }
        }
        BodyDiff::Lines { ops } => {
            writeln!(out, "body (text):")?;
            for op in ops {
                match op {
                    LineOp::Added { text } => writeln!(out, "  + {text}")?,
                    LineOp::Removed { text } => writeln!(out, "  - {text}")?,
                }
            }
        }
    }
    Ok(())
}

fn status_text(status: Option<u16>) -> String {
    status.map_or_else(|| "-".to_owned(), |status| status.to_string())
}

fn value_text(value: Option<&serde_json::Value>) -> String {
    value.map_or_else(|| "(missing)".to_owned(), serde_json::Value::to_string)
}

fn write_row(out: &mut impl Write, event: &EventSummary, now: i64) -> io::Result<()> {
    let tags: Vec<String> = event.tags.iter().map(|tag| format!("#{tag}")).collect();
    writeln!(
        out,
        "{}  {:>4}  {:<12}  {}  {}",
        event.id,
        crate::commands::output::age(now - event.ts),
        event.actor,
        event.summary,
        tags.join(" ")
    )
}

fn write_detail(out: &mut impl Write, detail: &EventDetail) -> io::Result<()> {
    let event = &detail.event;
    writeln!(out, "{}  {}", event.id, event.summary)?;
    writeln!(
        out,
        "when:      {} ago",
        crate::commands::output::age(now_ms() - event.ts)
    )?;
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
