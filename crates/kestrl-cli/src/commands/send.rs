//! `kestrl send`: send one request (file or METHOD URL).

use std::collections::BTreeMap;
use std::path::Path;

use clap::Args;
use kestrl_core::{
    env::Environment, http, request, secrets, vars::Variables, workspace::WorkspaceConfig,
};

use crate::context::Context;

#[derive(Debug, Args)]
pub struct SendArgs {
    #[arg(required = true, num_args = 1..=2)]
    pub target: Vec<String>,
    #[command(flatten)]
    pub resolve: crate::commands::ResolveArgs,
    /// Tag to add to the recorded event
    #[arg(long)]
    pub tag: Option<String>,
}

pub fn send_one(
    ctx: &Context,
    target: &[String],
    resolve: &crate::commands::ResolveArgs,
) -> anyhow::Result<http::Exchange> {
    let _actor = &ctx.actor;
    let root = match request::workspace_path() {
        Ok(root) => root,
        Err(_) => std::env::current_dir()?,
    };
    let config = WorkspaceConfig::load(&root)?;
    let env_name = resolve
        .env
        .as_deref()
        .or(config.default_env.as_deref())
        .unwrap_or("local");
    let environment = if root
        .join(".kestrl/envs")
        .join(format!("{env_name}.toml"))
        .exists()
    {
        Environment::load(&root, env_name)?.vars
    } else {
        BTreeMap::new()
    };
    let vars = Variables {
        overrides: parse_vars(&resolve.vars)?,
        environment,
        process: std::env::vars().collect(),
        workspace: config.vars.clone(),
        redactor: secrets::Redactor::default(),
    };
    let request_file = if target.len() == 2 {
        request::ad_hoc(&target[0], &target[1])
    } else {
        request::load(Path::new(&target[0]))?
    };
    let resolved = request::resolve(
        request_file,
        &root,
        &config,
        env_name,
        &resolve.profiles,
        vars,
    )?;
    Ok(http::send(resolved)?)
}

fn parse_vars(values: &[String]) -> anyhow::Result<BTreeMap<String, String>> {
    values
        .iter()
        .map(|value| {
            let (name, content) = value
                .split_once('=')
                .ok_or_else(|| anyhow::anyhow!("--var must use key=value"))?;
            Ok((name.to_owned(), content.to_owned()))
        })
        .collect()
}

pub fn run(ctx: &Context, args: SendArgs) -> anyhow::Result<()> {
    let exchange = send_one(ctx, &args.target, &args.resolve)?;
    if let Some(_tag) = args.tag {
        // Recorded once history lands.
    }
    if ctx.is_json {
        crate::commands::output::print_json(&exchange)?;
    } else {
        crate::commands::output::print_human(&exchange);
    }
    if let Some(error) = exchange.error {
        anyhow::bail!("{error}");
    }
    Ok(())
}
