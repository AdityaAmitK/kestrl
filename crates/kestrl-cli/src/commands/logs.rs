//! Tail or follow a service's live buffer, or attach an external log source.

use std::io;
use std::io::Write;
use std::path::PathBuf;

use anyhow::bail;
use clap::ArgGroup;
use clap::Args;
use clap::Subcommand;
use kestrl_core::duration::parse_since;
use kestrl_core::services::attach::{self, AttachOptions, AttachSource};
use kestrl_core::services::buffer::{LogLine, Stream};
use kestrl_core::services::client::{self, Query};

use crate::context::Context;

/// Arguments for `kestrl logs`.
#[derive(Debug, Args)]
pub struct LogsArgs {
    /// Attach to an external log source
    #[command(subcommand)]
    pub action: Option<LogsAction>,
    /// Service name to tail
    pub service: Option<String>,
    /// Follow new log lines
    #[arg(short, long)]
    pub follow: bool,
    /// Only lines from the last 30s, 2m, 1h or 2d
    #[arg(long)]
    pub since: Option<String>,
    /// Filter log text with a regular expression
    #[arg(long)]
    pub grep: Option<String>,
    /// Show the last n matching lines
    #[arg(long)]
    pub limit: Option<usize>,
}

#[derive(Debug, Subcommand)]
pub enum LogsAction {
    /// Follow a log file, Docker container or systemd journal, here or over SSH
    Attach(AttachArgs),
}

/// Arguments for `kestrl logs attach`.
#[derive(Debug, Args)]
#[command(group(ArgGroup::new("source").required(true).multiple(false).args(["file", "docker", "journal"])))]
pub struct AttachArgs {
    /// Service name
    pub service: String,
    /// Follow a log file
    #[arg(long)]
    pub file: Option<PathBuf>,
    /// Follow Docker container logs
    #[arg(long)]
    pub docker: Option<String>,
    /// Follow a systemd unit's journal
    #[arg(long, value_name = "UNIT")]
    pub journal: Option<String>,
    /// Follow on this SSH host instead (a ~/.ssh/config alias or user@host); read-only, never prompts
    #[arg(long, value_name = "HOST")]
    pub ssh: Option<String>,
}

impl TryFrom<AttachArgs> for AttachOptions {
    type Error = anyhow::Error;

    fn try_from(args: AttachArgs) -> Result<Self, Self::Error> {
        let source = match (args.file, args.docker, args.journal) {
            (Some(path), None, None) => AttachSource::File(
                path.into_os_string()
                    .into_string()
                    .map_err(|_| anyhow::anyhow!("log file path is not valid UTF-8"))?,
            ),
            (None, Some(container), None) => AttachSource::Docker(container),
            (None, None, Some(unit)) => AttachSource::Journal(unit),
            _ => bail!("choose exactly one of --file, --docker or --journal"),
        };
        Ok(AttachOptions {
            source,
            ssh: args.ssh,
        })
    }
}

/// Tail a service's live buffer or attach to an external log source.
pub fn run(context: &Context, args: LogsArgs) -> anyhow::Result<()> {
    if let Some(LogsAction::Attach(arguments)) = args.action {
        let service = arguments.service.clone();
        let options = AttachOptions::try_from(arguments)?;
        attach::run(&service, &options)?;
        return Ok(());
    }

    let service = args
        .service
        .ok_or_else(|| anyhow::anyhow!("service required"))?;
    if let Some(pattern) = args.grep.as_ref() {
        regex::Regex::new(pattern).map_err(|source| kestrl_core::Error::InvalidGrep {
            pattern: pattern.clone(),
            source,
        })?;
    }
    let since_ms = args.since.as_deref().map(parse_since).transpose()?;
    let query = Query {
        since_ms,
        grep: args.grep,
        follow: args.follow,
        limit: args.limit,
    };

    let mut stdout = io::stdout().lock();
    client::read(&service, &query, |line| {
        write_log_line(&mut stdout, context.is_json, context.utc_offset, &line).map(|()| true)
    })?;
    Ok(())
}

fn write_log_line(
    stdout: &mut impl Write,
    is_json: bool,
    offset: time::UtcOffset,
    line: &LogLine,
) -> io::Result<()> {
    if is_json {
        writeln!(stdout, "{}", serde_json::to_string(line)?)
    } else {
        let clock = crate::commands::output::clock(line.ts_ms, offset);
        match line.stream {
            Stream::Stderr => writeln!(stdout, "{clock} [stderr] {}", line.text),
            Stream::Stdout => writeln!(stdout, "{clock}  {}", line.text),
        }
    }
}
