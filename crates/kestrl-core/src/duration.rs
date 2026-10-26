//! Relative time windows such as `30s`, `2m`, `1h` and `2d`.

use crate::history::now_ms;
use crate::{Error, Result};

/// Unix ms at the start of the window that ends now: `2m` gives the time two minutes ago.
pub fn parse_since(spec: &str) -> Result<i64> {
    let window = i64::try_from(parse_duration(spec)?.as_millis()).unwrap_or(i64::MAX);
    Ok(now_ms().saturating_sub(window))
}

/// A length of time such as `30s`, `2m`, `1h` or `2d`.
pub fn parse_duration(spec: &str) -> Result<std::time::Duration> {
    let invalid = || Error::InvalidDuration(spec.to_owned());
    let unit_seconds: u64 = match spec.chars().last() {
        Some('s') => 1,
        Some('m') => 60,
        Some('h') => 3_600,
        Some('d') => 86_400,
        _ => return Err(invalid()),
    };
    // The last char is ASCII here, so this slice is on a char boundary.
    let count: u32 = spec[..spec.len() - 1].parse().map_err(|_| invalid())?;
    Ok(std::time::Duration::from_secs(
        u64::from(count) * unit_seconds,
    ))
}
