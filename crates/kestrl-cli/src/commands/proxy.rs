//! Certificate trust and system proxy recovery commands.

use anyhow::Context as _;
use clap::{Args, Subcommand};
use kestrl_core::capture;
use std::io::Write;

/// Proxy management subcommands.
#[derive(Debug, Args)]
pub struct ProxyArgs {
    #[command(subcommand)]
    command: ProxyCommand,
}

#[derive(Debug, Subcommand)]
enum ProxyCommand {
    /// Install the Kestrl root certificate on a device
    Trust(TrustOptions),
    /// Restore macOS and Android emulator proxy settings saved by an interrupted capture
    Restore,
    /// Print the root CA file path and fingerprint
    Ca,
}

#[derive(Debug, Args)]
#[group(required = true, multiple = false)]
struct TrustOptions {
    /// Trust on the booted iOS Simulator
    #[arg(long)]
    sim: bool,
    /// Push to the booted Android emulator
    #[arg(long)]
    android: bool,
}

/// Run a proxy management command.
pub fn run(args: ProxyArgs) -> anyhow::Result<()> {
    match args.command {
        ProxyCommand::Trust(options) if options.sim => {
            let paths = capture::ca::ensure()?;
            capture::device::trust_simulator(
                paths.certificate.to_str().context("CA path is not UTF-8")?,
            )?;
            writeln!(
                std::io::stdout(),
                "Kestrl root CA trusted by the booted iOS Simulator."
            )?;
        }
        ProxyCommand::Trust(options) => {
            let paths = capture::ca::ensure()?;
            capture::device::trust_android(
                paths.certificate.to_str().context("CA path is not UTF-8")?,
            )?;
            writeln!(
                std::io::stdout(),
                "Install /sdcard/Download/kestrl-ca.pem from Android Settings > Security > Encryption & credentials > Install a certificate > CA certificate."
            )?;
            writeln!(
                std::io::stdout(),
                "For debug builds, add a network_security_config.xml with <network-security-config><debug-overrides><trust-anchors><certificates src=\"user\" /></trust-anchors></debug-overrides></network-security-config> and reference it from the application manifest."
            )?;
            let _ = options;
        }
        ProxyCommand::Restore => {
            capture::session::restore_saved()?;
            writeln!(
                std::io::stdout(),
                "Device proxy settings saved by an interrupted capture restored."
            )?;
        }
        ProxyCommand::Ca => {
            let paths = capture::ca::ensure()?;
            let output = std::process::Command::new("shasum")
                .args([
                    "-a",
                    "256",
                    paths.certificate.to_str().context("CA path is not UTF-8")?,
                ])
                .output()?;
            if !output.status.success() {
                anyhow::bail!("could not calculate CA fingerprint");
            }
            writeln!(
                std::io::stdout(),
                "{}\n{}",
                paths.certificate.display(),
                String::from_utf8_lossy(&output.stdout)
                    .split_whitespace()
                    .next()
                    .unwrap_or("")
            )?;
        }
    }
    Ok(())
}
