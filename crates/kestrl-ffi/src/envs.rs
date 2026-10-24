//! Resolved environments for the app. Secret values are masked.

use std::path::Path;

use kestrl_core::env::Environment;

use crate::error::KestrlError;
use crate::types::KeyValue;

/// The variables of `env` with every reference expanded and secrets masked.
#[uniffi::export]
pub fn resolved_env(workspace_root: String, env: String) -> Result<Vec<KeyValue>, KestrlError> {
    let vars = Environment::resolved(Path::new(&workspace_root), &env)?;
    Ok(KeyValue::from_map(vars))
}
