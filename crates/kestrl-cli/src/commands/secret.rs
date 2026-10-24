//! `kestrl secret`: manage secrets in the macOS Keychain.

use std::io::IsTerminal;
use std::io::Read;
use std::io::Write;

use clap::{Args, Subcommand};
use kestrl_core::{secrets, workspace};

use crate::context::Context;

#[derive(Debug, Args)]
pub struct SecretArgs {
    #[command(subcommand)]
    pub action: SecretAction,
}

#[derive(Debug, Subcommand)]
pub enum SecretAction {
    /// Set a secret in the macOS Keychain
    Set { name: String },
    /// Remove a secret from the macOS Keychain
    Rm { name: String },
    /// List secrets referenced by the workspace
    Ls,
}

pub fn run(ctx: &Context, args: SecretArgs) -> anyhow::Result<()> {
    match args.action {
        SecretAction::Set { name } => {
            let mut value = String::new();
            if std::io::stdin().is_terminal() {
                value = read_hidden()?;
            } else {
                std::io::stdin().read_to_string(&mut value)?;
            }
            while value.ends_with(['\n', '\r']) {
                value.pop();
            }
            if value.is_empty() {
                anyhow::bail!("secret value cannot be empty");
            }
            secrets::set(&name, &value)?;
        }
        SecretAction::Rm { name } => secrets::delete(&name)?,
        SecretAction::Ls => {
            let root = workspace::find_root(&std::env::current_dir()?)?;
            let found = secrets::referenced_in_workspace(&root)?;
            if ctx.is_json {
                let values = found
                    .iter()
                    .map(|name| {
                        Ok(serde_json::json!({
                            "name": name,
                            "is_set": secrets::is_set(name)?,
                        }))
                    })
                    .collect::<anyhow::Result<Vec<_>>>()?;
                crate::commands::output::print_json(&serde_json::Value::Array(values))?;
            } else {
                for name in found {
                    writeln!(
                        std::io::stdout(),
                        "{name}\t{}",
                        if secrets::is_set(&name)? {
                            "set"
                        } else {
                            "missing"
                        }
                    )?;
                }
            }
        }
    }
    Ok(())
}

fn read_hidden() -> anyhow::Result<String> {
    Ok(rpassword::prompt_password("Secret: ")?)
}
