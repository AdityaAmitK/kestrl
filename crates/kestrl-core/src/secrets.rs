//! Secrets are referenced as `{{secret:name}}` and stored outside the workspace in
//! `secrets.toml` in Kestrl's data directory, readable only by the owner. Kestrl sets, removes
//! and lists them but never shows a value; resolved values are tracked so output can be masked.

use std::collections::{BTreeMap, BTreeSet};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::{Error, Result};

const FILE: &str = "secrets.toml";
const HEADER: &str =
    "# Kestrl secrets, referenced as {{secret:name}}. Do not commit or share this file.\n";
pub const MASK: &str = "••••••";

/// Where secrets are stored: `secrets.toml` in the data directory.
pub fn path() -> Result<PathBuf> {
    Ok(crate::paths::data_dir()?.join(FILE))
}

pub fn get(name: &str) -> Result<String> {
    load()?.remove(name).ok_or_else(|| Error::SecretNotSet {
        name: name.to_owned(),
    })
}

/// Names of every stored secret.
pub fn stored_names() -> Result<BTreeSet<String>> {
    Ok(load()?.into_keys().collect())
}

pub fn set(name: &str, value: &str) -> Result<()> {
    let mut secrets = load()?;
    secrets.insert(name.to_owned(), value.to_owned());
    save(&secrets)
}

pub fn delete(name: &str) -> Result<()> {
    let mut secrets = load()?;
    if secrets.remove(name).is_none() {
        return Err(Error::SecretNotSet {
            name: name.to_owned(),
        });
    }
    save(&secrets)
}

fn load() -> Result<BTreeMap<String, String>> {
    let path = path()?;
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(BTreeMap::new()),
        Err(source) => return Err(Error::Read { path, source }),
    };
    toml::from_str(&text).map_err(|source| Error::Toml { path, source })
}

/// Replace the secrets file in one rename, with owner-only permissions.
fn save(secrets: &BTreeMap<String, String>) -> Result<()> {
    let path = path()?;
    let text = toml::to_string(secrets).map_err(|error| Error::TomlEdit {
        path: path.clone(),
        message: error.to_string(),
    })?;
    let temporary = path.with_extension("toml.tmp");
    let write = || -> std::io::Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let _ = std::fs::remove_file(&temporary);
        let mut options = OpenOptions::new();
        options.create_new(true).write(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options.open(&temporary)?;
        file.write_all(HEADER.as_bytes())?;
        file.write_all(text.as_bytes())?;
        file.sync_all()?;
        std::fs::rename(&temporary, &path)
    };
    write().map_err(|source| Error::Write {
        path: path.clone(),
        source,
    })
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

/// Resolve every secret referenced by TOML files in a workspace for output masking.
pub fn workspace_redactor(root: &Path) -> Result<Redactor> {
    let names = referenced_in_workspace(root)?;
    let stored = load()?;
    let mut redactor = Redactor::default();
    for name in names {
        if let Some(value) = stored.get(&name) {
            redactor.track(&name, value);
        }
    }
    Ok(redactor)
}

/// Collects resolved secret values and replaces them in any outgoing text.
#[derive(Default)]
pub struct Redactor {
    values: Vec<(String, String)>, // (name, value)
}

impl Redactor {
    /// Mask `value` as it is and as it appears percent-encoded or form-encoded, so a secret in a
    /// query string or form body is masked too.
    pub fn track(&mut self, name: &str, value: &str) {
        if value.is_empty() {
            return;
        }
        let encoded = urlencoding::encode(value).into_owned();
        let form = encoded.replace("%20", "+");
        for variant in [value.to_owned(), encoded, form] {
            if !self.values.iter().any(|(_, known)| *known == variant) {
                self.values.push((name.to_owned(), variant));
            }
        }
    }

    pub fn redact(&self, text: &str) -> String {
        self.values
            .iter()
            .fold(text.to_owned(), |acc, (_, v)| acc.replace(v, MASK))
    }

    /// Mask a URL or a message containing one. The URL parser encodes only some characters, so a
    /// secret can survive in a form that matches none of the tracked variants; when the decoded
    /// text still contains one, the decoded text is masked instead.
    pub fn redact_url(&self, text: &str) -> String {
        let redacted = self.redact(text);
        match urlencoding::decode(&redacted) {
            Ok(decoded)
                if self
                    .values
                    .iter()
                    .any(|(_, value)| decoded.contains(value.as_str())) =>
            {
                self.redact(&decoded)
            }
            _ => redacted,
        }
    }

    /// The masked names, each once, in the order they were first tracked.
    pub fn names(&self) -> Vec<&str> {
        let mut names: Vec<&str> = Vec::new();
        for (name, _) in &self.values {
            if !names.contains(&name.as_str()) {
                names.push(name);
            }
        }
        names
    }
}
