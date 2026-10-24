//! List running services.
use clap::Args;
#[derive(Debug, Args)]
pub struct PsArgs {
    /// Output services as JSON
    #[arg(long)]
    pub json: bool,
}
#[allow(clippy::print_stdout)]
pub fn run(ctx: &crate::context::Context, a: PsArgs) -> anyhow::Result<()> {
    let v = kestrl_core::services::registry::list()?;
    if ctx.is_json || a.json {
        println!("{}", serde_json::to_string(&v)?)
    } else {
        println!("SERVICE PID PORTS UPTIME COMMAND");
        for s in v {
            println!(
                "{} {} {:?} {} {}",
                s.name, s.pid, s.ports, s.started_at, s.command
            )
        }
    }
    Ok(())
}
