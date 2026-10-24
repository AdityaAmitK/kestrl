//! Loading database connection definitions from `.kestrl/connections`.

use std::collections::BTreeMap;
use std::path::Path;

use serde::Deserialize;

use crate::vars::Variables;
use crate::workspace;
use crate::{Error, Result};

#[derive(Debug, Deserialize)]
pub struct ConnectionConfig {
    pub kind: String,
    #[serde(default)]
    pub allow_writes: Vec<String>,
    #[serde(default)]
    pub env: BTreeMap<String, ConnectionEnvironment>,
}

#[derive(Debug, Deserialize)]
pub struct ConnectionEnvironment {
    pub url: String,
}

#[derive(Debug)]
pub struct ConnectionInfo {
    pub name: String,
    pub env: String,
    pub url: String,
    pub is_read_only: bool,
}

impl ConnectionConfig {
    pub fn load(workspace: &Path, name: &str) -> Result<Self> {
        let path = workspace
            .join(".kestrl/connections")
            .join(format!("{name}.toml"));
        let text = std::fs::read_to_string(&path).map_err(|source| {
            if source.kind() == std::io::ErrorKind::NotFound {
                Error::UnknownConnection(name.to_owned())
            } else {
                Error::Read {
                    path: path.clone(),
                    source,
                }
            }
        })?;
        toml::from_str(&text).map_err(|source| Error::Toml { path, source })
    }

    pub fn names(workspace: &Path) -> Result<Vec<String>> {
        let dir = workspace.join(".kestrl/connections");
        match workspace::names(&dir) {
            Ok(names) => Ok(names),
            Err(Error::Read { source, .. }) if source.kind() == std::io::ErrorKind::NotFound => {
                Ok(Vec::new())
            }
            Err(error) => Err(error),
        }
    }

    pub fn resolve(
        self,
        name: &str,
        env: &str,
        variables: &mut Variables,
    ) -> Result<ConnectionInfo> {
        if self.kind != "postgres" {
            return Err(Error::UnsupportedConnectionKind(self.kind));
        }
        let entry = self
            .env
            .get(env)
            .ok_or_else(|| Error::ConnectionEnvironment {
                connection: name.to_owned(),
                env: env.to_owned(),
            })?;
        let url = variables.resolve(&entry.url)?;
        Ok(ConnectionInfo {
            name: name.to_owned(),
            env: env.to_owned(),
            url,
            is_read_only: !self.allow_writes.iter().any(|allowed| allowed == env),
        })
    }
}
