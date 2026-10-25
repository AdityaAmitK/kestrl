//! Plain-HTTP capture from a loopback TCP port.

use std::collections::{BTreeMap, HashMap, VecDeque};
use std::net::{Ipv4Addr, Ipv6Addr};
use std::path::Path;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crate::{Error, Result};

use super::{CapturedExchange, proxy::host_matches};

const MAX_FLOWS: usize = 16;
const MAX_STREAM_BYTES: usize = 1024 * 1024 + 64 * 1024;
const MAX_HEADER_BYTES: usize = 64 * 1024;
const MAX_BODY_BYTES: usize = 1024 * 1024;
const MAX_CAPTURE_BYTES: usize = 8 * 1024 * 1024;

type Callback = Arc<dyn Fn(CapturedExchange) + Send + Sync>;

/// Start loopback-only plain-HTTP capture on macOS `lo0`.
pub fn run(port: u16, hosts: Vec<String>, callback: Callback) -> Result<()> {
    run_cancellable(port, hosts, callback, Arc::new(AtomicBool::new(false)))
}

/// Validate a port number before the CLI announces capture startup.
pub fn validate_port(port: u16) -> Result<()> {
    if port == 0 {
        return Err(Error::Capture(
            "port must be between 1 and 65535".to_owned(),
        ));
    }
    Ok(())
}

/// Start capture that can be stopped by a session owner.
pub fn run_cancellable(
    port: u16,
    hosts: Vec<String>,
    callback: Callback,
    cancelled: Arc<AtomicBool>,
) -> Result<()> {
    validate_port(port)?;
    if !cfg!(target_os = "macos") {
        return Err(Error::Capture(
            "port capture is supported only on macOS lo0".to_owned(),
        ));
    }
    let device = pcap::Capture::from_device("lo0")
        .map_err(capture_error)?
        .promisc(false)
        .snaplen(65_535)
        .timeout(250)
        .open()
        .map_err(|error| Error::Capture(format!("cannot open macOS loopback packet capture: {error}. BPF access requires a privileged setup; run `kestrl capture port setup` for the current setup status.")))?;
    let mut device = device;
    device
        .filter(
            &format!("tcp and port {port} and (host 127.0.0.1 or host ::1)"),
            true,
        )
        .map_err(capture_error)?;
    let mut flows = HashMap::<FlowKey, Flow>::new();
    loop {
        if cancelled.load(Ordering::Relaxed) {
            return Ok(());
        }
        match device.next_packet() {
            Ok(packet) => {
                if let Some(segment) = decode_loopback(packet.data) {
                    if segment.source_port != port && segment.destination_port != port {
                        continue;
                    }
                    let key = FlowKey::new(
                        segment.source.clone(),
                        segment.source_port,
                        segment.destination.clone(),
                        segment.destination_port,
                    );
                    if !flows.contains_key(&key) && flows.len() >= MAX_FLOWS {
                        continue;
                    }
                    let flow = flows.entry(key.clone()).or_insert_with(|| Flow::new(&key));
                    flow.last_seen = Instant::now();
                    let exchanges = flow.accept(segment, port);
                    let flow_closed = flow.closed;
                    for exchange in exchanges.into_iter().filter(|exchange| {
                        hosts.is_empty()
                            || hosts.iter().any(|pattern| {
                                host_matches(
                                    pattern,
                                    host_name(
                                        exchange
                                            .url
                                            .split_once("://")
                                            .map_or(exchange.url.as_str(), |(_, rest)| {
                                                rest.split('/').next().unwrap_or(rest)
                                            }),
                                    ),
                                )
                            })
                    }) {
                        callback(exchange);
                    }
                    if flow_closed {
                        flows.remove(&key);
                    }
                }
            }
            Err(pcap::Error::TimeoutExpired) => {
                let cutoff = Instant::now() - Duration::from_secs(60);
                flows.retain(|_, flow| flow.last_seen >= cutoff);
            }
            Err(error) => {
                return Err(Error::Capture(format!(
                    "loopback packet capture failed: {error}"
                )));
            }
        }
    }
}

/// Installed copy of `scripts/kestrl-bpf.sh`, root-owned, at a fixed path.
const HELPER_PATH: &str = "/Library/Application Support/Kestrl/kestrl-bpf.sh";
/// LaunchDaemon that re-applies `kestrl_bpf` access to BPF devices at boot.
const LAUNCH_DAEMON_PATH: &str = "/Library/LaunchDaemons/dev.kestrl.bpf.plist";
const SETUP_COMMAND: &str = "sudo sh scripts/kestrl-bpf.sh install --user <your-username>";
const UNINSTALL_COMMAND: &str = "sudo sh scripts/kestrl-bpf.sh uninstall --user <your-username>";

/// Describe the one-time BPF setup without changing system state.
pub fn setup_status() -> String {
    format!(
        "Port capture reads loopback packets through macOS BPF devices, which are root-only by default. \
Run this once from a Kestrl checkout with administrator authorization. Kestrl does not run it for you:\n\n  {SETUP_COMMAND}\n\n\
The script creates the `kestrl_bpf` group, adds the named user, and installs a root-owned copy at `{HELPER_PATH}` with a LaunchDaemon that grants the group access to /dev/bpf* at each boot. \
It refuses to run while Wireshark's ChmodBPF is installed. \
Group members can read every packet on every interface through BPF, not only lo0. Kestrl's capture filter limits what Kestrl reads, not what the group can read. \
Log out and back in after setup. This command does not change system state."
    )
}

/// Report whether BPF access is available and whether the Kestrl setup is installed.
pub fn status() -> String {
    let accessible = std::fs::read_dir("/dev").ok().is_some_and(|entries| {
        entries.flatten().any(|entry| {
            entry.file_name().to_string_lossy().starts_with("bpf")
                && std::fs::OpenOptions::new()
                    .read(true)
                    .write(true)
                    .open(entry.path())
                    .is_ok()
        })
    });
    if accessible {
        "Port capture status: BPF device access is available to this process.".to_owned()
    } else if setup_installed() {
        "Port capture status: unavailable to this process. The Kestrl BPF setup is installed; if you were just added to `kestrl_bpf`, log out and back in. Run `kestrl capture port setup` to review the setup.".to_owned()
    } else {
        format!(
            "Port capture status: unavailable to this process. {}",
            setup_status()
        )
    }
}

/// Describe how to remove the Kestrl BPF setup, without changing system state.
pub fn uninstall() -> Result<String> {
    if !setup_installed() {
        return Ok("No Kestrl BPF setup is installed; nothing to remove.".to_owned());
    }
    Ok(format!(
        "The Kestrl BPF setup is installed. Remove it from the same checkout with administrator authorization. Kestrl does not run it for you:\n\n  {UNINSTALL_COMMAND}\n\n\
This removes your membership. When no members of `kestrl_bpf` remain, it also unloads the LaunchDaemon, removes the installed script and group, and restores the /dev/bpf* devices it assigned to root:wheel 0600. Other BPF access, including Wireshark's ChmodBPF, is not changed."
    ))
}

fn setup_installed() -> bool {
    Path::new(HELPER_PATH).exists() || Path::new(LAUNCH_DAEMON_PATH).exists()
}

fn capture_error(error: pcap::Error) -> Error {
    Error::Capture(error.to_string())
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct Endpoint {
    address: String,
    port: u16,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct FlowKey {
    first: Endpoint,
    second: Endpoint,
}

impl FlowKey {
    fn new(a: String, ap: u16, b: String, bp: u16) -> Self {
        let a = Endpoint {
            address: a,
            port: ap,
        };
        let b = Endpoint {
            address: b,
            port: bp,
        };
        if (a.address.as_str(), a.port) <= (b.address.as_str(), b.port) {
            Self {
                first: a,
                second: b,
            }
        } else {
            Self {
                first: b,
                second: a,
            }
        }
    }
}

#[derive(Debug)]
struct Segment {
    source: String,
    destination: String,
    source_port: u16,
    destination_port: u16,
    sequence: u32,
    payload: Vec<u8>,
    fin: bool,
    syn: bool,
    rst: bool,
}

fn decode_loopback(packet: &[u8]) -> Option<Segment> {
    if packet.len() < 4 {
        return None;
    }
    let family = u32::from_ne_bytes(packet.get(..4)?.try_into().ok()?);
    let (mut offset, protocol, source, destination, ip_end) = match family {
        2 => {
            let ip = packet.get(4..)?;
            if ip.len() < 20 || ip[0] >> 4 != 4 {
                return None;
            }
            let header = usize::from(ip[0] & 0x0f) * 4;
            if header < 20 || ip.len() < header || u16::from_be_bytes([ip[6], ip[7]]) & 0x3fff != 0
            {
                return None;
            }
            let source = Ipv4Addr::new(ip[12], ip[13], ip[14], ip[15]);
            let destination = Ipv4Addr::new(ip[16], ip[17], ip[18], ip[19]);
            if !source.is_loopback() || !destination.is_loopback() {
                return None;
            }
            (
                4 + header,
                ip[9],
                source.to_string(),
                destination.to_string(),
                4 + usize::from(u16::from_be_bytes([ip[2], ip[3]])),
            )
        }
        30 => {
            let ip = packet.get(4..)?;
            if ip.len() < 40 || ip[0] >> 4 != 6 {
                return None;
            }
            let source: [u8; 16] = ip[8..24].try_into().ok()?;
            let destination: [u8; 16] = ip[24..40].try_into().ok()?;
            let source = Ipv6Addr::from(source);
            let destination = Ipv6Addr::from(destination);
            if !source.is_loopback() || !destination.is_loopback() {
                return None;
            }
            let mut next_header = ip[6];
            let mut ip_offset = 40;
            while matches!(next_header, 0 | 43 | 60 | 135 | 51 | 44) {
                if next_header == 44 {
                    return None;
                }
                let extension = ip.get(ip_offset..)?;
                let extension_size = if next_header == 51 {
                    (usize::from(*extension.get(1)?) + 2) * 4
                } else {
                    (usize::from(*extension.get(1)?) + 1) * 8
                };
                next_header = *extension.first()?;
                ip_offset = ip_offset.checked_add(extension_size)?;
                if ip_offset > ip.len() {
                    return None;
                }
            }
            (
                4 + ip_offset,
                next_header,
                source.to_string(),
                destination.to_string(),
                4 + 40 + usize::from(u16::from_be_bytes([ip[4], ip[5]])),
            )
        }
        _ => return None,
    };
    if protocol != 6 {
        return None;
    }
    if ip_end > packet.len() || offset + 20 > ip_end {
        return None;
    }
    let tcp = &packet[offset..];
    let source_port = u16::from_be_bytes([tcp[0], tcp[1]]);
    let destination_port = u16::from_be_bytes([tcp[2], tcp[3]]);
    let sequence = u32::from_be_bytes(tcp[4..8].try_into().ok()?);
    let header = usize::from(tcp[12] >> 4) * 4;
    if header < 20 || offset + header > ip_end {
        return None;
    }
    offset += header;
    Some(Segment {
        source,
        destination,
        source_port,
        destination_port,
        sequence,
        payload: packet[offset..ip_end].to_vec(),
        fin: tcp[13] & 1 != 0,
        syn: tcp[13] & 2 != 0,
        rst: tcp[13] & 4 != 0,
    })
}

#[derive(Debug)]
struct Direction {
    next_sequence: Option<u32>,
    fin_sequence: Option<u32>,
    pending: BTreeMap<u32, Vec<u8>>,
    bytes: Vec<u8>,
    ended: bool,
    saw_syn: bool,
}

impl Direction {
    fn new() -> Self {
        Self {
            next_sequence: None,
            fin_sequence: None,
            pending: BTreeMap::new(),
            bytes: Vec::new(),
            ended: false,
            saw_syn: false,
        }
    }

    fn push(&mut self, sequence: u32, payload: &[u8], syn: bool, fin: bool) -> bool {
        self.saw_syn |= syn;
        let data_sequence = sequence.wrapping_add(u32::from(syn));
        if syn {
            self.next_sequence = Some(data_sequence);
        }
        if fin {
            self.fin_sequence = Some(data_sequence.wrapping_add(payload.len() as u32));
        }
        if !payload.is_empty() {
            if self.next_sequence.is_none() {
                if self.pending.len() < 128
                    && self
                        .pending
                        .values()
                        .map(Vec::len)
                        .sum::<usize>()
                        .saturating_add(payload.len())
                        <= MAX_STREAM_BYTES
                {
                    self.pending
                        .entry(data_sequence)
                        .or_insert_with(|| payload.to_vec());
                }
            } else {
                if let Some(next) = self.next_sequence {
                    let distance = data_sequence.wrapping_sub(next) as i32;
                    if distance < 0 {
                        let overlap = next.wrapping_sub(data_sequence) as usize;
                        if overlap >= payload.len() {
                            return true;
                        }
                        self.ended = true;
                        return false;
                    } else if distance == 0 {
                        self.bytes.extend_from_slice(payload);
                        self.next_sequence = Some(next.wrapping_add(payload.len() as u32));
                    } else if distance as usize <= MAX_STREAM_BYTES
                        && self.pending.len() < 128
                        && self
                            .pending
                            .values()
                            .map(Vec::len)
                            .sum::<usize>()
                            .saturating_add(payload.len())
                            <= MAX_STREAM_BYTES
                    {
                        self.pending
                            .entry(data_sequence)
                            .or_insert_with(|| payload.to_vec());
                    }
                }
            }
        }
        while let Some(next) = self.next_sequence {
            let Some(payload) = self.pending.remove(&next) else {
                break;
            };
            self.bytes.extend_from_slice(&payload);
            self.next_sequence = Some(next.wrapping_add(payload.len() as u32));
        }
        if self.bytes.len() > MAX_STREAM_BYTES {
            self.bytes.clear();
            self.pending.clear();
            self.ended = true;
            return false;
        }
        if self.fin_sequence.is_some() && self.fin_sequence == self.next_sequence {
            self.ended = true;
            self.next_sequence = self.next_sequence.map(|sequence| sequence.wrapping_add(1));
        }
        true
    }
}

#[derive(Debug)]
struct Request {
    method: String,
    target: String,
    host: String,
    headers: Vec<(String, String)>,
    body: Option<String>,
    started: Instant,
    ts: i64,
}

#[derive(Debug)]
struct Flow {
    first: Endpoint,
    directions: [Direction; 2],
    requests: VecDeque<Request>,
    last_seen: Instant,
    closed: bool,
}

impl Flow {
    fn new(key: &FlowKey) -> Self {
        Self {
            first: key.first.clone(),
            directions: [Direction::new(), Direction::new()],
            requests: VecDeque::new(),
            last_seen: Instant::now(),
            closed: false,
        }
    }

    fn accept(&mut self, segment: Segment, port: u16) -> Vec<CapturedExchange> {
        if segment.rst {
            self.closed = true;
            self.requests.clear();
            return Vec::new();
        }
        let request_direction = usize::from(self.first.port == port);
        let forward =
            segment.source == self.first.address && segment.source_port == self.first.port;
        let index = usize::from(!forward);
        let direction = &mut self.directions[index];
        if !direction.push(segment.sequence, &segment.payload, segment.syn, segment.fin) {
            self.closed = true;
            return Vec::new();
        }
        let mut exchanges = Vec::new();
        if direction.saw_syn && index == request_direction {
            while let Some(message) =
                parse_message(&mut direction.bytes, true, direction.ended, None)
            {
                let Some((method, target)) = parse_request_line(&message.start) else {
                    self.closed = true;
                    break;
                };
                if method.eq_ignore_ascii_case("CONNECT")
                    || message
                        .headers
                        .iter()
                        .any(|(n, _)| n.eq_ignore_ascii_case("upgrade"))
                {
                    self.closed = true;
                    break;
                }
                let host = message
                    .headers
                    .iter()
                    .find(|(name, _)| name.eq_ignore_ascii_case("host"))
                    .map(|(_, value)| value.clone())
                    .unwrap_or_else(|| format!("127.0.0.1:{port}"));
                self.requests.push_back(Request {
                    method,
                    target,
                    host,
                    headers: message.headers,
                    body: message.body,
                    started: Instant::now(),
                    ts: now_ms(),
                });
            }
        } else if index == request_direction {
        } else if direction.saw_syn {
            while let Some(method) = self.requests.front().map(|r| r.method.as_str()) {
                let Some(message) =
                    parse_message(&mut direction.bytes, false, direction.ended, Some(method))
                else {
                    break;
                };
                let Some(status_code) = parse_status_line(&message.start) else {
                    self.closed = true;
                    break;
                };
                if (100..200).contains(&status_code) && status_code != 101 {
                    continue;
                }
                if status_code == 101 {
                    self.closed = true;
                    break;
                }
                let Some(request) = self.requests.pop_front() else {
                    break;
                };
                let target =
                    if request.target.starts_with('/') || request.target.starts_with("http://") {
                        request.target.clone()
                    } else {
                        self.closed = true;
                        break;
                    };
                let url = if target.starts_with("http://") {
                    target
                } else {
                    format!("http://{}{}", request.host, target)
                };
                exchanges.push(CapturedExchange {
                    method: request.method,
                    url,
                    req_headers: request.headers,
                    req_body: request.body,
                    status: Some(status_code),
                    res_headers: message.headers,
                    res_body: message.body,
                    duration_ms: request.started.elapsed().as_millis() as u64,
                    ts: request.ts,
                    tunneled: false,
                });
            }
        }
        let _ = direction;
        self.account_bytes();
        self.closed |= self.directions.iter().all(|direction| direction.ended);
        exchanges
    }

    fn account_bytes(&mut self) {
        let total: usize = self
            .directions
            .iter()
            .map(|d| d.bytes.len() + d.pending.values().map(Vec::len).sum::<usize>())
            .sum::<usize>()
            + self
                .requests
                .iter()
                .map(|r| r.body.as_ref().map_or(0, String::len))
                .sum::<usize>();
        if total > MAX_CAPTURE_BYTES {
            self.closed = true;
            self.requests.clear();
            for d in &mut self.directions {
                d.bytes.clear();
                d.pending.clear();
            }
        }
    }
}

fn parse_request_line(line: &str) -> Option<(String, String)> {
    let mut p = line.split(' ');
    let method = p.next()?;
    let target = p.next()?;
    let version = p.next()?;
    if method.is_empty()
        || target.is_empty()
        || p.next().is_some()
        || !version.starts_with("HTTP/1.")
        || !matches!(version, "HTTP/1.0" | "HTTP/1.1")
    {
        return None;
    }
    Some((method.to_owned(), target.to_owned()))
}
fn parse_status_line(line: &str) -> Option<u16> {
    let mut p = line.splitn(3, ' ');
    if !matches!(p.next()?, "HTTP/1.0" | "HTTP/1.1") {
        return None;
    }
    let n = p.next()?;
    if n.len() != 3 || !n.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    n.parse().ok()
}

#[derive(Debug)]
struct Message {
    start: String,
    headers: Vec<(String, String)>,
    body: Option<String>,
}

fn parse_message(
    bytes: &mut Vec<u8>,
    request: bool,
    closed: bool,
    request_method: Option<&str>,
) -> Option<Message> {
    let header_end = bytes.windows(4).position(|window| window == b"\r\n\r\n")?;
    if header_end > MAX_HEADER_BYTES {
        bytes.clear();
        return None;
    }
    let head = std::str::from_utf8(&bytes[..header_end]).ok()?;
    let mut lines = head.split("\r\n");
    let start = lines.next()?.to_owned();
    let mut headers = Vec::new();
    for line in lines {
        let (name, value) = line.split_once(':')?;
        headers.push((name.trim().to_owned(), value.trim().to_owned()));
    }
    if headers
        .iter()
        .any(|(name, _)| name.is_empty() || name.bytes().any(|b| !(33..=126).contains(&b)))
    {
        bytes.clear();
        return None;
    }
    let chunked_headers: Vec<_> = headers
        .iter()
        .filter(|(name, _)| name.eq_ignore_ascii_case("transfer-encoding"))
        .collect();
    let length_headers: Vec<_> = headers
        .iter()
        .filter(|(name, _)| name.eq_ignore_ascii_case("content-length"))
        .collect();
    if chunked_headers.len() > 1
        || length_headers.len() > 1
        || (!chunked_headers.is_empty() && !length_headers.is_empty())
    {
        bytes.clear();
        return None;
    }
    let chunked = headers.iter().any(|(name, value)| {
        name.eq_ignore_ascii_case("transfer-encoding")
            && value
                .to_ascii_lowercase()
                .split(',')
                .any(|encoding| encoding.trim() == "chunked")
    });
    let length = headers
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case("content-length"))
        .and_then(|(_, value)| value.parse::<usize>().ok());
    if !length_headers.is_empty() && length.is_none() {
        bytes.clear();
        return None;
    }
    let body_start = header_end + 4;
    let no_body = !request && (request_method == Some("HEAD") || status_has_no_body(&start));
    let body_end = if no_body {
        body_start
    } else if chunked {
        chunked_end(&bytes[body_start..])?.map(|end| body_start + end)?
    } else if let Some(length) = length {
        if length > MAX_BODY_BYTES || bytes.len() < body_start.saturating_add(length) {
            return None;
        }
        body_start + length
    } else if !request && request_method != Some("HEAD") && !status_has_no_body(&start) {
        if !closed {
            return None;
        }
        bytes.len()
    } else {
        body_start
    };
    if body_end > bytes.len() || body_end - body_start > MAX_BODY_BYTES {
        return None;
    }
    let raw = bytes[body_start..body_end].to_vec();
    let consumed = body_end;
    if consumed > bytes.len() {
        return None;
    }
    bytes.drain(..consumed);
    let decoded = if raw.is_empty() {
        Some(Vec::new())
    } else if chunked {
        decode_chunked(&raw)
    } else {
        Some(raw)
    };
    let encoding = headers
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case("content-encoding"))
        .map(|(_, value)| value.as_str());
    let body = decoded.and_then(|body| super::proxy::display_body(&body, encoding));
    Some(Message {
        start,
        headers,
        body,
    })
}

fn host_name(authority: &str) -> &str {
    if let Some(rest) = authority.strip_prefix('[') {
        return rest.split_once(']').map_or(authority, |(host, _)| host);
    }
    if let Some((host, port)) = authority.rsplit_once(':')
        && port.parse::<u16>().is_ok()
    {
        return host;
    }
    authority
}

fn status_has_no_body(start: &str) -> bool {
    start
        .split_whitespace()
        .nth(1)
        .and_then(|status| status.parse::<u16>().ok())
        .is_some_and(|status| (100..200).contains(&status) || status == 204 || status == 304)
}

fn chunked_end(bytes: &[u8]) -> Option<Option<usize>> {
    let mut offset = 0;
    loop {
        let line_end = bytes[offset..]
            .windows(2)
            .position(|window| window == b"\r\n")?
            + offset;
        let size = std::str::from_utf8(&bytes[offset..line_end])
            .ok()?
            .split(';')
            .next()?
            .trim();
        let size = usize::from_str_radix(size, 16).ok()?;
        offset = line_end + 2;
        if size == 0 {
            loop {
                let line_start = offset;
                let trailer_end = bytes[offset..]
                    .windows(2)
                    .position(|window| window == b"\r\n")?
                    + offset;
                offset = trailer_end + 2;
                if trailer_end == line_start {
                    return Some(Some(offset));
                }
            }
        }
        offset = offset.checked_add(size)?;
        if bytes.get(offset..offset + 2)? != b"\r\n" {
            return None;
        }
        offset += 2;
        if offset > MAX_BODY_BYTES {
            return None;
        }
    }
}

fn decode_chunked(bytes: &[u8]) -> Option<Vec<u8>> {
    let end = chunked_end(bytes)??;
    let mut offset = 0;
    let mut body = Vec::new();
    while offset < end {
        let line_end = bytes[offset..]
            .windows(2)
            .position(|window| window == b"\r\n")?
            + offset;
        let size = usize::from_str_radix(
            std::str::from_utf8(&bytes[offset..line_end])
                .ok()?
                .split(';')
                .next()?
                .trim(),
            16,
        )
        .ok()?;
        offset = line_end + 2;
        if size == 0 {
            loop {
                let line_start = offset;
                let trailer_end = bytes[offset..]
                    .windows(2)
                    .position(|window| window == b"\r\n")?
                    + offset;
                offset = trailer_end + 2;
                if trailer_end == line_start {
                    return Some(body);
                }
            }
        }
        body.extend_from_slice(bytes.get(offset..offset.checked_add(size)?)?);
        offset += size + 2;
    }
    Some(body)
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or_default()
}
