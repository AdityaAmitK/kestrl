//! Secrets live in the macOS Keychain (service "kestrl"), referenced as
//! `{{secret:name}}`. Resolved values are tracked so output can be masked.

use std::collections::BTreeSet;
use std::path::Path;

use crate::{Error, Result};

const SERVICE: &str = "kestrl";
pub const MASK: &str = "••••••";

pub fn get(name: &str) -> Result<String> {
    match keyring::Entry::new(SERVICE, name)?.get_password() {
        Ok(value) => Ok(value),
        Err(keyring::Error::NoEntry) => Err(Error::SecretNotSet {
            name: name.to_owned(),
        }),
        Err(error) => Err(Error::Keychain(error)),
    }
}

/// Whether the Keychain holds a value for `name`.
pub fn is_set(name: &str) -> Result<bool> {
    match get(name) {
        Ok(_) => Ok(true),
        Err(Error::SecretNotSet { .. }) => Ok(false),
        Err(error) => Err(error),
    }
}

/// Every secret name the workspace refers to: in `.kestrl/` and in request files.
pub fn referenced_in_workspace(root: &Path) -> Result<BTreeSet<String>> {
    let mut found = BTreeSet::new();
    let kestrl_dir = root.join(".kestrl");
    if kestrl_dir.is_dir() {
        scan(&kestrl_dir, true, &mut found)?;
    }
    scan(root, false, &mut found)?;
    Ok(found)
}

fn scan(dir: &Path, is_kestrl: bool, found: &mut BTreeSet<String>) -> Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            let name = path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or_default();
            if name == "target" || name == "node_modules" || name == ".git" || name.starts_with('.')
            {
                continue;
            }
            scan(&path, is_kestrl || name == ".kestrl", found)?;
        } else if (is_kestrl
            || path
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.ends_with(".req.toml")))
            && let Ok(content) = std::fs::read_to_string(&path)
        {
            found.extend(referenced_names(&content));
        }
    }
    Ok(())
}

pub fn referenced_names(text: &str) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    let mut rest = text;
    while let Some(start) = rest.find("{{secret:") {
        rest = &rest[start + 9..];
        if let Some(end) = rest.find("}}") {
            names.insert(rest[..end].to_owned());
            rest = &rest[end + 2..];
        } else {
            break;
        }
    }
    names
}

pub fn set(name: &str, value: &str) -> Result<()> {
    Ok(keyring::Entry::new(SERVICE, name)?.set_password(value)?)
}

pub fn delete(name: &str) -> Result<()> {
    Ok(keyring::Entry::new(SERVICE, name)?.delete_credential()?)
}

/// Collects resolved secret values and replaces them in any outgoing text.
#[derive(Default)]
pub struct Redactor {
    values: Vec<(String, String)>, // (name, value)
}

impl Redactor {
    pub fn track(&mut self, name: &str, value: &str) {
        if !value.is_empty() {
            self.values.push((name.to_owned(), value.to_owned()));
        }
    }

    pub fn redact(&self, text: &str) -> String {
        self.values
            .iter()
            .fold(text.to_owned(), |acc, (_, v)| acc.replace(v, MASK))
    }

    pub fn names(&self) -> Vec<&str> {
        self.values.iter().map(|(n, _)| n.as_str()).collect()
    }
}
