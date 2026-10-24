//! Start a service through Kestrl so its logs are collected.
use clap::Args;
#[derive(Debug, Args)]
pub struct UpArgs {
    /// Service name
    pub service: String,
    /// Command and arguments to run
    #[arg(required = true, last = true, help = "Command and arguments to run")]
    pub command: Vec<String>,
}
pub fn run(_: &crate::context::Context, a: UpArgs) -> anyhow::Result<()> {
    let code = kestrl_core::services::supervisor::run(&a.service, &a.command)?;
    std::process::exit(code)
}
