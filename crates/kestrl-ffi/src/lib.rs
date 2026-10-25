//! UniFFI bindings over `kestrl-core` for the SwiftUI app. Every function is a
//! thin wrapper: it converts types and calls `kestrl-core`.

mod capture;
mod db;
mod envs;
mod error;
mod history;
mod profiles;
mod requests;
mod secrets;
mod services;
mod types;
mod workspace;

uniffi::setup_scaffolding!();

pub use capture::{
    BrowserCaptureOptions, CaptureBrowser, CaptureHandle, CaptureSink, CapturedExchange,
    DeviceCaptureOptions, DeviceTarget, PortCaptureOptions, PortCaptureSetup,
    android_capture_problem, port_capture_setup, simulator_capture_problem, start_browser_capture,
    start_device_capture, start_port_capture,
};
pub use db::{
    ColumnInfo, ConnectionRecord, QueryResultRecord, TableInfo, connection_kinds, db_connections,
    db_describe, db_query, db_tables, delete_connection, read_connection, save_connection,
};
pub use envs::{
    VariableInfo, create_environment, delete_environment, environment_variables,
    rename_environment, resolved_env, save_environment_variables, set_default_environment,
    validate_name, variable_catalog,
};
pub use error::KestrlError;
pub use history::{
    EventDetail, EventFilter, EventSummary, HttpExchange, diff_events, list_events, set_note,
    show_event, tag_event, untag_event,
};
pub use profiles::{
    ProfileRecord, create_profile, delete_profile, read_profile, rename_profile, save_profile,
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
pub use workspace::{
    RequestEntry, WorkspaceInfo, WorkspaceSettings, create_workspace, open_workspace,
};
