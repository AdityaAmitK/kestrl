//! Shared terminal and JSON rendering for CLI output.

use std::io::Write;

use kestrl_core::http::Exchange;

/// Compact age such as `5s`, `12m`, `3h` or `2d`.
pub fn age(milliseconds: i64) -> String {
    let seconds = milliseconds.max(0) / 1000;
    match seconds {
        0..60 => format!("{seconds}s"),
        60..3600 => format!("{}m", seconds / 60),
        3600..86400 => format!("{}h", seconds / 3600),
        _ => format!("{}d", seconds / 86400),
    }
}

/// Write pretty-printed JSON to stdout.
pub fn print_json(value: &serde_json::Value) -> anyhow::Result<()> {
    writeln!(
        std::io::stdout(),
        "{}",
        serde_json::to_string_pretty(value)?
    )?;
    Ok(())
}

/// Convert an HTTP exchange to its CLI JSON representation.
pub fn exchange_json(exchange: &Exchange) -> serde_json::Value {
    let body = serde_json::from_str(&exchange.response_body)
        .unwrap_or_else(|_| serde_json::Value::String(exchange.response_body.clone()));
    serde_json::json!({"status": exchange.status, "ms": exchange.duration_ms, "headers": exchange.response_headers, "body": body, "redacted": exchange.redacted, "error": exchange.error})
}

/// Write an HTTP exchange as JSON.
pub fn print_exchange_json(exchange: &Exchange) -> anyhow::Result<()> {
    print_json(&exchange_json(exchange))
}

/// Write an HTTP exchange in a human-readable form.
pub fn print_human(exchange: &Exchange) {
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
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(&exchange.response_body)
            && let Ok(pretty) = serde_json::to_string_pretty(&value)
        {
            let _ = writeln!(stdout, "\n{pretty}");
            return;
        }
        let _ = writeln!(stdout, "\n{}", exchange.response_body);
    }
}
