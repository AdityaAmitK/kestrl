//! Creating and editing environment files (`.kestrl/envs/<name>.toml`) and the default
//! environment in `.kestrl/kestrl.toml`.
//!
//! Edits keep comments and ordering and write atomically (see `toml_files`).

use std::path::Path;
use std::path::PathBuf;

use toml_edit::value;

use crate::Error;
use crate::Result;
use crate::env::Environment;
use crate::toml_files;
use crate::toml_files::read_document;
use crate::toml_files::read_text;
use crate::toml_files::write_atomic;
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
    Ok(toml_files::string_pairs(&document, "vars")
        .into_iter()
        .map(|(name, value)| RawVariable { name, value })
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
    let pairs: Vec<(String, String)> = variables
        .iter()
        .map(|variable| (variable.name.clone(), variable.value.clone()))
        .collect();
    for (name, _) in &pairs {
        validate_variable_name(name)?;
    }
    toml_files::reject_duplicates("variable", &pairs)?;
    let mut document = read_document(&path)?;
    toml_files::sync_string_table(toml_files::table_mut(&mut document, "vars", &path)?, &pairs);
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
    let mut document = toml_files::read_document_or_new(&path)?;
    document["default_env"] = value(name);
    write_atomic(&path, &document.to_string())
}

/// Environment names follow the shared file-name rules.
pub fn validate_environment_name(name: &str) -> Result<()> {
    toml_files::validate_file_name("environment", name)
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
