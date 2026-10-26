//! Variable and secret interpolation with workspace, environment, and CLI precedence.
//! `{{secret:name}}` reads a stored secret and `{{env:NAME}}` a process environment variable;
//! both values are masked in output and history.

use std::collections::{BTreeMap, BTreeSet};

use crate::secrets::{self, Redactor};
use crate::{Error, Result};

#[derive(Default)]
pub struct Variables {
    /// `--var` values; highest precedence
    pub overrides: BTreeMap<String, String>,
    /// Variables from the request's collection folders (`folder.toml`), nearest folder winning
    pub collection: BTreeMap<String, String>,
    pub environment: BTreeMap<String, String>,
    pub workspace: BTreeMap<String, String>,
    pub redactor: Redactor,
}

impl Variables {
    pub fn resolve(&mut self, input: &str) -> Result<String> {
        self.expand(input, &mut BTreeSet::new(), 0, false)
    }

    /// Expand variables while preserving secret references, without reading secret values.
    pub fn preview_template(&mut self, input: &str) -> Result<String> {
        self.expand(input, &mut BTreeSet::new(), 0, true)
    }

    fn expand(
        &mut self,
        input: &str,
        stack: &mut BTreeSet<String>,
        depth: usize,
        masked: bool,
    ) -> Result<String> {
        if depth > 8 {
            return Err(Error::VariableCycle("maximum depth".to_owned()));
        }
        let mut output = String::new();
        let mut rest = input;
        while let Some(start) = rest.find("{{") {
            output.push_str(&rest[..start]);
            let after = &rest[start + 2..];
            let end = after
                .find("}}")
                .ok_or_else(|| Error::InvalidRequest("unclosed variable".to_owned()))?;
            let name = after[..end].trim();
            if let Some(secret_name) = name.strip_prefix("secret:") {
                if masked {
                    output.push_str(&format!("{{{{secret:{secret_name}}}}}"));
                    rest = &after[end + 2..];
                    continue;
                }
                let value = secrets::get(secret_name)?;
                self.redactor.track(secret_name, &value);
                output.push_str(&value);
            } else if let Some(variable) = name.strip_prefix("env:") {
                if masked {
                    output.push_str(&format!("{{{{env:{variable}}}}}"));
                    rest = &after[end + 2..];
                    continue;
                }
                let value = std::env::var(variable)
                    .map_err(|_| Error::EnvironmentVariableNotSet(variable.to_owned()))?;
                self.redactor.track(name, &value);
                output.push_str(&value);
            } else {
                if !stack.insert(name.to_owned()) {
                    return Err(Error::VariableCycle(name.to_owned()));
                }
                let value = self
                    .overrides
                    .get(name)
                    .or_else(|| self.collection.get(name))
                    .or_else(|| self.environment.get(name))
                    .or_else(|| self.workspace.get(name))
                    .ok_or_else(|| Error::UnknownVariable(name.to_owned()))?
                    .clone();
                output.push_str(&self.expand(&value, stack, depth + 1, masked)?);
                stack.remove(name);
            }
            rest = &after[end + 2..];
        }
        output.push_str(rest);
        Ok(output)
    }
}
