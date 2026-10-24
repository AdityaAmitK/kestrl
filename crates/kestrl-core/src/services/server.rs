//! Unix socket server for live log queries.
use super::buffer::Buffer;
use std::{
    io::{BufRead, BufReader, Write},
    os::unix::net::UnixListener,
    sync::{Arc, Condvar, Mutex},
    thread,
};
#[derive(serde::Deserialize)]
struct Request {
    since_ms: Option<u128>,
    grep: Option<String>,
    follow: bool,
    limit: Option<usize>,
}
pub fn serve(path: std::path::PathBuf, b: Arc<(Mutex<Buffer>, Condvar)>) -> std::io::Result<()> {
    let _ = std::fs::remove_file(&path);
    let l = UnixListener::bind(path)?;
    thread::spawn(move || {
        for stream in l.incoming().flatten() {
            let b = b.clone();
            thread::spawn(move || {
                let Ok(mut s) = stream.try_clone() else {
                    return;
                };
                let mut line = String::new();
                if BufReader::new(stream).read_line(&mut line).is_err() {
                    return;
                }
                let Ok(r) = serde_json::from_str::<Request>(&line) else {
                    return;
                };
                let re = match r.grep {
                    Some(pattern) => match regex::Regex::new(&pattern) {
                        Ok(regex) => Some(regex),
                        Err(_) => return,
                    },
                    None => None,
                };
                let (lock, cv) = &*b;
                let mut last = 0;
                loop {
                    let Ok(g) = lock.lock() else { return };
                    let mut items = g.query(r.since_ms, re.as_ref(), r.limit);
                    items.retain(|x| x.seq > last);
                    let latest = items.last().map(|item| item.seq);
                    for item in &items {
                        last = item.seq;
                        if serde_json::to_writer(&mut s, item).is_err()
                            || s.write_all(b"\n").is_err()
                        {
                            return;
                        }
                    }
                    if !r.follow {
                        return;
                    }
                    if let Some(latest) = latest {
                        last = latest;
                    } else if last == 0 {
                        last = g.latest();
                    }
                    let Ok((guard, _)) = cv.wait_timeout(g, std::time::Duration::from_millis(500))
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
