//! `kestrl send`: send one request (file or METHOD URL).

use std::collections::BTreeMap;
use std::path::Path;

use clap::Args;
use kestrl_core::env::Environment;
use kestrl_core::http;
use kestrl_core::request;
use kestrl_core::secrets;
use kestrl_core::vars::Variables;
use kestrl_core::workspace::{self, WorkspaceConfig};

use crate::context::Context;

#[derive(Debug, Args)]
pub struct SendArgs {
    /// Request file path, or an HTTP method followed by a URL
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
    let client = http::client()?;
    send_one_with_client(ctx, target, resolve, &client)
}

pub fn send_one_with_client(
    ctx: &Context,
    target: &[String],
    resolve: &crate::commands::ResolveArgs,
    client: &http::Client,
) -> anyhow::Result<http::Exchange> {
    let _actor = &ctx.actor;
    let root = match workspace::find_root(&std::env::current_dir()?) {
        Ok(root) => root,
        Err(_) => std::env::current_dir()?,
    };
    let config = WorkspaceConfig::load(&root)?;
    let env_name = resolve.env.as_deref().or(config.default_env.as_deref());
    let env_name = env_name.unwrap_or("local");
    let environment = match Environment::load(&root, env_name) {
        Ok(environment) => environment.vars,
        Err(kestrl_core::Error::Read { source, .. })
            if resolve.env.is_none()
                && config.default_env.is_none()
                && source.kind() == std::io::ErrorKind::NotFound =>
        {
            BTreeMap::new()
        }
        Err(kestrl_core::Error::Read { source, .. })
            if source.kind() == std::io::ErrorKind::NotFound =>
        {
            let available = Environment::names(&root)?.join(", ");
            return Err(kestrl_core::Error::EnvironmentNotFound {
                name: env_name.to_owned(),
                available,
            }
            .into());
        }
        Err(error) => return Err(error.into()),
    };
    let vars = Variables {
        overrides: parse_vars(&resolve.vars)?,
        environment,
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
    Ok(http::send(client, resolved)?)
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
    if ctx.is_json {
        crate::commands::output::print_exchange_json(&exchange)?;
    } else {
        crate::commands::output::print_human(&exchange);
    }
    if let Some(error) = exchange.error {
        anyhow::bail!("{error}");
    }
    Ok(())
}
