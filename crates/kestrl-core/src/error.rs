//! Error type shared by all core modules.

use std::path::PathBuf;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
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
}
