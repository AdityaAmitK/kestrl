//! `kestrl stop`: stop a service started with `kestrl up` or `kestrl logs attach`.

use clap::Args;

#[derive(Debug, Args)]
pub struct StopArgs {
    /// Service name, as shown by `kestrl ps`
    pub service: String,
}

pub fn run(args: StopArgs) -> anyhow::Result<()> {
    kestrl_core::services::supervisor::stop(&args.service)?;
    eprintln!("Stopped {}", args.service);
    Ok(())
}
