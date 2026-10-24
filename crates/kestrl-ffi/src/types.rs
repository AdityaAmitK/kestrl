//! Records shared by several FFI areas. Maps and tuples cross as `KeyValue` lists.

use std::collections::BTreeMap;

/// One header, query value, environment variable or `--var` override.
#[derive(Debug, uniffi::Record)]
pub struct KeyValue {
    pub key: String,
    pub value: String,
}

impl KeyValue {
    pub(crate) fn from_map(map: BTreeMap<String, String>) -> Vec<Self> {
        map.into_iter()
            .map(|(key, value)| Self { key, value })
            .collect()
    }

    pub(crate) fn from_pairs(pairs: Vec<(String, String)>) -> Vec<Self> {
        pairs
            .into_iter()
            .map(|(key, value)| Self { key, value })
            .collect()
    }

    pub(crate) fn into_map(pairs: Vec<Self>) -> BTreeMap<String, String> {
        pairs
            .into_iter()
            .map(|pair| (pair.key, pair.value))
            .collect()
    }
}
