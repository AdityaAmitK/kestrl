//! Request files and resolved HTTP request construction.

use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::profile;
use crate::vars::Variables;
use crate::workspace::WorkspaceConfig;
use crate::{Error, Result};

#[derive(Debug, Deserialize, Serialize)]
pub struct RequestFile {
    pub name: Option<String>,
    pub method: String,
    pub url: String,
    #[serde(default)]
    pub profiles: Vec<String>,
    #[serde(default)]
    pub headers: BTreeMap<String, String>,
    #[serde(default)]
    pub query: BTreeMap<String, String>,
    pub body: Option<Body>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Body {
    Json { content: String },
    Text { content: String },
    Form { fields: BTreeMap<String, String> },
}

pub struct ResolvedRequest {
    pub method: String,
    pub url: String,
    pub headers: BTreeMap<String, String>,
    pub query: BTreeMap<String, String>,
    pub body: Option<String>,
    pub redactor: crate::secrets::Redactor,
}

pub fn load(path: &Path) -> Result<RequestFile> {
    let text = std::fs::read_to_string(path).map_err(|source| Error::Read {
        path: path.to_owned(),
        source,
    })?;
    toml::from_str(&text).map_err(|source| Error::Toml {
        path: path.to_owned(),
        source,
    })
}

pub fn ad_hoc(method: &str, url: &str) -> RequestFile {
    RequestFile {
        name: None,
        method: method.to_owned(),
        url: url.to_owned(),
        profiles: Vec::new(),
        headers: BTreeMap::new(),
        query: BTreeMap::new(),
        body: None,
    }
}

pub fn resolve(
    request: RequestFile,
    root: &Path,
    config: &WorkspaceConfig,
    env: &str,
    cli_profiles: &[String],
    cli_headers: &BTreeMap<String, String>,
    vars: Variables,
) -> Result<ResolvedRequest> {
    let mut resolved_vars = vars;
    for (name, value) in &config.vars {
        resolved_vars
            .workspace
            .entry(name.clone())
            .or_insert_with(|| value.clone());
    }
    let url = resolved_vars.resolve(&request.url)?;
    let profile_url = url.clone();
    let mut headers = request.headers.clone();
    let mut query = request.query.clone();
    let names = request
        .profiles
        .iter()
        .cloned()
        .chain(cli_profiles.iter().cloned())
        .collect::<Vec<_>>();
    for name in names {
        let item = profile::load(root, &name)?;
        merge_profile(&mut headers, &mut query, item);
    }
    let mut automatic = Vec::new();
    let dir = root.join(".kestrl/profiles");
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path
                .extension()
                .is_some_and(|extension| extension == "toml")
            {
                let name = path
                    .file_stem()
                    .and_then(|stem| stem.to_str())
                    .unwrap_or_default()
                    .to_owned();
                let item = profile::load(root, &name)?;
                if profile::matches(&item, env, &profile_url) {
                    automatic.push((name, item));
                }
            }
        }
    }
    automatic.sort_by(|a, b| a.0.cmp(&b.0));
    for (_, item) in automatic {
        merge_profile(&mut headers, &mut query, item);
    }
    for (name, value) in cli_headers {
        headers.retain(|existing, _| !existing.eq_ignore_ascii_case(name));
        headers.insert(name.clone(), value.clone());
    }
    headers = headers
        .into_iter()
        .map(|(key, value)| Ok((key, resolved_vars.resolve(&value)?)))
        .collect::<Result<_>>()?;
    query = query
        .into_iter()
        .map(|(key, value)| Ok((key, resolved_vars.resolve(&value)?)))
        .collect::<Result<_>>()?;
    let body = match request.body {
        Some(Body::Json { content } | Body::Text { content }) => {
            Some(resolved_vars.resolve(&content)?)
        }
        Some(Body::Form { fields }) => Some(
            fields
                .into_iter()
                .map(|(key, value)| {
                    Ok(format!(
                        "{}={}",
                        urlencoding::encode(&key),
                        urlencoding::encode(&resolved_vars.resolve(&value)?)
                    ))
                })
                .collect::<Result<Vec<_>>>()?
                .join("&"),
        ),
        None => None,
    };
    Ok(ResolvedRequest {
        method: request.method,
        url,
        headers,
        query,
        body,
        redactor: resolved_vars.redactor,
    })
}

fn merge_profile(
    headers: &mut BTreeMap<String, String>,
    query: &mut BTreeMap<String, String>,
    item: profile::Profile,
) {
    for (key, value) in item.headers {
        headers.insert(key, value);
    }
    for (key, value) in item.query {
        query.insert(key, value);
    }
}
