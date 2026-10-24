//! `kestrl profiles`: list profiles, or show one.

use clap::Args;

use crate::context::Context;

#[derive(Debug, Args)]
pub struct ProfilesArgs {
    /// Profile name
    pub name: Option<String>,
}

pub fn run(_ctx: &Context, _args: ProfilesArgs) -> anyhow::Result<()> {
    anyhow::bail!("`kestrl profiles` is not implemented yet")
}
