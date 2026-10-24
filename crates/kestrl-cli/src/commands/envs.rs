//! `kestrl envs`: list environments, or show one with resolved variables.

use clap::Args;

use crate::context::Context;

#[derive(Debug, Args)]
pub struct EnvsArgs {
    /// Environment name
    pub name: Option<String>,
}

pub fn run(_ctx: &Context, _args: EnvsArgs) -> anyhow::Result<()> {
    anyhow::bail!("`kestrl envs` is not implemented yet")
}
