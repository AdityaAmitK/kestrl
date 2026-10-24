//! Tail or follow a service's live buffer.
use clap::{Args, Subcommand};
#[derive(Debug, Args)]
pub struct LogsArgs {
    /// Attach to a file or Docker container
    #[command(subcommand)]
    pub action: Option<LogsAction>,
    /// Service name to tail
    #[arg()]
    pub service: Option<String>,
    /// Follow new log lines
    #[arg(short, long)]
    pub follow: bool,
    /// Show logs from the last duration (e.g. 2m)
    #[arg(long)]
    pub since: Option<String>,
    /// Filter log text with a regular expression
    #[arg(long)]
    pub grep: Option<String>,
    /// Maximum number of lines to return
    #[arg(long)]
    pub limit: Option<usize>,
}
#[derive(Debug, Subcommand)]
pub enum LogsAction {
    /// Attach to a log file or Docker container
    Attach(AttachArgs),
}
#[derive(Debug, Args)]
pub struct AttachArgs {
    /// Service name
    pub service: String,
    /// Follow a log file
    #[arg(long, conflicts_with = "docker", required_unless_present = "docker")]
    pub file: Option<String>,
    /// Follow Docker container logs
    #[arg(long, required_unless_present = "file")]
    pub docker: Option<String>,
}
#[allow(clippy::print_stdout)]
pub fn run(ctx: &crate::context::Context, a: LogsArgs) -> anyhow::Result<()> {
    if let Some(LogsAction::Attach(x)) = a.action {
        let cmd = if let Some(f) = x.file {
            vec!["tail".into(), "-F".into(), f]
        } else {
            vec![
                "docker".into(),
                "logs".into(),
                "-f".into(),
                "--timestamps=false".into(),
                x.docker.unwrap_or_default(),
            ]
        };
        kestrl_core::services::supervisor::run(&x.service, &cmd)?;
        return Ok(());
    }
    let s = a
        .service
        .ok_or_else(|| anyhow::anyhow!("service required"))?;
    let since = a
        .since
        .as_deref()
        .map(|x| {
            let (n, u) = x.split_at(x.find(|c: char| !c.is_ascii_digit()).unwrap_or(x.len()));
            let amount = n.parse::<u128>()?;
            let multiplier = match u {
                "s" => 1000,
                "m" => 60_000,
                "h" => 3_600_000,
                _ => anyhow::bail!("invalid duration '{x}'; use a number followed by s, m, or h"),
            };
            amount
                .checked_mul(multiplier)
                .ok_or_else(|| anyhow::anyhow!("duration is too large"))
        })
        .transpose()?
        .map(|milliseconds| {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis()
                .saturating_sub(milliseconds)
        });
    let q = kestrl_core::services::client::Query {
        since_ms: since,
        grep: a.grep,
        follow: a.follow,
        limit: a.limit,
    };
    kestrl_core::services::client::read(&s, &q, |l| {
        if ctx.is_json {
            println!("{}", serde_json::to_string(&l).unwrap_or_default())
        } else {
            let seconds = l.ts_ms / 1000;
            let millis = l.ts_ms % 1000;
            let clock = format!(
                "{:02}:{:02}:{:02}.{:03}",
                (seconds / 3600) % 24,
                (seconds / 60) % 60,
                seconds % 60,
                millis
            );
            if l.stream == "stderr" {
                println!("{clock} [stderr] {}", l.text)
            } else {
                println!("{clock}  {}", l.text)
            }
        }
        true
    })?;
    Ok(())
}
