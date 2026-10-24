//! Secrets live in the macOS Keychain (service "kestrl"), referenced as
//! `{{secret:name}}`. Resolved values are tracked so output can be masked.

use crate::{Error, Result};
use std::path::Path;

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

/// Resolve every secret referenced by TOML files in a workspace for output masking.
pub fn workspace_redactor(root: &Path) -> Result<Redactor> {
    let names = workspace_references(root)?;
    let mut redactor = Redactor::default();
    for name in names {
        if let Ok(value) = get(&name) {
            redactor.track(&name, &value);
        }
    }
    Ok(redactor)
}

/// Find secret references in workspace configuration and request files.
pub fn workspace_references(root: &Path) -> Result<std::collections::BTreeSet<String>> {
    fn scan(path: &Path, names: &mut std::collections::BTreeSet<String>) -> Result<()> {
        let entries = match std::fs::read_dir(path) {
            Ok(entries) => entries,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(source) => {
                return Err(Error::Read {
                    path: path.to_owned(),
                    source,
                });
            }
        };
        for entry in entries {
            let entry = entry.map_err(|source| Error::Read {
                path: path.to_owned(),
                source,
            })?;
            let item = entry.path();
            if item.is_dir() {
                let basename = item
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or_default();
                if matches!(basename, "target" | "node_modules" | ".git")
                    || basename.starts_with('.') && basename != ".kestrl"
                {
                    continue;
                }
                scan(&item, names)?;
            } else if item
                .extension()
                .is_some_and(|extension| extension == "toml")
            {
                let is_request = item
                    .file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.ends_with(".req.toml"));
                let in_config = item
                    .ancestors()
                    .any(|parent| parent.file_name().is_some_and(|name| name == ".kestrl"));
                if is_request || in_config {
                    let contents =
                        std::fs::read_to_string(&item).map_err(|source| Error::Read {
                            path: item.clone(),
                            source,
                        })?;
                    names.extend(referenced_names(&contents));
                }
            }
        }
        Ok(())
    }
    let mut names = std::collections::BTreeSet::new();
    scan(root, &mut names)?;
    Ok(names)
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
