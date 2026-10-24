//! Secrets live in the macOS Keychain (service "kestrl"), referenced as
//! `{{secret:name}}`. Resolved values are tracked so output can be masked.

use crate::Result;

const SERVICE: &str = "kestrl";
pub const MASK: &str = "••••••";

pub fn get(name: &str) -> Result<String> {
    Ok(keyring::Entry::new(SERVICE, name)?.get_password()?)
}

pub fn set(name: &str, value: &str) -> Result<()> {
    Ok(keyring::Entry::new(SERVICE, name)?.set_password(value)?)
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
