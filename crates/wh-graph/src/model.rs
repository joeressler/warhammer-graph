use serde::{Deserialize, Serialize};

use crate::attrs::Attrs;

/// One node in `nodes.jsonl` and in the Cozo `node` relation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphNode {
    pub id: String,
    pub kind: String,
    pub label: String,
    pub text: String,
    pub attrs: Attrs,
    pub source_url: Option<String>,
}

/// One edge in `edges.jsonl`. Field names follow the JSONL contract.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EdgeRecord {
    pub id: String,
    pub kind: String,
    pub from: String,
    pub to: String,
    pub attrs: Attrs,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Passage {
    pub node_id: String,
    pub title: String,
    pub text: String,
    pub wahapedia_link: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BundleManifest {
    pub format_version: u32,
    pub corpus_schema_version: u32,
    pub edition: String,
    pub corpus_fingerprint: String,
    pub last_update: String,
    pub node_count: usize,
    pub edge_count: usize,
    pub passage_count: usize,
    pub node_counts_by_kind: std::collections::BTreeMap<String, usize>,
    pub edge_counts_by_kind: std::collections::BTreeMap<String, usize>,
}
