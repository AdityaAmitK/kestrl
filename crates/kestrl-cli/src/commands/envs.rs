//! `kestrl envs`: list environments, or show one with resolved variables.

use clap::Args;
use kestrl_core::{env::Environment, workspace};
use std::io::Write;

use crate::context::Context;

#[derive(Debug, Args)]
pub struct EnvsArgs {
    pub name: Option<String>,
}

pub fn run(ctx: &Context, args: EnvsArgs) -> anyhow::Result<()> {
    let root = workspace::find_root(&std::env::current_dir()?)?;
    if let Some(name) = args.name {
        let resolved = Environment::resolved(&root, &name)?;
        if ctx.is_json {
            writeln!(
                std::io::stdout(),
                "{}",
                serde_json::to_string_pretty(&resolved)?
            )?;
        } else {
            for (key, value) in resolved {
                writeln!(std::io::stdout(), "{key}={value}")?;
            }
        }
    } else {
        let names = Environment::names(&root)?;
        if ctx.is_json {
            writeln!(
                std::io::stdout(),
                "{}",
                serde_json::to_string_pretty(&names)?
            )?;
        } else {
            for name in names {
                writeln!(std::io::stdout(), "{name}")?;
            }
        }
    }
    Ok(())
}
