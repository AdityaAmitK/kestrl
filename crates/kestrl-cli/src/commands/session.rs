//! `kestrl session`: start or end a session.

use std::io::{self, Write};

use anyhow::Context as _;
use clap::{Args, Subcommand};
use kestrl_core::history::History;

use crate::context::Context;

#[derive(Debug, Args)]
pub struct SessionArgs {
    #[command(subcommand)]
    pub action: SessionAction,
}

#[derive(Debug, Subcommand)]
pub enum SessionAction {
    /// Start a new session
    Start { name: Option<String> },
    /// End the current session
    End,
}

pub fn run(ctx: &Context, args: SessionArgs) -> anyhow::Result<()> {
    let history = History::open_default().context("failed to open history")?;
    match args.action {
        SessionAction::Start { name } => {
            let id = history.start_session(name.as_deref(), ctx.actor.as_str())?;
            writeln!(io::stdout(), "Started session {id}")?;
        }
        SessionAction::End => {
            let session = history.end_session()?;
            writeln!(
                io::stdout(),
                "Ended session {}",
                session.name.as_deref().unwrap_or(&session.id)
            )?;
        }
    }
    Ok(())
}
