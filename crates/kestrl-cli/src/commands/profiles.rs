//! `kestrl profiles`: list profiles, or show one.

use clap::Args;
use kestrl_core::{profile, workspace};
use std::io::Write;

use crate::context::Context;

#[derive(Debug, Args)]
pub struct ProfilesArgs {
    pub name: Option<String>,
}

pub fn run(ctx: &Context, args: ProfilesArgs) -> anyhow::Result<()> {
    let root = workspace::find_root(&std::env::current_dir()?)?;
    let dir = root.join(".kestrl/profiles");
    if let Some(name) = args.name {
        let item = profile::load(&root, &name)?;
        if ctx.is_json {
            writeln!(
                std::io::stdout(),
                "{}",
                serde_json::to_string_pretty(
                    &serde_json::json!({"headers": item.headers, "query": item.query})
                )?
            )?;
        } else {
            let path = dir.join(format!("{name}.toml"));
            write!(std::io::stdout(), "{}", std::fs::read_to_string(path)?)?;
        }
    } else {
        let mut names = std::fs::read_dir(&dir)?
            .filter_map(|entry| entry.ok().map(|entry| entry.path()))
            .filter(|path| {
                path.extension()
                    .is_some_and(|extension| extension == "toml")
            })
            .filter_map(|path| {
                path.file_stem()
                    .and_then(|stem| stem.to_str())
                    .map(str::to_owned)
            })
            .collect::<Vec<_>>();
        names.sort();
        if ctx.is_json {
            writeln!(
                std::io::stdout(),
                "{}",
                serde_json::to_string_pretty(&names)?
            )?;
        } else {
            for name in names {
                writeln!(std::io::stdout(), "{name}")?;
            }
        }
    }
    Ok(())
}
