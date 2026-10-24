//! Reusable headers and query parameters from `.kestrl/profiles`.

use std::collections::BTreeMap;
use std::path::Path;

use serde::Deserialize;

use crate::workspace;
use crate::{Error, Result};

pub fn names(root: &Path) -> Result<Vec<String>> {
    workspace::names(&root.join(".kestrl/profiles"))
}

#[derive(Debug, Default, Deserialize)]
pub struct Profile {
    #[serde(default)]
    pub headers: BTreeMap<String, String>,
    #[serde(default)]
    pub query: BTreeMap<String, String>,
    pub when: Option<ProfileWhen>,
}

#[derive(Debug, Default, Deserialize)]
pub struct ProfileWhen {
    #[serde(default)]
    pub env: Vec<String>,
    pub url: Option<String>,
}

pub fn load(workspace: &Path, name: &str) -> Result<Profile> {
    let path = workspace
        .join(".kestrl/profiles")
        .join(format!("{name}.toml"));
    let text = std::fs::read_to_string(&path).map_err(|source| Error::Read {
        path: path.clone(),
        source,
    })?;
    toml::from_str(&text).map_err(|source| Error::Toml { path, source })
}

pub fn matches(profile: &Profile, env: &str, url: &str) -> bool {
    profile.when.as_ref().is_some_and(|when| {
        (when.env.is_empty() || when.env.iter().any(|candidate| candidate == env))
            && when
                .url
                .as_deref()
                .is_none_or(|glob| glob_matches(glob, url))
    })
}

fn glob_matches(pattern: &str, value: &str) -> bool {
    let mut pieces = pattern.split('*');
    let Some(first) = pieces.next() else {
        return false;
    };
    if !value.starts_with(first) {
        return false;
    }
    let mut offset = first.len();
    let rest = pieces.collect::<Vec<_>>();
    for (index, piece) in rest.iter().enumerate() {
        if piece.is_empty() {
            continue;
        }
        let Some(relative) = value[offset..].find(piece) else {
            return false;
        };
        let position = offset + relative;
        if index == rest.len() - 1
            && !pattern.ends_with('*')
            && position + piece.len() != value.len()
        {
            return false;
        }
        offset = position + piece.len();
    }
    pattern.ends_with('*') || offset == value.len()
}
