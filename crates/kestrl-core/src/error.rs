//! Error type shared by all core modules.

use std::path::PathBuf;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("workspace not found from {0}")]
    WorkspaceNotFound(PathBuf),
    #[error("failed to read {path}")]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("invalid TOML in {path}")]
    Toml {
        path: PathBuf,
        #[source]
        source: toml::de::Error,
    },
    #[error("invalid request: {0}")]
    InvalidRequest(String),
    #[error("unknown variable: {0}")]
    UnknownVariable(String),
    #[error("environment \"{name}\" not found; available environments: {available}")]
    EnvironmentNotFound { name: String, available: String },
    #[error("secret \"{name}\" is not set — run: kestrl secret set {name}")]
    SecretNotSet { name: String },
    #[error("variable expansion cycle or depth limit at {0}")]
    VariableCycle(String),
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),
    #[error("invalid HTTP method: {0}")]
    InvalidMethod(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Keychain(#[from] keyring::Error),
    #[error(transparent)]
    Sqlite(#[from] rusqlite::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error("no history event starts with {0:?}")]
    EventNotFound(String),
    #[error("more than one history event starts with {0:?}; use a longer prefix")]
    AmbiguousId(String),
    #[error("no session is open")]
    NoOpenSession,
    #[error("could not locate the application data directory")]
    NoDataDir,
    #[error("capture failed: {0}")]
    Capture(String),
    #[error("unknown connection \"{0}\"")]
    UnknownConnection(String),
    #[error("connection \"{connection}\" has no entry for env \"{env}\"")]
    ConnectionEnvironment { connection: String, env: String },
    #[error("unsupported database connection kind: {0}")]
    UnsupportedConnectionKind(String),
    #[error(
        "connection \"{connection}\" is read-only in env \"{env}\"; add it to allow_writes to permit writes"
    )]
    ReadOnlyConnection { connection: String, env: String },
    #[error("database query failed: {0}")]
    Database(String),
    /// The requested service has no live log socket.
    #[error("service \"{name}\" is not running — start it with: kestrl up {name} -- <cmd>")]
    ServiceNotRunning { name: String },
    /// A service with this name already has a live log socket.
    #[error("service \"{name}\" is already running")]
    ServiceAlreadyRunning { name: String },
    /// The service name or command is missing or invalid.
    #[error("invalid service name: {name:?}")]
    InvalidServiceName { name: String },
    /// A `--grep` pattern could not be compiled.
    #[error("invalid --grep pattern {pattern:?}: {source}")]
    InvalidGrep {
        pattern: String,
        #[source]
        source: regex::Error,
    },
    #[error("invalid duration {0:?}; use a number followed by s, m, h or d, e.g. 30s, 2m, 1h, 2d")]
    InvalidDuration(String),
}
