//! Secrets live in the macOS Keychain (service "kestrl"), referenced as
//! `{{secret:name}}`. Resolved values are tracked so output can be masked.

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

pub fn referenced_names(text: &str) -> std::collections::BTreeSet<String> {
    let mut names = std::collections::BTreeSet::new();
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
