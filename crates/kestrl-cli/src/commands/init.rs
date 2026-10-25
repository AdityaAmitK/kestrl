//! Create a starter workspace.

use std::io::Write;
use std::path::PathBuf;

use anyhow::Context as _;
use clap::Args;
use kestrl_core::workspace;

use crate::context::Context;

/// The folder to initialize.
#[derive(Debug, Args)]
pub struct InitArgs {
    /// Workspace folder (defaults to the current directory)
    #[arg(default_value = ".")]
    pub dir: PathBuf,
}

/// Create files and print the report and the next request to send.
pub fn run(ctx: &Context, args: InitArgs) -> anyhow::Result<()> {
    let report = workspace::init(&args.dir).context("could not initialize workspace")?;
    let next_steps = "kestrl send requests/health.req.toml";
    let mut out = std::io::stdout().lock();
    if ctx.is_json {
        writeln!(
            out,
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "root": args.dir,
                "created_paths": report.created_paths,
                "next_steps": next_steps,
            }))?
        )?;
    } else {
        for path in report.created_paths {
            writeln!(out, "Created {}", args.dir.join(path).display())?;
        }
        writeln!(out, "\nFrom {} run:\n  {next_steps}", args.dir.display())?;
    }
    Ok(())
}
