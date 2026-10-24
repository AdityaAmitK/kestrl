//! `kestrl session`: start or end a session.

use clap::Args;
use clap::Subcommand;

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

pub fn run(_ctx: &Context, _args: SessionArgs) -> anyhow::Result<()> {
    anyhow::bail!("`kestrl session` is not implemented yet")
}
