//! `kestrl secret`: manage secrets in the macOS Keychain.

use std::io::Write;
use std::io::{IsTerminal, Read};

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
    Set { name: String },
    Rm { name: String },
    Ls,
}

pub fn run(_ctx: &Context, args: SecretArgs) -> anyhow::Result<()> {
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
            secrets::set(&name, &value)?;
        }
        SecretAction::Rm { name } => secrets::delete(&name)?,
        SecretAction::Ls => {
            let root = workspace::find_root(&std::env::current_dir()?)?;
            let mut found = std::collections::BTreeSet::new();
            scan(&root, &mut found)?;
            for name in found {
                writeln!(
                    std::io::stdout(),
                    "{name}\t{}",
                    if secrets::get(&name).is_ok() {
                        "set"
                    } else {
                        "missing"
                    }
                )?;
            }
        }
    }
    Ok(())
}

fn read_hidden() -> anyhow::Result<String> {
    Ok(rpassword::prompt_password("Secret: ")?)
}

fn scan(
    dir: &std::path::Path,
    found: &mut std::collections::BTreeSet<String>,
) -> anyhow::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            if path.file_name().is_some_and(|name| name == ".git") {
                continue;
            }
            scan(&path, found)?;
        } else if let Ok(content) = std::fs::read_to_string(&path) {
            let mut rest = content.as_str();
            while let Some(start) = rest.find("{{secret:") {
                rest = &rest[start + 9..];
                if let Some(end) = rest.find("}}") {
                    found.insert(rest[..end].to_owned());
                    rest = &rest[end + 2..];
                } else {
                    break;
                }
            }
        }
    }
    Ok(())
}
