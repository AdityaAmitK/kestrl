//! Minimal synchronous CDP event loop.

use std::thread;
use std::time::Duration;

use serde_json::{Value, json};
use std::net::TcpStream;
use std::sync::atomic::{AtomicBool, Ordering};
use tungstenite::{Message, WebSocket, stream::MaybeTlsStream};

use super::launch::BrowserProcess;
use super::network::NetworkCapture;
use crate::Result;

type Socket = WebSocket<MaybeTlsStream<TcpStream>>;

pub(super) fn capture(
    process: &mut BrowserProcess,
    hosts: &[String],
    start_url: Option<&str>,
    callback: &std::sync::Arc<dyn Fn(crate::capture::CapturedExchange) + Send + Sync>,
    stop: &AtomicBool,
) -> Result<()> {
    let endpoint = format!("http://127.0.0.1:{}/json/version", process.port);
    let response: Value = reqwest::blocking::get(endpoint)?.json()?;
    let websocket = response["webSocketDebuggerUrl"].as_str().ok_or_else(|| {
        crate::Error::Capture("browser DevTools endpoint omitted websocket URL".to_owned())
    })?;
    let (mut socket, _) = tungstenite::connect(websocket)?;
    let mut next_id = 0;
    send(
        &mut socket,
        &mut next_id,
        "Target.setDiscoverTargets",
        json!({"discover": true}),
    )?;
    send(
        &mut socket,
        &mut next_id,
        "Target.setAutoAttach",
        json!({"autoAttach": true,"waitForDebuggerOnStart": false,"flatten": true}),
    )?;
    let get_targets_command = send(&mut socket, &mut next_id, "Target.getTargets", json!({}))?;
    let mut attached = std::collections::HashSet::new();
    let mut pages = std::collections::HashSet::new();
    let mut network = NetworkCapture::new(hosts.to_vec());
    let mut navigated = false;
    loop {
        if process.has_exited()? || stop.load(Ordering::Acquire) {
            break;
        }
        if let MaybeTlsStream::Plain(stream) = socket.get_mut() {
            stream.set_read_timeout(Some(Duration::from_millis(100)))?;
        }
        let message = socket.read();
        match message {
            Ok(Message::Text(text)) => {
                let value: Value = serde_json::from_str(&text)?;
                if value["id"].as_u64() == Some(get_targets_command)
                    && let Some(targets) = value["result"]["targetInfos"].as_array()
                {
                    for target in targets.iter().filter(|target| target["type"] == "page") {
                        if let Some(target_id) = target["targetId"].as_str() {
                            send(
                                &mut socket,
                                &mut next_id,
                                "Target.attachToTarget",
                                json!({"targetId": target_id, "flatten": true}),
                            )?;
                        }
                    }
                }
                if let Some(method) = value["method"].as_str() {
                    if method == "Target.attachedToTarget" {
                        if let Some(session) = value["params"]["sessionId"].as_str() {
                            let is_page = value["params"]["targetInfo"]["type"] == "page";
                            let target_id = value["params"]["targetInfo"]["targetId"]
                                .as_str()
                                .unwrap_or_default()
                                .to_owned();
                            let is_new = attached.insert(target_id.clone());
                            if is_page && !is_new {
                                send(
                                    &mut socket,
                                    &mut next_id,
                                    "Target.detachFromTarget",
                                    json!({"sessionId": session}),
                                )?;
                                continue;
                            }
                            if is_page {
                                pages.insert(target_id.clone());
                            }
                            send_session(
                                &mut socket,
                                &mut next_id,
                                "Network.enable",
                                json!({}),
                                session,
                            )?;
                            send_session(
                                &mut socket,
                                &mut next_id,
                                "Runtime.runIfWaitingForDebugger",
                                json!({}),
                                session,
                            )?;
                            if is_page
                                && !navigated
                                && let Some(url) = start_url
                            {
                                navigated = true;
                                send_session(
                                    &mut socket,
                                    &mut next_id,
                                    "Page.enable",
                                    json!({}),
                                    session,
                                )?;
                                send_session(
                                    &mut socket,
                                    &mut next_id,
                                    "Page.navigate",
                                    json!({"url": url}),
                                    session,
                                )?;
                            }
                        }
                    } else if method == "Target.targetDestroyed" {
                        if let Some(target_id) = value["params"]["targetId"].as_str() {
                            pages.remove(target_id);
                            if pages.is_empty() && !attached.is_empty() {
                                break;
                            }
                        }
                    } else if method.starts_with("Network.") {
                        let session = value["sessionId"].as_str().unwrap_or_default();
                        if let Some((mut exchange, request_id)) =
                            network.event(method, &value["params"], session, callback)?
                        {
                            let body_id = send_session(
                                &mut socket,
                                &mut next_id,
                                "Network.getResponseBody",
                                json!({"requestId": request_id}),
                                session,
                            )?;
                            let encoded = exchange_body(&mut socket, body_id, stop)?;
                            if let Some(encoded) = encoded {
                                let body = encoded["body"].as_str().unwrap_or_default();
                                if encoded["base64Encoded"].as_bool().unwrap_or(false) {
                                    use base64::Engine;
                                    let bytes = base64::engine::general_purpose::STANDARD
                                        .decode(body)
                                        .unwrap_or_default();
                                    exchange.res_body = Some(
                                        String::from_utf8_lossy(
                                            &bytes[..bytes.len().min(1024 * 1024)],
                                        )
                                        .into_owned(),
                                    );
                                } else {
                                    exchange.res_body =
                                        Some(body.chars().take(1024 * 1024).collect());
                                }
                            }
                            callback(exchange);
                        }
                    }
                }
            }
            Ok(Message::Close(_)) => break,
            Err(tungstenite::Error::Io(error))
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                ) =>
            {
                thread::sleep(Duration::from_millis(20))
            }
            Err(error) => return Err(error.into()),
            _ => {}
        }
    }
    Ok(())
}

fn send(socket: &mut Socket, id: &mut u64, method: &str, params: Value) -> Result<u64> {
    *id += 1;
    let current = *id;
    socket.send(Message::Text(
        json!({"id": current,"method": method,"params": params})
            .to_string()
            .into(),
    ))?;
    Ok(current)
}
fn exchange_body(
    socket: &mut Socket,
    expected_id: u64,
    stop: &AtomicBool,
) -> Result<Option<Value>> {
    loop {
        if stop.load(Ordering::Acquire) {
            return Ok(None);
        }
        match socket.read() {
            Ok(Message::Text(text)) => {
                let value: Value = serde_json::from_str(&text)?;
                if value["id"].as_u64() == Some(expected_id) {
                    return Ok(value.get("result").cloned());
                }
            }
            Err(tungstenite::Error::Io(error))
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                ) =>
            {
                continue;
            }
            Err(error) => return Err(error.into()),
            _ => {}
        }
    }
}
fn send_session(
    socket: &mut Socket,
    id: &mut u64,
    method: &str,
    params: Value,
    session: &str,
) -> Result<u64> {
    *id += 1;
    let current = *id;
    socket.send(Message::Text(
        json!({"id": current,"method": method,"params": params,"sessionId": session})
            .to_string()
            .into(),
    ))?;
    Ok(current)
}
