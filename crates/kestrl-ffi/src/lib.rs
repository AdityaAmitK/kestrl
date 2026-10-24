//! UniFFI bindings over `kestrl-core` for the SwiftUI app. Every function is a
//! thin wrapper: it converts types and calls `kestrl-core`.

mod envs;
mod error;
mod history;
mod requests;
mod secrets;
mod types;
mod workspace;

uniffi::setup_scaffolding!();

pub use envs::resolved_env;
pub use error::KestrlError;
pub use history::{
    EventDetail, EventFilter, EventSummary, HttpExchange, diff_events, list_events, set_note,
    show_event, tag_event, untag_event,
};
pub use requests::{SentExchange, send_ad_hoc, send_request_file};
pub use secrets::{SecretStatus, delete_secret, secret_status, set_secret};
pub use types::KeyValue;
pub use workspace::{WorkspaceInfo, open_workspace};
