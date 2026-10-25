//! `kestrl envs`: list environments, show one, or create and edit them.

use std::io::Write;

use anyhow::Context as _;
use anyhow::bail;
use clap::Args;
use clap::Subcommand;
use kestrl_core::env::Environment;
use kestrl_core::env_files;
use kestrl_core::env_files::RawVariable;
use kestrl_core::workspace;

use crate::context::Context;

#[derive(Debug, Args)]
#[command(args_conflicts_with_subcommands = true)]
pub struct EnvsArgs {
    #[command(subcommand)]
    pub action: Option<EnvsAction>,
    /// Environment to show with resolved variables (secrets masked)
    pub name: Option<String>,
}

#[derive(Debug, Subcommand)]
pub enum EnvsAction {
    /// Create an environment, empty or as a copy of another
    New {
        /// Name of the new environment, e.g. staging
        name: String,
        /// Duplicate this environment's variables and comments
        #[arg(long = "from", value_name = "ENV")]
        copy_from: Option<String>,
    },
    /// Set variables in an environment: `KEY=VALUE`, repeatable
    Set {
        /// Environment to change
        env: String,
        /// Assignments such as api_url=http://localhost:3000
        #[arg(required = true, value_name = "KEY=VALUE")]
        assignments: Vec<String>,
    },
    /// Remove variables from an environment
    Unset {
        /// Environment to change
        env: String,
        /// Variable names to remove
        #[arg(required = true)]
        keys: Vec<String>,
    },
    /// Rename an environment (the default follows it)
    Rename {
        /// Current name
        from: String,
        /// New name
        to: String,
    },
    /// Delete an environment (not the default one)
    Rm {
        /// Environment to delete
        name: String,
    },
    /// Make an environment the workspace default
    Default {
        /// Environment to use by default
        name: String,
    },
}

pub fn run(ctx: &Context, args: EnvsArgs) -> anyhow::Result<()> {
    let root = workspace::find_current()?;
    match args.action {
        Some(EnvsAction::New { name, copy_from }) => {
            env_files::create(&root, &name, copy_from.as_deref())?;
            done(ctx, &format!("Created environment {name}"))
        }
        Some(EnvsAction::Set { env, assignments }) => {
            let mut variables = env_files::variables(&root, &env)?;
            for assignment in assignments {
                let (key, value) = assignment
                    .split_once('=')
                    .with_context(|| format!("{assignment:?} is not KEY=VALUE"))?;
                match variables.iter_mut().find(|variable| variable.name == key) {
                    Some(variable) => variable.value = value.to_owned(),
                    None => variables.push(RawVariable {
                        name: key.to_owned(),
                        value: value.to_owned(),
                    }),
                }
            }
            env_files::save_variables(&root, &env, &variables)?;
            done(ctx, &format!("Updated {env}"))
        }
        Some(EnvsAction::Unset { env, keys }) => {
            let mut variables = env_files::variables(&root, &env)?;
            for key in &keys {
                if !variables.iter().any(|variable| &variable.name == key) {
                    bail!("{env} has no variable {key:?}");
                }
            }
            variables.retain(|variable| !keys.contains(&variable.name));
            env_files::save_variables(&root, &env, &variables)?;
            done(ctx, &format!("Updated {env}"))
        }
        Some(EnvsAction::Rename { from, to }) => {
            env_files::rename(&root, &from, &to)?;
            done(ctx, &format!("Renamed {from} to {to}"))
        }
        Some(EnvsAction::Rm { name }) => {
            env_files::delete(&root, &name)?;
            done(ctx, &format!("Deleted environment {name}"))
        }
        Some(EnvsAction::Default { name }) => {
            env_files::set_default(&root, &name)?;
            done(ctx, &format!("{name} is now the default environment"))
        }
        None => match args.name {
            Some(name) => show(ctx, &root, &name),
            None => list(ctx, &root),
        },
    }
}

fn show(ctx: &Context, root: &std::path::Path, name: &str) -> anyhow::Result<()> {
    let resolved = Environment::resolved(root, name)?;
    let mut out = std::io::stdout().lock();
    if ctx.is_json {
        writeln!(out, "{}", serde_json::to_string_pretty(&resolved)?)?;
    } else {
        for (key, value) in resolved {
            writeln!(out, "{key}={value}")?;
        }
    }
    Ok(())
}

fn list(ctx: &Context, root: &std::path::Path) -> anyhow::Result<()> {
    let names = Environment::names(root)?;
    let mut out = std::io::stdout().lock();
    if ctx.is_json {
        writeln!(out, "{}", serde_json::to_string_pretty(&names)?)?;
    } else {
        for name in names {
            writeln!(out, "{name}")?;
        }
    }
    Ok(())
}

fn done(ctx: &Context, message: &str) -> anyhow::Result<()> {
    let mut out = std::io::stdout().lock();
    if ctx.is_json {
        writeln!(
            out,
            "{}",
            serde_json::json!({ "ok": true, "message": message })
        )?;
    } else {
        writeln!(out, "{message}")?;
    }
    Ok(())
}
