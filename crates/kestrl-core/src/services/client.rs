//! Client for a service's live log socket.

use std::io::BufRead;
use std::io::BufReader;
use std::io::Write;
use std::os::unix::net::UnixStream;

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
