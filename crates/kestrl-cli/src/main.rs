use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "kestrl", version, about = "Local backend workbench")]
struct Cli {
    /// Who is running this command (me, cursor, claude-code, codex, ...)
    #[arg(long, global = true)]
    actor: Option<String>,
    /// Force JSON output (default when stdout is not a TTY)
    #[arg(long, global = true)]
    json: bool,
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Send a request file or an ad-hoc request
    Send {
        target: String,
        #[arg(long, short)]
        env: Option<String>,
    },
    /// Run every request in a folder
    Run {
        dir: String,
        #[arg(long, short)]
        env: Option<String>,
    },
    /// List or show environments
    Envs { name: Option<String> },
    /// Manage Keychain secrets
    Secret {
        #[command(subcommand)]
        cmd: SecretCmd,
    },
    /// Search and inspect history
    History { query: Option<String> },
    /// Tag a history event
    Tag { event: String, tag: String },
}

#[derive(Subcommand)]
enum SecretCmd {
    Set { name: String },
    Rm { name: String },
    Ls,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let _actor = kestrl_core::actor::Actor::resolve(cli.actor.as_deref());
    anyhow::bail!("`{}` is not implemented yet", command_name(&cli.cmd))
}

fn command_name(cmd: &Cmd) -> &'static str {
    match cmd {
        Cmd::Send { .. } => "send",
        Cmd::Run { .. } => "run",
        Cmd::Envs { .. } => "envs",
        Cmd::Secret { .. } => "secret",
        Cmd::History { .. } => "history",
        Cmd::Tag { .. } => "tag",
    }
}
