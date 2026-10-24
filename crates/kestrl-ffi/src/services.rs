//! Service listing and log access for the app.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;

use kestrl_core::services::{client, registry};

use crate::error::KestrlError;

#[derive(Debug, uniffi::Record)]
pub struct ServiceInfo {
    pub name: String,
    pub pid: u32,
    pub ports: Vec<u16>,
    pub command: String,
    pub cwd: String,
    pub started_at_ms: i64,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct LogLine {
    pub seq: u64,
    pub ts_ms: i64,
    pub stream: String,
    pub text: String,
}

#[uniffi::export(with_foreign)]
pub trait LogSink: Send + Sync {
    fn on_line(&self, line: LogLine);
    fn on_end(&self, error: Option<String>);
}

#[derive(uniffi::Object)]
pub struct LogFollow {
    cancelled: Arc<AtomicBool>,
}

#[uniffi::export]
impl LogFollow {
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Release);
    }
}

#[uniffi::export]
pub fn list_services() -> Result<Vec<ServiceInfo>, KestrlError> {
    registry::list()
        .map(|services| {
            services
                .into_iter()
                .map(|service| ServiceInfo {
                    name: service.name,
                    pid: service.pid,
                    ports: service.ports,
                    command: service.command,
                    cwd: service.cwd,
                    started_at_ms: service.started_at,
                })
                .collect()
        })
        .map_err(|error| KestrlError::File {
            message: error.to_string(),
        })
}

#[uniffi::export]
pub fn read_logs(
    service: String,
    since_ms: Option<i64>,
    grep: Option<String>,
    limit: Option<u32>,
) -> Result<Vec<LogLine>, KestrlError> {
    let mut lines = Vec::new();
    client::read(
        &service,
        &client::Query {
            since_ms,
            grep,
            follow: false,
            limit: limit.map(|limit| limit as usize),
        },
        |line| {
            lines.push(convert_line(line));
            Ok(true)
        },
    )?;
    Ok(lines)
}

#[uniffi::export]
pub fn follow_logs(
    service: String,
    grep: Option<String>,
    sink: Arc<dyn LogSink>,
) -> Arc<LogFollow> {
    let cancelled = Arc::new(AtomicBool::new(false));
    let handle = Arc::new(LogFollow {
        cancelled: Arc::clone(&cancelled),
    });
    thread::spawn(move || {
        let result = client::follow(&service, grep, cancelled, |line| {
            sink.on_line(convert_line(line))
        });
        sink.on_end(result.err().map(|error| error.to_string()));
    });
    handle
}

fn convert_line(line: kestrl_core::services::buffer::LogLine) -> LogLine {
    LogLine {
        seq: line.seq,
        ts_ms: line.ts_ms,
        stream: match line.stream {
            kestrl_core::services::buffer::Stream::Stdout => "stdout".to_owned(),
            kestrl_core::services::buffer::Stream::Stderr => "stderr".to_owned(),
        },
        text: line.text,
    }
}
