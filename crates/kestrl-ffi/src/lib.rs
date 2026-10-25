//! UniFFI bindings over `kestrl-core` for the SwiftUI app. Every function is a
//! thin wrapper: it converts types and calls `kestrl-core`.

mod db;
mod envs;
mod error;
mod history;
mod requests;
mod secrets;
mod services;
mod types;
mod workspace;

uniffi::setup_scaffolding!();

pub use db::{
    ColumnInfo, QueryResultRecord, TableInfo, db_connections, db_describe, db_query, db_tables,
};
pub use envs::resolved_env;
pub use error::KestrlError;
pub use history::{
    EventDetail, EventFilter, EventSummary, HttpExchange, diff_events, list_events, set_note,
    show_event, tag_event, untag_event,
};
pub use requests::{
    RequestDetail, SentExchange, read_request_file, save_request_draft, send_ad_hoc,
    send_request_draft, send_request_file,
};
pub use secrets::{SecretStatus, delete_secret, secret_status, set_secret};
pub use services::{
    LogFollow, LogLine, LogSink, ServiceInfo, follow_logs, list_services, read_logs,
};
pub use types::KeyValue;
pub use workspace::{RequestEntry, WorkspaceInfo, WorkspaceSettings, open_workspace};
