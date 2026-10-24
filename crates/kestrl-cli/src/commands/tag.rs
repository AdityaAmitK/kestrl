//! `kestrl tag`: tag a history event.

use clap::Args;

use crate::context::Context;

#[derive(Debug, Args)]
pub struct TagArgs {
    /// Event id
    pub event: String,
    /// Tag, e.g. login-bug
    pub tag: String,
}

pub fn run(_ctx: &Context, _args: TagArgs) -> anyhow::Result<()> {
    anyhow::bail!("`kestrl tag` is not implemented yet")
}
