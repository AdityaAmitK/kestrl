//! Assemble CDP Network events into captured exchanges.

use std::collections::HashMap;
use std::time::Instant;

use serde_json::Value;

use crate::Result;
use crate::capture::CapturedExchange;
use crate::history::now_ms;

struct Pending {
    method: String,
    url: String,
    headers: Vec<(String, String)>,
    body: Option<String>,
    ts: i64,
    started: Instant,
    status: Option<u16>,
    response_headers: Vec<(String, String)>,
    mime: String,
}
pub(super) struct NetworkCapture {
    pending: HashMap<(String, String), Pending>,
    hosts: Vec<String>,
}

impl NetworkCapture {
    pub fn new(hosts: Vec<String>) -> Self {
        Self {
            pending: HashMap::new(),
            hosts,
        }
    }
    pub fn event(
        &mut self,
        method: &str,
        params: &Value,
        session: &str,
        callback: &std::sync::Arc<dyn Fn(CapturedExchange) + Send + Sync>,
    ) -> Result<Option<(CapturedExchange, String)>> {
        let id = params["requestId"].as_str().unwrap_or_default().to_owned();
        let key = (session.to_owned(), id.clone());
        match method {
            "Network.requestWillBeSent" => {
                let request = &params["request"];
                let url = request["url"].as_str().unwrap_or_default();
                if !(url.starts_with("http://") || url.starts_with("https://")) {
                    return Ok(None);
                }
                let host = url
                    .split_once("://")
                    .map(|(_, rest)| {
                        rest.split('/')
                            .next()
                            .unwrap_or_default()
                            .split(':')
                            .next()
                            .unwrap_or_default()
                    })
                    .unwrap_or_default();
                if !self.hosts.is_empty()
                    && !self.hosts.iter().any(|pattern| host_matches(pattern, host))
                {
                    return Ok(None);
                }
                self.pending.insert(
                    key,
                    Pending {
                        method: request["method"].as_str().unwrap_or("GET").to_owned(),
                        url: url.to_owned(),
                        headers: headers(&request["headers"]),
                        body: request["postData"].as_str().map(str::to_owned),
                        ts: now_ms(),
                        started: Instant::now(),
                        status: None,
                        response_headers: Vec::new(),
                        mime: String::new(),
                    },
                );
            }
            "Network.requestWillBeSentExtraInfo" => {
                if let Some(p) = self.pending.get_mut(&key) {
                    p.headers = headers(&params["headers"]);
                }
            }
            "Network.responseReceived" => {
                if let Some(p) = self.pending.get_mut(&key) {
                    let response = &params["response"];
                    p.status = response["status"].as_u64().map(|n| n as u16);
                    p.response_headers = headers(&response["headers"]);
                    p.mime = response["mimeType"].as_str().unwrap_or_default().to_owned();
                }
            }
            "Network.loadingFinished" => {
                if let Some(p) = self.pending.remove(&key) {
                    let skip_body = p.mime.starts_with("image/")
                        || p.mime.starts_with("font/")
                        || p.mime.starts_with("audio/")
                        || p.mime.starts_with("video/");
                    let exchange = CapturedExchange {
                        method: p.method,
                        url: p.url,
                        req_headers: p.headers,
                        req_body: p.body,
                        status: p.status,
                        res_headers: p.response_headers,
                        res_body: None,
                        duration_ms: p.started.elapsed().as_millis() as u64,
                        ts: p.ts,
                        tunneled: false,
                    };
                    if !skip_body {
                        return Ok(Some((exchange, id)));
                    }
                    callback(exchange);
                }
            }
            "Network.loadingFailed" => {
                if let Some(p) = self.pending.remove(&key) {
                    callback(CapturedExchange {
                        method: p.method,
                        url: p.url,
                        req_headers: p.headers,
                        req_body: p.body,
                        status: None,
                        res_headers: Vec::new(),
                        res_body: Some(
                            params["errorText"]
                                .as_str()
                                .unwrap_or("request failed")
                                .to_owned(),
                        ),
                        duration_ms: p.started.elapsed().as_millis() as u64,
                        ts: p.ts,
                        tunneled: false,
                    });
                }
            }
            _ => {}
        }
        Ok(None)
    }
}

fn headers(value: &Value) -> Vec<(String, String)> {
    value
        .as_object()
        .map(|map| {
            map.iter()
                .map(|(k, v)| (k.clone(), v.as_str().unwrap_or_default().to_owned()))
                .collect()
        })
        .unwrap_or_default()
}
fn host_matches(pattern: &str, host: &str) -> bool {
    super::super::proxy::host_matches(pattern, host)
}
