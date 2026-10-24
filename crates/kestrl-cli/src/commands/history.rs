//! `kestrl history`: search and inspect history.

use clap::Args;

use crate::context::Context;

#[derive(Debug, Args)]
pub struct HistoryArgs {
    /// Full-text search query
    pub query: Option<String>,
}

pub fn run(_ctx: &Context, _args: HistoryArgs) -> anyhow::Result<()> {
    anyhow::bail!("`kestrl history` is not implemented yet")
}
