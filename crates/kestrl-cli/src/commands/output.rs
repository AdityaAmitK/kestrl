//! Shared terminal and JSON rendering for HTTP exchanges.

use std::io::Write;

use kestrl_core::http::Exchange;

pub fn print_json(value: &serde_json::Value) -> anyhow::Result<()> {
    writeln!(
        std::io::stdout(),
        "{}",
        serde_json::to_string_pretty(value)?
    )?;
    Ok(())
}

/// Print a warning on stderr; the command itself still succeeds.
pub fn warn(message: &str) {
    let _ = writeln!(std::io::stderr(), "warning: {message}");
}

pub fn exchange_json(exchange: &Exchange, event_id: Option<&str>) -> serde_json::Value {
    let body = serde_json::from_str(&exchange.response_body)
        .unwrap_or_else(|_| serde_json::Value::String(exchange.response_body.clone()));
    serde_json::json!({"id": event_id, "status": exchange.status, "ms": exchange.duration_ms, "headers": exchange.response_headers, "body": body, "redacted": exchange.redacted, "error": exchange.error})
}

pub fn print_exchange_json(exchange: &Exchange, event_id: Option<&str>) -> anyhow::Result<()> {
    print_json(&exchange_json(exchange, event_id))
}

pub fn print_human(exchange: &Exchange, event_id: Option<&str>) {
    let mut stdout = std::io::stdout().lock();
    let _ = writeln!(
        stdout,
        "{} {} {} ({} ms)",
        exchange.method,
        exchange.url,
        exchange
            .status
            .map_or_else(|| "failed".to_owned(), |status| status.to_string()),
        exchange.duration_ms
    );
    for (name, value) in &exchange.response_headers {
        let _ = writeln!(stdout, "{name}: {value}");
    }
    if let Some(error) = &exchange.error {
        let _ = writeln!(std::io::stderr(), "Error: {error}");
    }
    if !exchange.response_body.is_empty() {
        let pretty = serde_json::from_str::<serde_json::Value>(&exchange.response_body)
            .ok()
            .and_then(|value| serde_json::to_string_pretty(&value).ok());
        let _ = writeln!(
            stdout,
            "\n{}",
            pretty.as_deref().unwrap_or(exchange.response_body.as_str())
        );
    }
    if let Some(id) = event_id {
        let _ = writeln!(stdout, "\n\x1b[2mevent {id}\x1b[0m");
    }
}
