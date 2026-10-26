//! Share: expose one local HTTP or HTTPS port through a Cloudflare tunnel for a bounded time.
//!
//! Each share is owned by one process that runs `cloudflared` and records the share in
//! `shares/<id>.json` in the data directory, so the CLI, the app and agents see the same shares.
//! The owner tears the tunnel down when the share expires or is closed. Kestrl never edits the
//! user's own `cloudflared` configuration; it writes a separate configuration per share.

use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use crate::capture::state::write_private;
use crate::history::now_ms;
use crate::{Error, Result};

/// How long a share stays open when no lifetime is given.
pub const DEFAULT_TTL: Duration = Duration::from_secs(60 * 60);
/// The longest lifetime a share may have.
pub const MAX_TTL: Duration = Duration::from_secs(24 * 60 * 60);
const MIN_TTL: Duration = Duration::from_secs(60);
/// The tunnel named hostnames use when none is given.
pub const DEFAULT_TUNNEL: &str = "kestrl";
/// How long opening may take before the share is abandoned.
pub const START_TIMEOUT: Duration = Duration::from_secs(60);
const POLL: Duration = Duration::from_millis(200);

/// Who may reach a share. There is no default: every share states its access.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Access {
    /// Anyone with the URL.
    Public,
    /// Only people Cloudflare Access lets in; Kestrl checks this before the tunnel starts.
    Protected,
}

/// Where a share is in its life.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ShareState {
    Starting,
    Ready,
    Failed,
}

/// What to share and for how long.
#[derive(Clone, Debug)]
pub struct ShareOptions {
    pub port: u16,
    /// The local server speaks HTTPS (certificates are not verified, as dev servers self-sign).
    pub is_https: bool,
    pub access: Access,
    /// A named hostname in the user's Cloudflare zone; `None` opens a temporary
    /// `trycloudflare.com` URL.
    pub hostname: Option<String>,
    /// The named tunnel that serves `hostname`; `DEFAULT_TUNNEL` when `None`.
    pub tunnel: Option<String>,
    pub ttl: Duration,
}

/// One share, as recorded in the data directory.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Share {
    pub id: String,
    pub port: u16,
    pub is_https: bool,
    pub access: Access,
    pub hostname: Option<String>,
    pub tunnel: Option<String>,
    /// The public URL, once the tunnel is connected.
    pub url: Option<String>,
    pub state: ShareState,
    /// Why the share failed.
    pub error: Option<String>,
    /// The owner process; 0 until it starts.
    pub pid: u32,
    pub created_at: i64,
    pub expires_at: i64,
}

/// Validate `options` and record a new share for an owner process to start with `serve`.
pub fn create(options: &ShareOptions) -> Result<Share> {
    let invalid = |message: &str| Err(Error::Share(message.to_owned()));
    if options.port == 0 {
        return invalid("port must be between 1 and 65535");
    }
    if options.ttl < MIN_TTL || options.ttl > MAX_TTL {
        return invalid("lifetime must be between 1m and 24h");
    }
    if options.hostname.is_none() && options.tunnel.is_some() {
        return invalid("--tunnel needs --hostname");
    }
    if options.hostname.is_none() && options.access == Access::Protected {
        return invalid(
            "a temporary trycloudflare.com URL is always public; protected shares need --hostname covered by Cloudflare Access",
        );
    }
    let tunnel = options.hostname.as_ref().map(|_| {
        options
            .tunnel
            .as_deref()
            .unwrap_or(DEFAULT_TUNNEL)
            .to_owned()
    });
    if let Some(hostname) = &options.hostname
        && !is_hostname(hostname)
    {
        return Err(Error::Share(format!("invalid hostname {hostname:?}")));
    }
    let mut _reservation = None;
    if let Some(tunnel) = &tunnel {
        if tunnel.is_empty()
            || !tunnel
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        {
            return Err(Error::Share(format!("invalid tunnel name {tunnel:?}")));
        }
        // Every connector of a tunnel gets a share of its traffic, so one share per tunnel. The lock
        // makes checking and reserving one step, so two concurrent opens cannot both pass.
        let lock = std::fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(dir()?.join("create.lock"))?;
        lock.lock()?;
        _reservation = Some(lock);
        if let Some(open) = list()?
            .into_iter()
            .find(|share| share.tunnel.as_ref() == Some(tunnel))
        {
            return Err(Error::Share(format!(
                "tunnel {tunnel:?} already serves share {}; close it first",
                open.id
            )));
        }
    }
    let created_at = now_ms();
    let ttl_ms = i64::try_from(options.ttl.as_millis()).unwrap_or(i64::MAX);
    let ulid = ulid::Ulid::new().to_string().to_ascii_lowercase();
    let share = Share {
        id: ulid[ulid.len() - 8..].to_owned(),
        port: options.port,
        is_https: options.is_https,
        access: options.access,
        hostname: options.hostname.clone(),
        tunnel,
        url: None,
        state: ShareState::Starting,
        error: None,
        pid: 0,
        created_at,
        expires_at: created_at.saturating_add(ttl_ms),
    };
    save(&share)?;
    Ok(share)
}

/// Every open share. Records whose owner has exited are removed.
pub fn list() -> Result<Vec<Share>> {
    let mut shares = Vec::new();
    for entry in std::fs::read_dir(dir()?)? {
        let path = entry?.path();
        if path.extension().is_none_or(|extension| extension != "json") {
            continue;
        }
        let Ok(share) = serde_json::from_slice::<Share>(&std::fs::read(&path)?) else {
            continue;
        };
        let is_stale = if share.pid == 0 {
            now_ms() - share.created_at > duration_ms(START_TIMEOUT)
        } else {
            !is_owner(share.pid, &share.id)
        };
        if is_stale {
            remove(&share.id);
        } else {
            shares.push(share);
        }
    }
    shares.sort_by_key(|share| share.created_at);
    Ok(shares)
}

/// One open share.
pub fn get(id: &str) -> Result<Share> {
    list()?
        .into_iter()
        .find(|share| share.id == id)
        .ok_or_else(|| Error::NotFound {
            kind: "share",
            name: id.to_owned(),
        })
}

/// Close a share and wait until its tunnel is gone.
pub fn close(id: &str) -> Result<()> {
    let share = get(id)?;
    if share.pid == 0 {
        remove(id);
        return Ok(());
    }
    let _ = signal("-TERM", &share.pid.to_string());
    let deadline = Instant::now() + Duration::from_secs(10);
    while record(id).exists() {
        if Instant::now() >= deadline {
            // The owner leads its own process group, which includes `cloudflared`. Check it is
            // still this share's owner, so a reused process ID is never signalled.
            if is_owner(share.pid, id) {
                let _ = signal("-KILL", &format!("-{}", share.pid));
            }
            remove(id);
            break;
        }
        std::thread::sleep(POLL);
    }
    Ok(())
}

/// Wait until a share started by `serve` is ready, and return it. A failed share is removed.
pub fn wait_ready(id: &str) -> Result<Share> {
    let deadline = Instant::now() + START_TIMEOUT + Duration::from_secs(10);
    loop {
        let Ok(bytes) = std::fs::read(record(id)) else {
            return Err(Error::Share(
                "the share process exited before the tunnel was ready".to_owned(),
            ));
        };
        let share: Share = serde_json::from_slice(&bytes)?;
        match share.state {
            ShareState::Ready => return Ok(share),
            ShareState::Failed => {
                remove(id);
                return Err(Error::Share(share.error.unwrap_or_default()));
            }
            ShareState::Starting if Instant::now() >= deadline => {
                let _ = close(id);
                return Err(Error::Share("timed out opening the tunnel".to_owned()));
            }
            ShareState::Starting => std::thread::sleep(POLL),
        }
    }
}

/// Own a share recorded by `create`: start the tunnel, keep it up until it expires, is closed or
/// fails, then tear it down. Blocks; run it in a separate process.
pub fn serve(id: &str) -> Result<()> {
    let mut share: Share = serde_json::from_slice(&std::fs::read(record(id))?)?;
    share.pid = std::process::id();
    save(&share)?;
    let stop = Arc::new(AtomicBool::new(false));
    let flag = stop.clone();
    ctrlc::set_handler(move || flag.store(true, Ordering::SeqCst))
        .map_err(|error| Error::Share(error.to_string()))?;
    let config = dir()?.join(format!("{id}.cloudflared.yml"));
    let result = run(&mut share, &config, &stop);
    let _ = std::fs::remove_file(&config);
    match &result {
        // Kept until `wait_ready` reads it; `list` removes it once this process is gone.
        Err(error) if share.state == ShareState::Starting => {
            share.state = ShareState::Failed;
            share.error = Some(match error {
                Error::Share(message) => message.clone(),
                other => other.to_string(),
            });
            let _ = save(&share);
        }
        _ => remove(id),
    }
    result
}

fn run(share: &mut Share, config: &std::path::Path, stop: &AtomicBool) -> Result<()> {
    let origin = format!(
        "{}://localhost:{}",
        if share.is_https { "https" } else { "http" },
        share.port
    );
    write_private(config, config_text(share, &origin).as_bytes())?;
    // `cloudflared` runs under a shell that stops it when this owner goes away, even if the owner is
    // killed: the owner holds the shell's stdin open and never writes to it.
    let mut command = Command::new("sh");
    command.args(["-c", OWNER_WATCH, "kestrl-share", "cloudflared"]);
    command.args([
        "tunnel",
        "--no-autoupdate",
        "--grace-period",
        "2s",
        "--config",
    ]);
    command.arg(config);
    match (&share.hostname, &share.tunnel) {
        (Some(hostname), Some(tunnel)) => {
            route_dns(config, tunnel, hostname)?;
            if share.access == Access::Protected {
                verify_access(hostname, stop)?;
            } else if access_client()
                .is_ok_and(|client| probe_access(&client, hostname, "/").is_ok())
            {
                // Cloudflare Access covers the hostname whatever was asked, so report what visitors
                // actually get.
                share.access = Access::Protected;
            }
            command.args(["run", tunnel]);
        }
        _ => {
            command.args(["--url", &origin]);
            if share.is_https {
                command.arg("--no-tls-verify");
            }
        }
    }
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| {
            Error::Share(format!(
                "cloudflared is required but could not be started: {error}"
            ))
        })?;
    let result = keep_open(share, &mut child, stop);
    stop_gracefully(&mut child);
    result
}

/// Runs `"$@"` (cloudflared) and sends it SIGTERM when stdin closes, that is when the owner stops
/// or dies. The command's own stdin is /dev/null.
const OWNER_WATCH: &str = r#"exec 3<&0; "$@" </dev/null & p=$!; (cat <&3 >/dev/null; kill $p 2>/dev/null) >/dev/null 2>&1 & wait $p"#;

/// Ask `cloudflared` to unregister its connections, so Cloudflare stops routing to them at once,
/// and kill it if it is still running after a few seconds.
fn stop_gracefully(child: &mut Child) {
    // Closing the watcher's stdin makes it send SIGTERM to cloudflared.
    drop(child.stdin.take());
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        if matches!(child.try_wait(), Ok(Some(_))) {
            return;
        }
        std::thread::sleep(POLL);
    }
    let _ = Command::new("pkill")
        .args(["-KILL", "-P", &child.id().to_string()])
        .status();
    let _ = child.kill();
    let _ = child.wait();
}

/// What `cloudflared` has reported so far.
#[derive(Default)]
struct Progress {
    url: Option<String>,
    is_connected: bool,
    errors: Vec<String>,
}

fn keep_open(share: &mut Share, child: &mut Child, stop: &AtomicBool) -> Result<()> {
    let progress = Arc::new(Mutex::new(Progress::default()));
    if let Some(stderr) = child.stderr.take() {
        let progress = progress.clone();
        std::thread::spawn(move || {
            for line in BufReader::new(stderr)
                .lines()
                .map_while(std::result::Result::ok)
            {
                let Ok(mut progress) = progress.lock() else {
                    break;
                };
                if progress.url.is_none() {
                    progress.url = quick_tunnel_url(&line);
                }
                if line.contains("Registered tunnel connection") {
                    progress.is_connected = true;
                }
                if line.contains(" ERR ") {
                    progress.errors.push(line);
                    if progress.errors.len() > 3 {
                        progress.errors.remove(0);
                    }
                }
            }
        });
    }
    let failure = |progress: &Arc<Mutex<Progress>>, what: &str| {
        let errors = progress
            .lock()
            .map(|progress| progress.errors.join("\n"))
            .unwrap_or_default();
        Error::Share(if errors.is_empty() {
            what.to_owned()
        } else {
            format!("{what}:\n{errors}")
        })
    };
    let started = Instant::now();
    loop {
        if stop.load(Ordering::SeqCst) {
            return Ok(());
        }
        if child.try_wait()?.is_some() {
            return Err(failure(&progress, "cloudflared exited"));
        }
        if let Ok(progress) = progress.lock() {
            let url = match &share.hostname {
                _ if !progress.is_connected => None,
                Some(hostname) => Some(format!("https://{hostname}")),
                None => progress.url.clone(),
            };
            if let Some(url) = url {
                share.url = Some(url);
                break;
            }
        }
        if started.elapsed() >= START_TIMEOUT {
            return Err(failure(&progress, "timed out waiting for the tunnel"));
        }
        std::thread::sleep(POLL);
    }
    if share.hostname.is_none()
        && let Some(host) = share
            .url
            .as_deref()
            .and_then(|url| url.strip_prefix("https://"))
        && !wait_for_dns(host, stop)
    {
        return Err(Error::Share(format!(
            "{host} did not appear in DNS within 30 s; try again"
        )));
    }
    if share.hostname.is_some()
        && share.access == Access::Public
        && let Some(url) = &share.url
        && !wait_until_routed(url, stop)
    {
        return Err(Error::Share(format!(
            "Cloudflare still answers 530 for {url} after 20 s, so the tunnel is not routing; check that no other connector serves the tunnel"
        )));
    }
    share.state = ShareState::Ready;
    save(share)?;
    while !stop.load(Ordering::SeqCst) && now_ms() < share.expires_at {
        if child.try_wait()?.is_some() {
            return Err(failure(&progress, "cloudflared exited"));
        }
        std::thread::sleep(POLL);
    }
    Ok(())
}

/// The `cloudflared` configuration for one share. Quick tunnels get an empty configuration, so the
/// user's own `~/.cloudflared/config.yml` is never read.
fn config_text(share: &Share, origin: &str) -> String {
    let (Some(hostname), Some(tunnel)) = (&share.hostname, &share.tunnel) else {
        return "# Kestrl share: temporary trycloudflare.com tunnel\n".to_owned();
    };
    let tls = if share.is_https {
        "    originRequest:\n      noTLSVerify: true\n"
    } else {
        ""
    };
    format!(
        "# Kestrl share {}\ntunnel: {tunnel}\ningress:\n  - hostname: {hostname}\n    service: {origin}\n{tls}  - service: http_status:404\n",
        share.id
    )
}

/// Point `hostname` at `tunnel`. An existing DNS record for another target is never overwritten.
/// Kestrl's own `config` is passed because `cloudflared` otherwise takes the tunnel from the
/// user's default configuration instead of `tunnel`.
fn route_dns(config: &std::path::Path, tunnel: &str, hostname: &str) -> Result<()> {
    let output = Command::new("cloudflared")
        .args(["tunnel", "--config"])
        .arg(config)
        .args(["route", "dns", tunnel, hostname])
        .stdin(Stdio::null())
        .output()
        .map_err(|error| {
            Error::Share(format!(
                "cloudflared is required but could not be started: {error}"
            ))
        })?;
    if output.status.success() {
        return Ok(());
    }
    Err(Error::Share(format!(
        "could not route {hostname} to tunnel {tunnel:?}: {}",
        String::from_utf8_lossy(&output.stderr).trim()
    )))
}

/// Wait up to 30 s until the zone's authoritative nameservers answer for `host`. A quick tunnel's
/// name appears there a moment after its connection registers. Asking any caching resolver
/// before that would make it remember "no such host" for a minute, so only authoritative servers,
/// which do not cache, are asked.
/// `false` only when the name is confirmed missing for the whole wait; when the nameservers cannot
/// be asked, the outcome is unknown and treated as ready.
fn wait_for_dns(host: &str, stop: &AtomicBool) -> bool {
    let Some((_, zone)) = host.split_once('.') else {
        return true;
    };
    let servers = nameservers(zone);
    if servers.is_empty() {
        return true;
    }
    let deadline = Instant::now() + Duration::from_secs(30);
    let mut was_answered = false;
    while Instant::now() < deadline && !stop.load(Ordering::SeqCst) {
        let answers: Vec<Option<bool>> = servers
            .iter()
            .map(|server| has_address(server, host))
            .collect();
        if answers.contains(&Some(true)) {
            return true;
        }
        was_answered |= answers.iter().any(Option::is_some);
        std::thread::sleep(Duration::from_secs(1));
    }
    stop.load(Ordering::SeqCst) || !was_answered
}

/// Wait up to 20 s until Cloudflare's edge routes `url` to the new connection instead of answering
/// 530 (no tunnel), which it does for a few seconds after the connection registers.
/// `false` only when every answer during the wait was 530; when the edge cannot be reached the
/// outcome is unknown and treated as ready.
fn wait_until_routed(url: &str, stop: &AtomicBool) -> bool {
    let Ok(client) = reqwest::blocking::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(5))
        .build()
    else {
        return true;
    };
    let deadline = Instant::now() + Duration::from_secs(20);
    let mut saw_530 = false;
    while Instant::now() < deadline && !stop.load(Ordering::SeqCst) {
        match client.get(url).send() {
            Ok(response) if response.status().as_u16() != 530 => return true,
            Ok(_) => saw_530 = true,
            Err(_) => {}
        }
        std::thread::sleep(Duration::from_secs(1));
    }
    stop.load(Ordering::SeqCst) || !saw_530
}

/// The nameservers of `zone`, from Cloudflare's DNS over HTTPS. The zone exists, so its answer is
/// safe to cache.
fn nameservers(zone: &str) -> Vec<String> {
    let Ok(client) = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
    else {
        return Vec::new();
    };
    client
        .get("https://cloudflare-dns.com/dns-query")
        .query(&[("name", zone), ("type", "NS")])
        .header(reqwest::header::ACCEPT, "application/dns-json")
        .send()
        .and_then(|response| response.json::<serde_json::Value>())
        .ok()
        .and_then(|answer| answer["Answer"].as_array().cloned())
        .unwrap_or_default()
        .iter()
        .filter_map(|record| record["data"].as_str())
        .map(|name| name.trim_end_matches('.').to_owned())
        .collect()
}

/// Ask `server` directly, without recursion, whether `host` has an IPv4 address.
fn has_address(server: &str, host: &str) -> Option<bool> {
    use std::net::{ToSocketAddrs, UdpSocket};

    let address = (server, 53)
        .to_socket_addrs()
        .ok()?
        .find(|address| address.is_ipv4())?;
    let id = u16::try_from(now_ms().rem_euclid(65_536))
        .ok()?
        .to_be_bytes();
    // Header: id, no flags (recursion not desired), one question.
    let mut query = vec![id[0], id[1], 0, 0, 0, 1, 0, 0, 0, 0, 0, 0];
    for label in host.split('.') {
        query.push(u8::try_from(label.len()).ok()?);
        query.extend(label.bytes());
    }
    // Root label, type A, class IN.
    query.extend([0, 0, 1, 0, 1]);
    let socket = UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.set_read_timeout(Some(Duration::from_secs(2))).ok()?;
    socket.send_to(&query, address).ok()?;
    let mut response = [0; 512];
    let (length, _) = socket.recv_from(&mut response).ok()?;
    if length < 12 || response[..2] != id {
        return None;
    }
    let is_ok = response[3] & 0x0f == 0;
    let answers = u16::from_be_bytes([response[6], response[7]]);
    Some(is_ok && answers > 0)
}

/// Check, before anything is exposed, that Cloudflare Access sends unauthenticated visitors of
/// `hostname` to its login page. New DNS records can take a few seconds, so this retries.
fn verify_access(hostname: &str, stop: &AtomicBool) -> Result<()> {
    let client = access_client()?;
    let mut last = String::new();
    for _ in 0..4 {
        if stop.load(Ordering::SeqCst) {
            break;
        }
        // A path-specific Access application could exempt a sub-path, so check one that cannot be
        // known in advance as well as the root.
        let random = format!(
            "/kestrl-access-check-{}",
            ulid::Ulid::new().to_string().to_ascii_lowercase()
        );
        match probe_access(&client, hostname, "/")
            .and_then(|()| probe_access(&client, hostname, &random))
        {
            Ok(()) => return Ok(()),
            Err(seen) => last = seen,
        }
        std::thread::sleep(Duration::from_secs(3));
    }
    Err(Error::Share(format!(
        "{hostname} is not behind Cloudflare Access ({last}); add an Access application for it in Cloudflare Zero Trust, or share with --public"
    )))
}

fn access_client() -> Result<reqwest::blocking::Client> {
    Ok(reqwest::blocking::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(10))
        .build()?)
}

/// One unauthenticated request: `Ok` when Cloudflare Access answers with its login redirect,
/// otherwise what came back instead.
fn probe_access(
    client: &reqwest::blocking::Client,
    hostname: &str,
    path: &str,
) -> std::result::Result<(), String> {
    let response = client
        .get(format!("https://{hostname}{path}"))
        .send()
        .map_err(|error| error.to_string())?;
    let location = response
        .headers()
        .get(reqwest::header::LOCATION)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default();
    if response.status().is_redirection() && is_access_login(location) {
        Ok(())
    } else {
        Err(format!(
            "got HTTP {} for {path}",
            response.status().as_u16()
        ))
    }
}

fn is_access_login(location: &str) -> bool {
    location
        .strip_prefix("https://")
        .and_then(|rest| rest.split('/').next())
        .is_some_and(|host| host.ends_with(".cloudflareaccess.com"))
}

fn quick_tunnel_url(line: &str) -> Option<String> {
    let start = line.find("https://")?;
    let url: String = line[start..]
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || matches!(c, ':' | '/' | '.' | '-'))
        .collect();
    let host = url.strip_prefix("https://")?;
    (host.ends_with(".trycloudflare.com") && host != "api.trycloudflare.com").then_some(url)
}

fn is_hostname(hostname: &str) -> bool {
    hostname.contains('.')
        && hostname.len() <= 253
        && hostname.split('.').all(|label| {
            !label.is_empty()
                && !label.starts_with('-')
                && label
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        })
}

/// Whether `pid` is still the process that owns share `id`, not a later process that reused the ID.
fn is_owner(pid: u32, id: &str) -> bool {
    Command::new("ps")
        .args(["-o", "args=", "-p", &pid.to_string()])
        .output()
        .is_ok_and(|output| {
            String::from_utf8_lossy(&output.stdout).contains(&format!("share serve {id}"))
        })
}

fn signal(signal: &str, target: &str) -> bool {
    Command::new("kill")
        .args([signal, "--", target])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

fn duration_ms(duration: Duration) -> i64 {
    i64::try_from(duration.as_millis()).unwrap_or(i64::MAX)
}

fn dir() -> Result<PathBuf> {
    let dir = crate::paths::data_dir()?.join("shares");
    std::fs::create_dir_all(&dir)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700))?;
    }
    Ok(dir)
}

fn record(id: &str) -> PathBuf {
    dir()
        .map(|dir| dir.join(format!("{id}.json")))
        .unwrap_or_default()
}

fn save(share: &Share) -> Result<()> {
    write_private(&record(&share.id), &serde_json::to_vec_pretty(share)?)
}

fn remove(id: &str) {
    let _ = std::fs::remove_file(record(id));
}
