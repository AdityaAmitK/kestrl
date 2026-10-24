//! Client for a service's live log socket.

use std::io::BufRead;
use std::io::BufReader;
use std::io::Write;
use std::os::unix::net::UnixStream;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use super::buffer::LogLine;
use super::registry;
use crate::Error;
use crate::Result;

#[derive(serde::Deserialize)]
#[serde(untagged)]
enum Response {
    Log(LogLine),
    Error { error: String },
}

/// Filters and follow mode sent to a service log socket.
#[derive(serde::Serialize)]
pub struct Query {
    /// Include lines at or after this Unix millisecond timestamp.
    pub since_ms: Option<i64>,
    /// Optional regular expression applied to line text.
    pub grep: Option<String>,
    /// Keep the socket open for new lines.
    pub follow: bool,
    /// Maximum number of initial lines to return.
    pub limit: Option<usize>,
}

/// Read log lines until the server closes the socket or `emit` returns false.
pub fn read(
    name: &str,
    query: &Query,
    mut emit: impl FnMut(LogLine) -> std::io::Result<bool>,
) -> Result<()> {
    let socket_path = registry::dir()?.join(format!("{name}.sock"));
    let mut socket = UnixStream::connect(socket_path).map_err(|error| {
        if error.kind() == std::io::ErrorKind::ConnectionRefused
            || error.kind() == std::io::ErrorKind::NotFound
        {
            Error::ServiceNotRunning {
                name: name.to_owned(),
            }
        } else {
            Error::Io(error)
        }
    })?;
    serde_json::to_writer(&mut socket, query)?;
    socket.write_all(b"\n")?;
    for line in BufReader::new(socket).lines() {
        let line = line?;
        let response: Response = serde_json::from_str(&line)?;
        let log_line = match response {
            Response::Log(log_line) => log_line,
            Response::Error { error } => return Err(Error::InvalidRequest(error)),
        };
        if !emit(log_line)? {
            break;
        }
    }
    Ok(())
}

/// Read a live log stream until the socket closes or cancellation is requested.
pub fn follow(
    name: &str,
    grep: Option<String>,
    cancelled: Arc<AtomicBool>,
    mut emit: impl FnMut(LogLine),
) -> Result<()> {
    if let Some(pattern) = grep.as_deref() {
        regex::Regex::new(pattern).map_err(|source| Error::InvalidGrep {
            pattern: pattern.to_owned(),
            source,
        })?;
    }
    let socket_path = registry::dir()?.join(format!("{name}.sock"));
    let socket = UnixStream::connect(socket_path).map_err(|error| {
        if error.kind() == std::io::ErrorKind::ConnectionRefused
            || error.kind() == std::io::ErrorKind::NotFound
        {
            Error::ServiceNotRunning {
                name: name.to_owned(),
            }
        } else {
            Error::Io(error)
        }
    })?;
    socket.set_read_timeout(Some(std::time::Duration::from_millis(250)))?;
    let mut socket_writer = socket.try_clone()?;
    serde_json::to_writer(
        &mut socket_writer,
        &Query {
            since_ms: None,
            grep,
            follow: true,
            limit: None,
        },
    )?;
    socket_writer.write_all(b"\n")?;
    let mut reader = BufReader::new(socket);
    let mut line = String::new();
    loop {
        if cancelled.load(Ordering::Acquire) {
            return Ok(());
        }
        line.clear();
        match reader.read_line(&mut line) {
            Ok(0) => return Ok(()),
            Ok(_) => {
                let response: Response = serde_json::from_str(&line)?;
                match response {
                    Response::Log(log_line) => emit(log_line),
                    Response::Error { error } => return Err(Error::InvalidRequest(error)),
                }
            }
            Err(error)
                if error.kind() == std::io::ErrorKind::WouldBlock
                    || error.kind() == std::io::ErrorKind::TimedOut =>
            {
                continue;
            }
            Err(error) => return Err(Error::Io(error)),
        }
    }
}
