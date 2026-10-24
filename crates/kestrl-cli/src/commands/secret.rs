//! `kestrl secret`: manage secrets in the macOS Keychain.

use std::io::IsTerminal;
use std::io::Read;
use std::io::Write;
use std::path::Path;

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
            let mut found = std::collections::BTreeSet::new();
            let kestrl_dir = root.join(".kestrl");
            if kestrl_dir.is_dir() {
                scan(&kestrl_dir, true, &mut found)?;
            }
            scan(&root, false, &mut found)?;
            if ctx.is_json {
                let values = found
                    .iter()
                    .map(|name| {
                        Ok(serde_json::json!({
                            "name": name,
                            "is_set": is_set(name)?,
                        }))
                    })
                    .collect::<anyhow::Result<Vec<_>>>()?;
                crate::commands::output::print_json(&serde_json::Value::Array(values))?;
            } else {
                for name in found {
                    writeln!(
                        std::io::stdout(),
                        "{name}\t{}",
                        if is_set(&name)? { "set" } else { "missing" }
                    )?;
                }
            }
        }
    }
    Ok(())
}

fn is_set(name: &str) -> anyhow::Result<bool> {
    match secrets::get(name) {
        Ok(_) => Ok(true),
        Err(kestrl_core::Error::SecretNotSet { .. }) => Ok(false),
        Err(error) => Err(error.into()),
    }
}

fn read_hidden() -> anyhow::Result<String> {
    Ok(rpassword::prompt_password("Secret: ")?)
}

fn scan(
    dir: &Path,
    is_kestrl: bool,
    found: &mut std::collections::BTreeSet<String>,
) -> anyhow::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            let name = path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or_default();
            if name == "target" || name == "node_modules" || name == ".git" || name.starts_with('.')
            {
                continue;
            }
            scan(&path, is_kestrl || name == ".kestrl", found)?;
        } else if (is_kestrl
            || path
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.ends_with(".req.toml")))
            && let Ok(content) = std::fs::read_to_string(&path)
        {
            found.extend(secrets::referenced_names(&content));
        }
    }
    Ok(())
}
