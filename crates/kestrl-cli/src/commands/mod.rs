//! One module per subcommand. Each exposes `<Name>Args` and `run`.

pub mod capture;
pub mod db;
pub mod envs;
pub mod history;
pub mod logs;
pub mod output;
pub mod profiles;
pub mod proxy;
pub mod ps;
pub mod run;
pub mod secret;
pub mod send;
pub mod session;
pub mod tag;
pub mod up;

use clap::Args;
use clap::Subcommand;

use crate::context::Context;

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Send one request (file or METHOD URL)
    Send(send::SendArgs),
    /// Send every request in a collection, in order
    Run(run::RunArgs),
    /// List environments, or show one with resolved variables
    Envs(envs::EnvsArgs),
    /// List profiles, or show one
    Profiles(profiles::ProfilesArgs),
    /// Manage secrets in the macOS Keychain
    Secret(secret::SecretArgs),
    /// Capture traffic from a simulator, emulator or browser
    Capture(capture::CaptureArgs),
    /// Manage capture proxy trust and system settings
    Proxy(proxy::ProxyArgs),
    /// Search and inspect history
    History(history::HistoryArgs),
    /// Tag a history event
    Tag(tag::TagArgs),
    /// Start or end a session
    Session(session::SessionArgs),
    /// Query databases and browse tables
    Db(db::DbArgs),
    /// Start a service through Kestrl so its logs are collected
    Up(up::UpArgs),
    /// List running services and their ports
    Ps(ps::PsArgs),
    /// Tail or follow a service's live buffer
    Logs(logs::LogsArgs),
}

/// Flags shared by commands that resolve variables and profiles.
#[derive(Debug, Args)]
pub struct ResolveArgs {
    /// Environment name
    #[arg(long, short)]
    pub env: Option<String>,
    /// Profile name
    #[arg(long = "profile", short = 'p')]
    pub profiles: Vec<String>,
    /// Variable override, `key=value`; repeatable
    #[arg(long = "var")]
    pub vars: Vec<String>,
}

pub fn dispatch(ctx: &Context, command: Command) -> anyhow::Result<()> {
    match command {
        Command::Send(args) => send::run(ctx, args),
        Command::Run(args) => run::run(ctx, args),
        Command::Envs(args) => envs::run(ctx, args),
        Command::Profiles(args) => profiles::run(ctx, args),
        Command::Secret(args) => secret::run(ctx, args),
        Command::Capture(args) => capture::run(ctx, args),
        Command::Proxy(args) => proxy::run(args),
        Command::History(args) => history::run(ctx, args),
        Command::Tag(args) => tag::run(ctx, args),
        Command::Session(args) => session::run(ctx, args),
        Command::Db(args) => db::run(ctx, args),
        Command::Up(args) => up::run(ctx, args),
        Command::Ps(args) => ps::run(ctx, args),
        Command::Logs(args) => logs::run(ctx, args),
    }
}
