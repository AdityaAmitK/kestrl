//! Kestrl core: shared data model and services used by the CLI and (later) the
//! SwiftUI app via UniFFI.

pub mod actor;
pub mod capture;
pub mod collections;
pub mod connection_files;
pub mod db;
pub mod duration;
pub mod env;
pub mod env_files;
pub mod error;
pub mod history;
pub mod http;
pub mod profile;
pub mod profile_files;
pub mod request;
pub mod secrets;
pub mod send;
pub mod services;
pub mod toml_files;
pub mod vars;
pub mod workspace;

pub use error::{Error, Result};
