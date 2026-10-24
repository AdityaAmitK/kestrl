//! Unix socket server for live log queries.

use std::io::BufRead;
use std::io::BufReader;
use std::io::Write;
use std::os::unix::net::UnixListener;
use std::sync::Arc;
use std::sync::Condvar;
use std::sync::Mutex;
use std::thread;

use super::buffer::Buffer;
use crate::Error;

#[derive(serde::Deserialize)]
struct Request {
    since_ms: Option<i64>,
    grep: Option<String>,
    follow: bool,
    limit: Option<usize>,
}

/// Bind a socket and serve log queries for a shared buffer.
pub fn serve(
    path: std::path::PathBuf,
    buffer: Arc<(Mutex<Buffer>, Condvar)>,
) -> std::io::Result<()> {
    let _ = std::fs::remove_file(&path);
    let listener = UnixListener::bind(path)?;
    thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let buffer = buffer.clone();
            thread::spawn(move || {
                let Ok(mut socket) = stream.try_clone() else {
                    return;
                };
                let mut line = String::new();
                if BufReader::new(stream).read_line(&mut line).is_err() {
                    return;
                }
                let Ok(request) = serde_json::from_str::<Request>(&line) else {
                    return;
                };
                let regex = match request.grep {
                    Some(pattern) => match regex::Regex::new(&pattern) {
                        Ok(regex) => Some(regex),
                        Err(source) => {
                            let error = Error::InvalidGrep { pattern, source };
                            let response = serde_json::json!({"error": error.to_string()});
                            let _ = serde_json::to_writer(&mut socket, &response);
                            let _ = socket.write_all(b"\n");
                            return;
                        }
                    },
                    None => None,
                };
                let (lock, condition) = &*buffer;
                let mut last = 0;
                loop {
                    let Ok(guard) = lock.lock() else { return };
                    let mut items = guard.query(request.since_ms, regex.as_ref(), request.limit);
                    items.retain(|item| item.seq > last);
                    let latest = items.last().map(|item| item.seq);
                    for item in &items {
                        last = item.seq;
                        if serde_json::to_writer(&mut socket, item).is_err()
                            || socket.write_all(b"\n").is_err()
                        {
                            return;
                        }
                    }
                    if !request.follow {
                        return;
                    }
                    if let Some(latest) = latest {
                        last = latest;
                    } else if last == 0 {
                        last = guard.latest();
                    }
                    let Ok((guard, _)) =
                        condition.wait_timeout(guard, std::time::Duration::from_millis(500))
                    else {
                        return;
                    };
                    drop(guard);
                }
            });
        }
    });
    Ok(())
}
