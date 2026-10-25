//! The one error type the app sees, mapped from `kestrl_core::Error`.

/// Each variant carries a message the app can show as is.
#[derive(Debug, uniffi::Error, thiserror::Error)]
pub enum KestrlError {
    /// A workspace, request, environment or profile file is missing or invalid
    #[error("{message}")]
    File { message: String },
    /// A request could not be built or sent
    #[error("{message}")]
    Request { message: String },
    /// A secret is missing, or the Keychain failed
    #[error("{message}")]
    Secret { message: String },
    /// The history database failed, or an event lookup did not match
    #[error("{message}")]
    History { message: String },
    /// An argument was not valid, such as a bad duration or HTTP method
    #[error("{message}")]
    InvalidInput { message: String },
}

impl From<kestrl_core::Error> for KestrlError {
    fn from(error: kestrl_core::Error) -> Self {
        use kestrl_core::Error;
        let message = error.to_string();
        match error {
            Error::WorkspaceNotFound(_)
            | Error::WorkspaceExists { .. }
            | Error::Write { .. }
            | Error::Read { .. }
            | Error::Toml { .. }
            | Error::TomlEdit { .. }
            | Error::Io(_) => Self::File { message },
            Error::InvalidRequest(_)
            | Error::UnknownVariable(_)
            | Error::VariableCycle(_)
            | Error::EnvironmentNotFound { .. }
            | Error::EnvironmentExists { .. }
            | Error::NameTaken { .. }
            | Error::NotFound { .. }
            | Error::DefaultEnvironmentInUse { .. }
            | Error::Http(_)
            | Error::ServiceNotRunning { .. }
            | Error::ServiceAlreadyRunning { .. } => Self::Request { message },
            Error::SecretNotSet { .. } | Error::Keychain(_) => Self::Secret { message },
            Error::InvalidMethod(_)
            | Error::InvalidDuration(_)
            | Error::UnknownConnection(_)
            | Error::ConnectionEnvironment { .. }
            | Error::UnsupportedConnectionKind(_)
            | Error::ReadOnlyConnection { .. }
            | Error::InvalidServiceName { .. }
            | Error::InvalidGrep { .. }
            | Error::InvalidName { .. } => Self::InvalidInput { message },
            Error::Capture(_) | Error::Database(_) => Self::Request { message },
            Error::Sqlite(_)
            | Error::Json(_)
            | Error::EventNotFound(_)
            | Error::AmbiguousId(_)
            | Error::NoOpenSession
            | Error::NoDataDir => Self::History { message },
        }
    }
}
