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

/// Redact secrets and credential headers from a captured exchange.
pub fn redact(
    mut exchange: CapturedExchange,
    redactor: &crate::secrets::Redactor,
) -> CapturedExchange {
    use crate::secrets::MASK;

    exchange.method = redactor.redact(&exchange.method);
    exchange.url = redactor.redact(&exchange.url);
    for (name, value) in exchange
        .req_headers
        .iter_mut()
        .chain(&mut exchange.res_headers)
    {
        if matches!(
            name.to_ascii_lowercase().as_str(),
            "authorization" | "proxy-authorization" | "cookie" | "set-cookie"
        ) {
            *value = MASK.to_owned();
        } else {
            *value = redactor.redact(value);
        }
        *name = redactor.redact(name);
    }
    exchange.req_body = exchange.req_body.map(|body| redactor.redact(&body));
    exchange.res_body = exchange.res_body.map(|body| redactor.redact(&body));
    exchange
}
