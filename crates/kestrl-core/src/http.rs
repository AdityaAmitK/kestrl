//! Synchronous HTTP sending and fully redacted exchange results.

use std::collections::BTreeMap;
use std::time::Instant;

use serde::Serialize;

use crate::Result;
use crate::request::ResolvedRequest;

pub type Client = reqwest::blocking::Client;

#[derive(Debug, Serialize)]
pub struct Exchange {
    pub method: String,
    pub url: String,
    pub request_headers: BTreeMap<String, String>,
    pub request_body: Option<String>,
    pub status: Option<u16>,
    pub response_headers: BTreeMap<String, String>,
    pub response_body: String,
    pub duration_ms: u128,
    pub error: Option<String>,
    pub redacted: Vec<String>,
}

pub fn client() -> Result<reqwest::blocking::Client> {
    Ok(reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()?)
}

pub fn send(client: &reqwest::blocking::Client, request: ResolvedRequest) -> Result<Exchange> {
    let method = reqwest::Method::from_bytes(request.method.as_bytes())
        .map_err(|_| crate::Error::InvalidMethod(request.method.clone()))?;
    let mut builder = client.request(method, &request.url).query(&request.query);
    for (name, value) in &request.headers {
        builder = builder.header(name, value);
    }
    if let Some(body) = &request.body {
        builder = builder.body(body.clone());
    }
    let final_url = builder
        .try_clone()
        .ok_or_else(|| crate::Error::InvalidRequest("request body cannot be cloned".to_owned()))?
        .build()?
        .url()
        .as_str()
        .to_owned();
    let started = Instant::now();
    let result = builder.send();
    let duration_ms = started.elapsed().as_millis();
    let (status, response_headers, response_body, error) = match result {
        Ok(response) => {
            let status = response.status().as_u16();
            let response_headers = response
                .headers()
                .iter()
                .map(|(key, value)| {
                    (
                        key.to_string(),
                        value.to_str().unwrap_or_default().to_owned(),
                    )
                })
                .collect();
            let response_body = response.text().unwrap_or_default();
            (Some(status), response_headers, response_body, None)
        }
        Err(error) => (
            None,
            BTreeMap::new(),
            String::new(),
            Some(error.to_string()),
        ),
    };
    let names = request
        .redactor
        .names()
        .into_iter()
        .map(str::to_owned)
        .collect();
    let redact = |value: &str| request.redactor.redact(value);
    Ok(Exchange {
        method: redact(&request.method),
        url: redact(&final_url),
        request_headers: request
            .headers
            .into_iter()
            .map(|(k, v)| (redact(&k), redact(&v)))
            .collect(),
        request_body: request.body.as_deref().map(redact),
        status,
        response_headers: response_headers
            .into_iter()
            .map(|(k, v)| (redact(&k), redact(&v)))
            .collect(),
        response_body: redact(&response_body),
        duration_ms,
        error: error.as_deref().map(redact),
        redacted: names,
    })
}
