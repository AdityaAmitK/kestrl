//! `kestrl run`: send every request in a collection, in order.

use clap::Args;
use kestrl_core::http;
use std::io::Write;

use crate::context::Context;

#[derive(Debug, Args)]
pub struct RunArgs {
    /// Collection directory
    pub dir: std::path::PathBuf,
    #[command(flatten)]
    pub resolve: crate::commands::ResolveArgs,
}

pub fn run(ctx: &Context, args: RunArgs) -> anyhow::Result<()> {
    let client = http::client()?;
    let mut paths = std::fs::read_dir(&args.dir)?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.ends_with(".req.toml"))
        })
        .collect::<Vec<_>>();
    paths.sort();
    let mut results = Vec::new();
    for path in paths {
        let sent = super::send::send_one_with_client(
            ctx,
            &[path.display().to_string()],
            &args.resolve,
            &client,
            None,
        )?;
        let exchange = &sent.exchange;
        if !ctx.is_json {
            writeln!(
                std::io::stdout(),
                "{} {} {} ({} ms)",
                path.display(),
                exchange
                    .status
                    .map_or_else(|| "failed".to_owned(), |status| status.to_string()),
                if exchange.error.is_some() || exchange.status.is_none_or(|status| status >= 400) {
                    "failed"
                } else {
                    "ok"
                },
                exchange.duration_ms
            )?;
        }
        results.push(sent);
    }
    if ctx.is_json {
        super::output::print_json(&serde_json::Value::Array(
            results
                .iter()
                .map(|sent| super::output::exchange_json(&sent.exchange, sent.event_id.as_deref()))
                .collect(),
        ))?;
    }
    if results.iter().any(|sent| {
        sent.exchange.error.is_some() || sent.exchange.status.is_none_or(|status| status >= 400)
    }) {
        anyhow::bail!("one or more requests failed");
    }
    Ok(())
}
