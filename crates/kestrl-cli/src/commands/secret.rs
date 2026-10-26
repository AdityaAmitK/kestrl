//! `kestrl secret`: set, remove and list secrets. Values are never shown.

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
    /// Set a secret; the value is read from stdin or a hidden prompt
    Set { name: String },
    /// Remove a stored secret
    Rm { name: String },
    /// List secrets referenced by the workspace or stored, without values
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
            // Outside a workspace there are no references; stored secrets are still listed.
            let referenced = match workspace::find_root(&std::env::current_dir()?) {
                Ok(root) => secrets::referenced_in_workspace(&root)?,
                Err(_) => Default::default(),
            };
            let stored = secrets::stored_names()?;
            let names: std::collections::BTreeSet<&String> = referenced.union(&stored).collect();
            if ctx.is_json {
                let values = names
                    .iter()
                    .map(|name| {
                        serde_json::json!({
                            "name": name,
                            "is_set": stored.contains(*name),
                            "is_referenced": referenced.contains(*name),
                        })
                    })
                    .collect();
                crate::commands::output::print_json(&serde_json::Value::Array(values))?;
            } else {
                for name in names {
                    let status = if stored.contains(name) {
                        "set"
                    } else {
                        "missing"
                    };
                    let usage = if referenced.contains(name) {
                        ""
                    } else {
                        "\tunused"
                    };
                    writeln!(std::io::stdout(), "{name}\t{status}{usage}")?;
                }
            }
        }
    }
    Ok(())
}

fn read_hidden() -> anyhow::Result<String> {
    Ok(rpassword::prompt_password("Secret: ")?)
}
