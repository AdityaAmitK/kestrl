//! `kestrl run`: send every request in a collection, in order.

use clap::Args;

use crate::context::Context;

#[derive(Debug, Args)]
pub struct RunArgs {
    /// Collection directory
    pub dir: std::path::PathBuf,
    #[command(flatten)]
    pub resolve: crate::commands::ResolveArgs,
}

pub fn run(_ctx: &Context, _args: RunArgs) -> anyhow::Result<()> {
    anyhow::bail!("`kestrl run` is not implemented yet")
}
