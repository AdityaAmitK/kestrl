//! Profiles for the app: reusable headers and query parameters, optionally conditional.

use std::path::Path;

use kestrl_core::profile_files::{self, ProfileSpec};

use crate::error::KestrlError;
use crate::types::KeyValue;

/// A profile as written in its file.
#[derive(Debug, Clone, uniffi::Record)]
pub struct ProfileRecord {
    pub headers: Vec<KeyValue>,
    pub query: Vec<KeyValue>,
    /// Apply automatically only in these environments (empty: any)
    pub when_envs: Vec<String>,
    /// Apply automatically only to URLs matching this `*` pattern
    pub when_url: Option<String>,
}

impl From<ProfileSpec> for ProfileRecord {
    fn from(spec: ProfileSpec) -> Self {
        Self {
            headers: KeyValue::from_pairs(spec.headers),
            query: KeyValue::from_pairs(spec.query),
            when_envs: spec.when_envs,
            when_url: spec.when_url,
        }
    }
}

impl From<ProfileRecord> for ProfileSpec {
    fn from(record: ProfileRecord) -> Self {
        Self {
            headers: record
                .headers
                .into_iter()
                .map(|pair| (pair.key, pair.value))
                .collect(),
            query: record
                .query
                .into_iter()
                .map(|pair| (pair.key, pair.value))
                .collect(),
            when_envs: record.when_envs,
            when_url: record.when_url.filter(|url| !url.trim().is_empty()),
        }
    }
}

#[uniffi::export]
pub fn read_profile(workspace_root: String, name: String) -> Result<ProfileRecord, KestrlError> {
    Ok(profile_files::read(Path::new(&workspace_root), &name)?.into())
}

#[uniffi::export]
pub fn save_profile(
    workspace_root: String,
    name: String,
    profile: ProfileRecord,
) -> Result<(), KestrlError> {
    Ok(profile_files::save(
        Path::new(&workspace_root),
        &name,
        &profile.into(),
    )?)
}

#[uniffi::export]
pub fn create_profile(
    workspace_root: String,
    name: String,
    copy_from: Option<String>,
) -> Result<(), KestrlError> {
    Ok(profile_files::create(
        Path::new(&workspace_root),
        &name,
        copy_from.as_deref(),
    )?)
}

#[uniffi::export]
pub fn rename_profile(workspace_root: String, from: String, to: String) -> Result<(), KestrlError> {
    Ok(profile_files::rename(
        Path::new(&workspace_root),
        &from,
        &to,
    )?)
}

#[uniffi::export]
pub fn delete_profile(workspace_root: String, name: String) -> Result<(), KestrlError> {
    Ok(profile_files::delete(Path::new(&workspace_root), &name)?)
}
