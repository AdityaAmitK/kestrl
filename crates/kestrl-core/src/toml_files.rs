//! Shared helpers for editing workspace TOML files (environments, profiles, connections).
//!
//! Edits go through `toml_edit`, so comments, ordering and unrelated keys are kept, and every
//! write replaces the file atomically (write to a sibling, then rename).

use std::path::Path;

use toml_edit::DocumentMut;
use toml_edit::Item;
use toml_edit::Table;
use toml_edit::value;

use crate::Error;
use crate::Result;

pub(crate) fn read_text(path: &Path) -> Result<String> {
    std::fs::read_to_string(path).map_err(|source| Error::Read {
        path: path.to_owned(),
        source,
    })
}

pub(crate) fn read_document(path: &Path) -> Result<DocumentMut> {
    read_text(path)?
        .parse::<DocumentMut>()
        .map_err(|error| Error::TomlEdit {
            path: path.to_owned(),
            message: error.to_string(),
        })
}

/// The document at `path`, or an empty one when the file does not exist yet.
pub(crate) fn read_document_or_new(path: &Path) -> Result<DocumentMut> {
    if path.exists() {
        read_document(path)
    } else {
        Ok(DocumentMut::new())
    }
}

/// Write via a sibling temp file and rename, so a crash never leaves a half-written file.
pub(crate) fn write_atomic(path: &Path, contents: &str) -> Result<()> {
    let write_error = |source| Error::Write {
        path: path.to_owned(),
        source,
    };
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(write_error)?;
    }
    let temporary = path.with_extension("toml.kestrl-tmp");
    std::fs::write(&temporary, contents).map_err(write_error)?;
    std::fs::rename(&temporary, path).map_err(write_error)
}

/// File-backed names (environments, profiles, connections): letters, digits, `-`, `_`, `.`,
/// starting with a letter or digit, at most 64 characters.
pub fn validate_file_name(kind: &'static str, name: &str) -> Result<()> {
    let is_valid = name.len() <= 64
        && name
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphanumeric())
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'));
    if is_valid {
        Ok(())
    } else {
        Err(Error::InvalidName {
            kind,
            name: name.to_owned(),
            rule: "use letters, digits, - _ or . (start with a letter or digit, at most 64)",
        })
    }
}

/// The table at `key`, created if it is missing. Errors when `key` holds something else.
pub(crate) fn table_mut<'a>(
    document: &'a mut DocumentMut,
    key: &str,
    path: &Path,
) -> Result<&'a mut Table> {
    if !document.contains_key(key) {
        document[key] = Item::Table(Table::new());
    }
    document[key].as_table_mut().ok_or_else(|| Error::TomlEdit {
        path: path.to_owned(),
        message: format!("[{key}] is not a table"),
    })
}

/// Make `table`'s string entries exactly `pairs`: keys that stay keep their comments and
/// position, removed keys go, new keys are appended in order.
pub(crate) fn sync_string_table(table: &mut Table, pairs: &[(String, String)]) {
    let removed: Vec<String> = table
        .iter()
        .map(|(key, _)| key.to_owned())
        .filter(|key| !pairs.iter().any(|(name, _)| name == key))
        .collect();
    for key in removed {
        table.remove(&key);
    }
    for (name, text) in pairs {
        match table.get_mut(name).and_then(Item::as_value_mut) {
            Some(existing) => {
                let decor = existing.decor().clone();
                *existing = toml_edit::Value::from(text.as_str());
                *existing.decor_mut() = decor;
            }
            None => {
                table.insert(name, value(text.as_str()));
            }
        }
    }
}

/// String entries of the table at `key`, in file order; empty when it is absent.
pub(crate) fn string_pairs(document: &DocumentMut, key: &str) -> Vec<(String, String)> {
    document
        .get(key)
        .and_then(Item::as_table)
        .map(|table| {
            table
                .iter()
                .filter_map(|(name, item)| {
                    item.as_str().map(|text| (name.to_owned(), text.to_owned()))
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Reject duplicate keys before saving a table.
pub(crate) fn reject_duplicates(kind: &'static str, pairs: &[(String, String)]) -> Result<()> {
    for (index, (name, _)) in pairs.iter().enumerate() {
        if pairs[..index].iter().any(|(other, _)| other == name) {
            return Err(Error::InvalidName {
                kind,
                name: name.clone(),
                rule: "each name may appear only once",
            });
        }
    }
    Ok(())
}
