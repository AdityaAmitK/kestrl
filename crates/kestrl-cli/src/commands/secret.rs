//! `kestrl secret`: manage secrets in the macOS Keychain.

use clap::Args;
use clap::Subcommand;

use crate::context::Context;

#[derive(Debug, Args)]
pub struct SecretArgs {
    #[command(subcommand)]
    pub action: SecretAction,
}

#[derive(Debug, Subcommand)]
pub enum SecretAction {
    /// Store a secret (value read from stdin or a prompt)
    Set { name: String },
    /// Remove a secret
    Rm { name: String },
    /// List secret names used in this workspace
    Ls,
}

pub fn run(_ctx: &Context, _args: SecretArgs) -> anyhow::Result<()> {
    anyhow::bail!("`kestrl secret` is not implemented yet")
}
