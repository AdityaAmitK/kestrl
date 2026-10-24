//! Keychain secrets for the app. Values go in and never come back out.

use std::path::Path;

use kestrl_core::secrets;

use crate::error::KestrlError;

/// A secret the workspace refers to, and whether the Keychain holds it.
#[derive(Debug, uniffi::Record)]
pub struct SecretStatus {
    pub name: String,
    pub is_set: bool,
}

/// Store a secret in the Keychain, replacing any previous value.
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

/// Remove a secret from the Keychain.
#[uniffi::export]
pub fn delete_secret(name: String) -> Result<(), KestrlError> {
    secrets::delete(&name)?;
    Ok(())
}

/// Every secret the workspace refers to, with whether it is set.
#[uniffi::export]
pub fn secret_status(workspace_root: String) -> Result<Vec<SecretStatus>, KestrlError> {
    secrets::referenced_in_workspace(Path::new(&workspace_root))?
        .into_iter()
        .map(|name| {
            let is_set = secrets::is_set(&name)?;
            Ok::<_, KestrlError>(SecretStatus { name, is_set })
        })
        .collect()
}
