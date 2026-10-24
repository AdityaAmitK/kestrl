//! `kestrl` command-line entry point.

mod commands;
mod context;

use clap::Parser;

use crate::commands::Command;
use crate::context::Context;

#[derive(Debug, Parser)]
#[command(name = "kestrl", version, about = "Local-first backend workbench")]
struct Cli {
    /// Who is running this command (me, cursor, claude-code, codex, ...)
    #[arg(long, global = true)]
    actor: Option<String>,
    /// Force JSON output (default when stdout is not a terminal)
    #[arg(long, global = true)]
    json: bool,
    #[command(subcommand)]
    command: Command,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let ctx = Context::new(cli.actor.as_deref(), cli.json);
    commands::dispatch(&ctx, cli.command)
}
