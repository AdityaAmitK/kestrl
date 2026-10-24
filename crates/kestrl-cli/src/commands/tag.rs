//! `kestrl tag`: add or remove a tag on a history event.

use std::io::{self, Write};

use anyhow::{Context as _, bail};
use clap::Args;
use kestrl_core::history::History;

use crate::context::Context;

#[derive(Debug, Args)]
pub struct TagArgs {
    /// Event id or unique prefix
    pub event: String,
    /// Tag, e.g. login-bug (a leading # is optional)
    pub tag: String,
    /// Remove the tag instead of adding it
    #[arg(long)]
    pub remove: bool,
}

pub fn run(_ctx: &Context, args: TagArgs) -> anyhow::Result<()> {
    let tag = args.tag.trim_start_matches('#');
    if tag.is_empty() {
        bail!("tag must not be empty");
    }
    let history = History::open_default().context("failed to open history")?;
    if args.remove {
        history.remove_tag(&args.event, tag)?;
        writeln!(io::stdout(), "Removed #{tag} from {}", args.event)?;
    } else {
        history.add_tag(&args.event, tag)?;
        writeln!(io::stdout(), "Tagged {} #{tag}", args.event)?;
    }
    Ok(())
}
