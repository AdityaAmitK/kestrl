//! Creating and editing environment files (`.kestrl/envs/<name>.toml`) and the default
//! environment in `.kestrl/kestrl.toml`.
//!
//! Edits go through `toml_edit`, so comments, ordering and unrelated keys in the files are
//! kept. Every write replaces the file atomically (write to a sibling, then rename).

use std::path::Path;
use std::path::PathBuf;

use toml_edit::DocumentMut;
use toml_edit::Item;
use toml_edit::Table;
use toml_edit::value;

use crate::Error;
use crate::Result;
use crate::env::Environment;
use crate::workspace::WorkspaceConfig;

/// One variable as written in an environment file, before any resolution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawVariable {
    pub name: String,
    pub value: String,
}

/// The variables of `name` exactly as written, in file order.
pub fn variables(root: &Path, name: &str) -> Result<Vec<RawVariable>> {
    let path = env_path(root, name)?;
    let document = read_document(&path)?;
    let Some(table) = document.get("vars").and_then(Item::as_table) else {
        return Ok(Vec::new());
    };
    Ok(table
        .iter()
        .filter_map(|(key, item)| {
            item.as_str().map(|text| RawVariable {
                name: key.to_owned(),
                value: text.to_owned(),
            })
        })
        .collect())
}

/// Create an environment, empty or as a copy of `copy_from` (comments included).
pub fn create(root: &Path, name: &str, copy_from: Option<&str>) -> Result<()> {
    let path = env_path(root, name)?;
    if path.exists() {
        return Err(Error::EnvironmentExists {
            name: name.to_owned(),
        });
    }
    let contents = match copy_from {
        Some(source) => read_text(&env_path(root, source)?)?,
        None => "[vars]\n".to_owned(),
    };
    write_atomic(&path, &contents)
}

/// Replace the variables of `name` with `variables`, keeping comments on variables that stay.
pub fn save_variables(root: &Path, name: &str, variables: &[RawVariable]) -> Result<()> {
    let path = env_path(root, name)?;
    for (index, variable) in variables.iter().enumerate() {
        validate_variable_name(&variable.name)?;
        if variables[..index]
            .iter()
            .any(|other| other.name == variable.name)
        {
            return Err(Error::InvalidName {
                kind: "variable",
                name: variable.name.clone(),
                rule: "each variable name may appear only once",
            });
        }
    }
    let mut document = read_document(&path)?;
    if !document.contains_key("vars") || document["vars"].as_table().is_none() {
        document["vars"] = Item::Table(Table::new());
    }
    let Some(table) = document["vars"].as_table_mut() else {
        return Err(Error::TomlEdit {
            path,
            message: "[vars] is not a table".to_owned(),
        });
    };
    let kept: Vec<String> = table
        .iter()
        .map(|(key, _)| key.to_owned())
        .filter(|key| !variables.iter().any(|variable| &variable.name == key))
        .collect();
    for key in kept {
        table.remove(&key);
    }
    for variable in variables {
        match table.get_mut(&variable.name).and_then(Item::as_value_mut) {
            // Replacing only the value keeps the key's surrounding comments and position.
            Some(existing) => {
                let decor = existing.decor().clone();
                *existing = toml_edit::Value::from(variable.value.as_str());
                *existing.decor_mut() = decor;
            }
            None => {
                table.insert(&variable.name, value(variable.value.as_str()));
            }
        }
    }
    write_atomic(&path, &document.to_string())
}

/// Rename an environment, moving the default to the new name when needed.
pub fn rename(root: &Path, from: &str, to: &str) -> Result<()> {
    let source = env_path(root, from)?;
    let target = env_path(root, to)?;
    if !source.exists() {
        return Err(not_found(root, from));
    }
    if from == to {
        return Ok(());
    }
    if target.exists() {
        return Err(Error::EnvironmentExists {
            name: to.to_owned(),
        });
    }
    std::fs::rename(&source, &target).map_err(|source| Error::Write {
        path: target.clone(),
        source,
    })?;
    if WorkspaceConfig::load(root)?.default_env.as_deref() == Some(from) {
        set_default(root, to)?;
    }
    Ok(())
}

/// Delete an environment. The default environment is protected.
pub fn delete(root: &Path, name: &str) -> Result<()> {
    let path = env_path(root, name)?;
    if !path.exists() {
        return Err(not_found(root, name));
    }
    if WorkspaceConfig::load(root)?.default_env.as_deref() == Some(name) {
        return Err(Error::DefaultEnvironmentInUse {
            name: name.to_owned(),
        });
    }
    std::fs::remove_file(&path).map_err(|source| Error::Write { path, source })
}

/// Make `name` the default environment in `.kestrl/kestrl.toml`.
pub fn set_default(root: &Path, name: &str) -> Result<()> {
    if !env_path(root, name)?.exists() {
        return Err(not_found(root, name));
    }
    let path = root.join(".kestrl/kestrl.toml");
    let mut document = if path.exists() {
        read_document(&path)?
    } else {
        DocumentMut::new()
    };
    document["default_env"] = value(name);
    write_atomic(&path, &document.to_string())
}

/// Environment names: letters, digits, `-`, `_` and `.`, starting with a letter or digit.
pub fn validate_environment_name(name: &str) -> Result<()> {
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
            kind: "environment",
            name: name.to_owned(),
            rule: "use letters, digits, - _ or . (start with a letter or digit, at most 64)",
        })
    }
}

/// Variable names: usable inside `{{…}}`, so letters, digits and `_`, not starting with a digit.
pub fn validate_variable_name(name: &str) -> Result<()> {
    let is_valid = name.len() <= 128
        && name
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
    if is_valid {
        Ok(())
    } else {
        Err(Error::InvalidName {
            kind: "variable",
            name: name.to_owned(),
            rule: "use letters, digits and _, not starting with a digit",
        })
    }
}

fn env_path(root: &Path, name: &str) -> Result<PathBuf> {
    validate_environment_name(name)?;
    Ok(root.join(".kestrl/envs").join(format!("{name}.toml")))
}

fn not_found(root: &Path, name: &str) -> Error {
    Error::EnvironmentNotFound {
        name: name.to_owned(),
        available: Environment::names(root)
            .map(|names| names.join(", "))
            .unwrap_or_default(),
    }
}

fn read_text(path: &Path) -> Result<String> {
    std::fs::read_to_string(path).map_err(|source| Error::Read {
        path: path.to_owned(),
        source,
    })
}

fn read_document(path: &Path) -> Result<DocumentMut> {
    read_text(path)?
        .parse::<DocumentMut>()
        .map_err(|error| Error::TomlEdit {
            path: path.to_owned(),
            message: error.to_string(),
        })
}

/// Write via a sibling temp file and rename, so a crash never leaves a half-written file.
fn write_atomic(path: &Path, contents: &str) -> Result<()> {
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
