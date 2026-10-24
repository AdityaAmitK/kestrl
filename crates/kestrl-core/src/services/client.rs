//! Client for a service's live log socket.
use super::{buffer::LogLine, registry};
use std::{
    io::{BufRead, BufReader, Write},
    os::unix::net::UnixStream,
};
#[derive(serde::Serialize)]
pub struct Query {
    pub since_ms: Option<u128>,
    pub grep: Option<String>,
    pub follow: bool,
    pub limit: Option<usize>,
}
pub fn read(name: &str, q: &Query, mut emit: impl FnMut(LogLine) -> bool) -> std::io::Result<()> {
    let p = registry::dir()?.join(format!("{name}.sock"));
    let mut s = UnixStream::connect(p)?;
    serde_json::to_writer(&mut s, q)?;
    s.write_all(b"\n")?;
    for l in BufReader::new(s).lines() {
        let l = l?;
        if let Ok(v) = serde_json::from_str(&l)
            && !emit(v)
        {
            break;
        }
    }
    Ok(())
}
