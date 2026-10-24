//! Request files: `*.req.toml` (see PLAN.md). Interpolation and sending land here.

use std::collections::BTreeMap;

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct RequestFile {
    pub name: Option<String>,
    pub method: String,
    pub url: String,
    #[serde(default)]
    pub headers: BTreeMap<String, String>,
    #[serde(default)]
    pub query: BTreeMap<String, String>,
    pub body: Option<Body>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Body {
    Json { content: String },
    Text { content: String },
    Form { fields: BTreeMap<String, String> },
}
