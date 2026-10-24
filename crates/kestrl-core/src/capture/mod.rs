//! Device capture through a local HTTPS interception proxy.

pub mod ca;
pub mod device;
pub mod proxy;
pub mod system_proxy;

/// One completed request and response observed by the proxy.
#[derive(Debug, Clone, serde::Serialize)]
pub struct CapturedExchange {
    /// HTTP method.
    pub method: String,
    /// Full URL.
    pub url: String,
    /// Request headers.
    pub req_headers: Vec<(String, String)>,
    /// Request body.
    pub req_body: Option<String>,
    /// Response status.
    pub status: Option<u16>,
    /// Response headers.
    pub res_headers: Vec<(String, String)>,
    /// Response body.
    pub res_body: Option<String>,
    /// Request duration in milliseconds.
    pub duration_ms: u64,
    /// Unix timestamp in milliseconds.
    pub ts: i64,
    /// Whether this connection was passed through as a TLS tunnel.
    pub tunneled: bool,
}
