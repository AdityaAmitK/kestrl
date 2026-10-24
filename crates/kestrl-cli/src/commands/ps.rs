//! List services managed by Kestrl.

use std::io;
use std::io::Write;

use clap::Args;
use kestrl_core::history::now_ms;
use kestrl_core::services::registry::Service;

use crate::commands::output::age;

/// List running services.
#[derive(Debug, Args)]
pub struct PsArgs {}

/// Print the current service list.
pub fn run(context: &crate::context::Context, _args: PsArgs) -> anyhow::Result<()> {
    let services = kestrl_core::services::registry::list()?;
    let mut stdout = io::stdout().lock();
    if context.is_json {
        writeln!(stdout, "{}", serde_json::to_string(&services)?)?;
        return Ok(());
    }

    writeln!(
        stdout,
        "{:<16} {:>7} {:<16} {:>6} COMMAND",
        "SERVICE", "PID", "PORTS", "UPTIME"
    )?;
    let now = now_ms();
    for service in &services {
        write_service(&mut stdout, service, now)?;
    }
    Ok(())
}

fn write_service(stdout: &mut impl Write, service: &Service, now: i64) -> io::Result<()> {
    let ports = service
        .ports
        .iter()
        .map(u16::to_string)
        .collect::<Vec<_>>()
        .join(", ");
    let uptime = age(now.saturating_sub(service.started_at));
    let command = truncate(&service.command, 60);
    writeln!(
        stdout,
        "{:<16} {:>7} {:<16} {:>6} {}",
        service.name, service.pid, ports, uptime, command
    )
}

fn truncate(value: &str, maximum_chars: usize) -> String {
    let mut characters = value.chars();
    let prefix: String = characters
        .by_ref()
        .take(maximum_chars.saturating_sub(1))
        .collect();
    if characters.next().is_some() {
        format!("{prefix}…")
    } else {
        prefix
    }
}
