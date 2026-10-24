//! `kestrl send`: send one request (file or METHOD URL).

use clap::Args;

use crate::context::Context;

#[derive(Debug, Args)]
pub struct SendArgs {
    /// Request file path, or an HTTP method followed by a URL
    #[arg(required = true, num_args = 1..=2)]
    pub target: Vec<String>,
    #[command(flatten)]
    pub resolve: crate::commands::ResolveArgs,
    /// Tag to add to the recorded event
    #[arg(long)]
    pub tag: Option<String>,
}

pub fn run(_ctx: &Context, _args: SendArgs) -> anyhow::Result<()> {
    anyhow::bail!("`kestrl send` is not implemented yet")
}
