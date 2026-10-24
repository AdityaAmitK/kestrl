//! HTTPS interception proxy lifecycle.

use std::collections::HashSet;
use std::io::Read;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use http_body_util::BodyExt;
use hudsucker::certificate_authority::RcgenAuthority;
use hudsucker::hyper::{Request, Response};
use hudsucker::rcgen::{Issuer, KeyPair};
use hudsucker::rustls::crypto::aws_lc_rs;
use hudsucker::{Body, HttpContext, HttpHandler, Proxy, RequestOrResponse};
use tokio::net::TcpListener;

use super::CapturedExchange;
use crate::history::now_ms;
use crate::{Error, Result};

const MAX_CAPTURE_BODY_BYTES: usize = 1024 * 1024;

/// Options for starting the capture proxy.
pub struct ProxyOptions {
    /// TCP port to bind on loopback.
    pub port: u16,
    /// Already-bound loopback listener.
    pub listener: TcpListener,
    /// Optional host glob filters.
    pub hosts: Vec<String>,
    /// Receives completed HTTP exchanges.
    pub callback: Arc<dyn Fn(CapturedExchange) + Send + Sync>,
}

#[derive(Clone)]
struct CaptureHandler {
    callback: Arc<dyn Fn(CapturedExchange) + Send + Sync>,
    pending: Option<PendingExchange>,
    hosts: Vec<String>,
    tunneled_hosts: Arc<Mutex<HashSet<String>>>,
}

#[derive(Clone)]
struct PendingExchange {
    method: String,
    url: String,
    req_headers: Vec<(String, String)>,
    req_body: Option<String>,
    started: Instant,
    ts: i64,
}

impl HttpHandler for CaptureHandler {
    async fn handle_request(
        &mut self,
        _ctx: &HttpContext,
        request: Request<Body>,
    ) -> RequestOrResponse {
        let (parts, body) = request.into_parts();
        let bytes = body
            .collect()
            .await
            .map(|collected| collected.to_bytes())
            .unwrap_or_default();
        let host = parts
            .uri
            .host()
            .or_else(|| {
                parts
                    .headers
                    .get("host")
                    .and_then(|value| value.to_str().ok())
            })
            .unwrap_or_default();
        if !self.hosts.is_empty() && !self.hosts.iter().any(|glob| host_matches(glob, host)) {
            return Request::from_parts(parts, Body::from(bytes)).into();
        }
        let url = parts.uri.to_string();
        let req_headers = parts
            .headers
            .iter()
            .map(|(name, value)| {
                (
                    name.to_string(),
                    String::from_utf8_lossy(value.as_bytes()).into_owned(),
                )
            })
            .collect();
        let pending = PendingExchange {
            method: parts.method.to_string(),
            url,
            req_headers,
            req_body: display_body(
                &bytes,
                parts
                    .headers
                    .get("content-encoding")
                    .and_then(|value| value.to_str().ok()),
            ),
            started: Instant::now(),
            ts: now_ms(),
        };
        // hudsucker creates an InternalProxy and clones its handler inside each request's
        // service_fn call; the same instance handles this request and its response.
        self.pending = Some(pending);
        Request::from_parts(parts, Body::from(bytes)).into()
    }

    async fn handle_response(
        &mut self,
        _ctx: &HttpContext,
        response: Response<Body>,
    ) -> Response<Body> {
        let (parts, body) = response.into_parts();
        let bytes = body
            .collect()
            .await
            .map(|collected| collected.to_bytes())
            .unwrap_or_default();
        if let Some(pending) = self.pending.take() {
            (self.callback)(CapturedExchange {
                method: pending.method,
                url: pending.url,
                req_headers: pending.req_headers,
                req_body: pending.req_body,
                status: Some(parts.status.as_u16()),
                res_headers: parts
                    .headers
                    .iter()
                    .map(|(name, value)| {
                        (
                            name.to_string(),
                            String::from_utf8_lossy(value.as_bytes()).into_owned(),
                        )
                    })
                    .collect(),
                res_body: display_body(
                    &bytes,
                    parts
                        .headers
                        .get("content-encoding")
                        .and_then(|value| value.to_str().ok()),
                ),
                duration_ms: u64::try_from(pending.started.elapsed().as_millis())
                    .unwrap_or(u64::MAX),
                ts: pending.ts,
                tunneled: false,
            });
        }
        Response::from_parts(parts, Body::from(bytes))
    }

    async fn should_intercept_tls(
        &mut self,
        _ctx: &HttpContext,
        client_hello: hudsucker::rustls::server::ClientHello<'_>,
    ) -> bool {
        let host = client_hello.server_name().unwrap_or_default().to_owned();
        let should_intercept =
            self.hosts.is_empty() || self.hosts.iter().any(|glob| host_matches(glob, &host));
        if !should_intercept
            && let Ok(mut tunneled_hosts) = self.tunneled_hosts.lock()
            && tunneled_hosts.insert(host.clone())
        {
            (self.callback)(CapturedExchange {
                method: "CONNECT".to_owned(),
                url: format!("https://{host}"),
                req_headers: Vec::new(),
                req_body: None,
                status: None,
                res_headers: Vec::new(),
                res_body: None,
                duration_ms: 0,
                ts: now_ms(),
                tunneled: true,
            });
        }
        should_intercept
    }
}

fn host_matches(pattern: &str, host: &str) -> bool {
    if pattern == "*" {
        return true;
    }
    if let Some(suffix) = pattern.strip_prefix('*') {
        return host.ends_with(suffix);
    }
    pattern.eq_ignore_ascii_case(host)
}

fn display_body(bytes: &[u8], encoding: Option<&str>) -> Option<String> {
    let decoded = match encoding.map(str::to_ascii_lowercase).as_deref() {
        Some("gzip") => read_limited(flate2::read::GzDecoder::new(bytes)),
        Some("deflate") => read_limited(flate2::read::ZlibDecoder::new(bytes)),
        Some("br") => read_limited(brotli::Decompressor::new(bytes, 4096)),
        _ => bytes[..bytes.len().min(MAX_CAPTURE_BODY_BYTES)].to_vec(),
    };
    String::from_utf8(decoded).ok()
}

fn read_limited(reader: impl Read) -> Vec<u8> {
    let mut limited = reader.take(MAX_CAPTURE_BODY_BYTES as u64);
    let mut decoded = Vec::new();
    let _ = limited.read_to_end(&mut decoded);
    decoded
}

/// Run the HTTPS interception proxy on a pre-bound loopback listener.
pub fn run(
    options: ProxyOptions,
    shutdown: tokio::sync::oneshot::Receiver<()>,
    ready: tokio::sync::oneshot::Sender<()>,
) -> Result<()> {
    let paths = super::ca::ensure()?;
    let certificate = std::fs::read_to_string(paths.certificate)?;
    let private_key = std::fs::read_to_string(paths.private_key)?;
    let key = KeyPair::from_pem(&private_key).map_err(|error| Error::Capture(error.to_string()))?;
    let issuer = Issuer::from_ca_cert_pem(&certificate, key)
        .map_err(|error| Error::Capture(error.to_string()))?;
    let authority = RcgenAuthority::new(issuer, 1_000, aws_lc_rs::default_provider());
    let handler = CaptureHandler {
        callback: options.callback,
        pending: None,
        hosts: options.hosts,
        tunneled_hosts: Arc::new(Mutex::new(HashSet::new())),
    };
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(|error| Error::Capture(error.to_string()))?;
    let listener = options.listener;
    runtime.block_on(async move {
        let proxy = Proxy::builder()
            .with_listener(listener)
            .with_ca(authority)
            .with_rustls_connector(aws_lc_rs::default_provider())
            .with_http_handler(handler)
            .with_graceful_shutdown(async move {
                let _ = shutdown.await;
            })
            .build()
            .map_err(|error| Error::Capture(error.to_string()))?;
        let server = tokio::spawn(proxy.start());
        let _ = ready.send(());
        server
            .await
            .map_err(|error| Error::Capture(error.to_string()))?
            .map_err(|error| Error::Capture(error.to_string()))
    })
}
