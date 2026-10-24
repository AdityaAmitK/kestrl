//! Kestrl core: shared data model and services used by the CLI and (later) the
//! SwiftUI app via UniFFI.

pub mod actor;
pub mod db;
pub mod duration;
pub mod env;
pub mod error;
pub mod history;
pub mod http;
pub mod profile;
pub mod request;
pub mod secrets;
pub mod services;
pub mod vars;
pub mod workspace;

pub use error::{Error, Result};
