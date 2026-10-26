//! Secrets for the app. Values go in and never come back out.

use std::path::Path;

use kestrl_core::secrets;

use crate::error::KestrlError;

/// A secret the workspace refers to or the secrets file holds.
#[derive(Debug, uniffi::Record)]
pub struct SecretStatus {
    pub name: String,
    /// A value is stored.
    pub is_set: bool,
    /// A workspace file uses it as `{{secret:name}}`.
    pub is_referenced: bool,
}

/// Store a secret, replacing any previous value.
#[uniffi::export]
pub fn set_secret(name: String, value: String) -> Result<(), KestrlError> {
    if value.is_empty() {
        return Err(KestrlError::InvalidInput {
            message: "secret value cannot be empty".to_owned(),
        });
    }
    secrets::set(&name, &value)?;
    Ok(())
}

/// Remove a stored secret.
#[uniffi::export]
pub fn delete_secret(name: String) -> Result<(), KestrlError> {
    secrets::delete(&name)?;
    Ok(())
}

/// Every secret the workspace refers to or Kestrl stored, with whether each is set and used.
#[uniffi::export]
pub fn secret_status(workspace_root: String) -> Result<Vec<SecretStatus>, KestrlError> {
    let referenced = secrets::referenced_in_workspace(Path::new(&workspace_root))?;
    let stored = secrets::stored_names()?;
    Ok(referenced
        .union(&stored)
        .map(|name| SecretStatus {
            name: name.clone(),
            is_set: stored.contains(name),
            is_referenced: referenced.contains(name),
        })
        .collect())
}
