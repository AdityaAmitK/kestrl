//! Memory-only service processes and their live log buffers.

pub mod attach;
pub mod buffer;
pub mod client;
pub mod ports;
pub mod registry;
pub mod server;
pub mod supervisor;

pub use client::follow as follow_logs;
pub use client::{Query as LogQuery, read as read_logs};
pub use registry::{Service, list as list_services};
