//! Relative time windows such as `30s`, `2m`, `1h` and `2d`.

use crate::history::now_ms;
use crate::{Error, Result};

/// Unix ms at the start of the window that ends now: `2m` gives the time two minutes ago.
pub fn parse_since(spec: &str) -> Result<i64> {
    let invalid = || Error::InvalidDuration(spec.to_owned());
    let unit_ms: i64 = match spec.chars().last() {
        Some('s') => 1_000,
        Some('m') => 60_000,
        Some('h') => 3_600_000,
        Some('d') => 86_400_000,
        _ => return Err(invalid()),
    };
    // The last char is ASCII here, so this slice is on a char boundary.
    let count: u32 = spec[..spec.len() - 1].parse().map_err(|_| invalid())?;
    Ok(now_ms().saturating_sub(i64::from(count) * unit_ms))
}
