use std::collections::HashMap;
use std::fs;
use std::path::Path;

use petgraph::stable_graph::{NodeIndex, StableGraph};
use sha2::{Digest, Sha256};
use wh_graph::{BundleManifest, EdgeRecord, GraphEdge, GraphNode, Passage};

use crate::error::AskError;

/// A format-version-1 bundle. Node payloads come from the postcard snapshot
/// when that snapshot matches the JSONL, and from the JSONL rebuild otherwise.
pub struct LoadedBundle {
    pub manifest: BundleManifest,
    pub edges: Vec<EdgeRecord>,
    pub passages: Vec<Passage>,
    pub nodes_sha256: [u8; 32],
    pub passages_sha256: [u8; 32],
    graph: StableGraph<GraphNode, GraphEdge>,
    node_index: HashMap<String, NodeIndex>,
}

impl LoadedBundle {
    pub fn node(&self, id: &str) -> Option<&GraphNode> {
        self.node_index.get(id).map(|index| &self.graph[*index])
    }

    pub fn fingerprint_bytes(&self) -> [u8; 64] {
        fingerprint_bytes(&self.manifest.corpus_fingerprint)
    }
}

pub fn load_bundle(dir: &Path) -> Result<LoadedBundle, AskError> {
    let manifest_path = dir.join("manifest.json");
    let manifest_bytes =
        fs::read(&manifest_path).map_err(|err| AskError::bundle(&manifest_path, err))?;
    let manifest: BundleManifest = serde_json::from_slice(&manifest_bytes)
        .map_err(|err| AskError::bundle(&manifest_path, format!("malformed JSON: {err}")))?;
    if manifest.format_version != 1 {
        return Err(AskError::bundle(
            &manifest_path,
            format!(
                "format_version is not 1 (found {})",
                manifest.format_version
            ),
        ));
    }

    let nodes_path = dir.join("nodes.jsonl");
    let nodes_bytes = fs::read(&nodes_path).map_err(|err| AskError::bundle(&nodes_path, err))?;
    let nodes: Vec<GraphNode> = read_jsonl(&nodes_path, &nodes_bytes)?;

    let edges_path = dir.join("edges.jsonl");
    let edges_bytes = fs::read(&edges_path).map_err(|err| AskError::bundle(&edges_path, err))?;
    let edges: Vec<EdgeRecord> = read_jsonl(&edges_path, &edges_bytes)?;

    let passages_path = dir.join("passages.jsonl");
    let passages_bytes =
        fs::read(&passages_path).map_err(|err| AskError::bundle(&passages_path, err))?;
    let passages: Vec<Passage> = read_jsonl(&passages_path, &passages_bytes)?;

    let jsonl_graph = build_graph(&nodes_path, &nodes, &edges)?;
    let graph = match read_postcard(dir, &nodes, &edges) {
        Some(decoded) => decoded,
        None => jsonl_graph,
    };
    let mut node_index = HashMap::new();
    for index in graph.node_indices() {
        node_index.insert(graph[index].id.clone(), index);
    }

    Ok(LoadedBundle {
        manifest,
        edges,
        passages,
        nodes_sha256: sha256(&nodes_bytes),
        passages_sha256: sha256(&passages_bytes),
        graph,
        node_index,
    })
}

fn read_postcard(
    dir: &Path,
    nodes: &[GraphNode],
    edges: &[EdgeRecord],
) -> Option<StableGraph<GraphNode, GraphEdge>> {
    let path = dir.join("graph.postcard");
    let bytes = fs::read(path).ok()?;
    let decoded: StableGraph<GraphNode, GraphEdge> = postcard::from_bytes(&bytes).ok()?;
    if payloads_match(&decoded, nodes, edges) {
        Some(decoded)
    } else {
        None
    }
}

fn payloads_match(
    graph: &StableGraph<GraphNode, GraphEdge>,
    nodes: &[GraphNode],
    edges: &[EdgeRecord],
) -> bool {
    use std::collections::HashSet;
    let post_nodes = graph
        .node_weights()
        .map(node_payload)
        .collect::<HashSet<_>>();
    if post_nodes.len() != nodes.len() {
        return false;
    }
    if nodes
        .iter()
        .any(|node| !post_nodes.contains(&node_payload(node)))
    {
        return false;
    }
    let post_edges = graph
        .edge_weights()
        .map(graph_edge_payload)
        .collect::<HashSet<_>>();
    if post_edges.len() != edges.len() {
        return false;
    }
    edges
        .iter()
        .all(|edge| post_edges.contains(&edge_payload(edge)))
}

fn node_payload(node: &GraphNode) -> (String, String, String, String, Option<String>) {
    (
        node.id.clone(),
        node.kind.clone(),
        node.label.clone(),
        node.text.clone(),
        node.source_url.clone(),
    )
}

fn edge_payload(edge: &EdgeRecord) -> (String, String, String, String, String) {
    (
        edge.id.clone(),
        edge.kind.clone(),
        edge.from.clone(),
        edge.to.clone(),
        canonical_attrs(&edge.attrs),
    )
}

fn graph_edge_payload(edge: &GraphEdge) -> (String, String, String, String, String) {
    (
        edge.id.clone(),
        edge.kind.clone(),
        edge.from_id.clone(),
        edge.to_id.clone(),
        canonical_attrs(&edge.attrs),
    )
}

fn canonical_attrs(attrs: &wh_graph::Attrs) -> String {
    serde_json::to_string(attrs).unwrap_or_else(|_| "{}".to_string())
}

fn build_graph(
    nodes_path: &Path,
    nodes: &[GraphNode],
    edges: &[EdgeRecord],
) -> Result<StableGraph<GraphNode, GraphEdge>, AskError> {
    let mut graph = StableGraph::default();
    let mut index = HashMap::new();
    for node in nodes {
        if index.contains_key(&node.id) {
            return Err(AskError::bundle(
                nodes_path,
                format!("duplicate node id {}", node.id),
            ));
        }
        let node_index = graph.add_node(node.clone());
        index.insert(node.id.clone(), node_index);
    }
    for edge in edges {
        let Some(from) = index.get(&edge.from).copied() else {
            return Err(AskError::bundle(
                nodes_path,
                format!("edge endpoint is not a node id ({})", edge.from),
            ));
        };
        let Some(to) = index.get(&edge.to).copied() else {
            return Err(AskError::bundle(
                nodes_path,
                format!("edge endpoint is not a node id ({})", edge.to),
            ));
        };
        graph.add_edge(from, to, edge.to_graph_edge());
    }
    Ok(graph)
}

fn read_jsonl<T: serde::de::DeserializeOwned>(
    path: &Path,
    bytes: &[u8],
) -> Result<Vec<T>, AskError> {
    let text = std::str::from_utf8(bytes)
        .map_err(|err| AskError::bundle(path, format!("not UTF-8: {err}")))?;
    let mut lines: Vec<&str> = text.split('\n').collect();
    if lines.last().is_some_and(|line| line.is_empty()) {
        lines.pop();
    }
    let mut rows = Vec::with_capacity(lines.len());
    for (index, line) in lines.iter().enumerate() {
        if line.is_empty() {
            return Err(AskError::bundle(path, format!("blank line {}", index + 1)));
        }
        let row = serde_json::from_str(line).map_err(|err| {
            AskError::bundle(path, format!("malformed JSON on line {}: {err}", index + 1))
        })?;
        rows.push(row);
    }
    Ok(rows)
}

pub fn sha256(bytes: &[u8]) -> [u8; 32] {
    let digest = Sha256::digest(bytes);
    let mut out = [0u8; 32];
    out.copy_from_slice(&digest);
    out
}

pub fn sha256_file(path: &Path) -> Result<[u8; 32], AskError> {
    let mut file = fs::File::open(path)
        .map_err(|err| AskError::model(format!("{}: {err}", path.display())))?;
    let mut hasher = Sha256::new();
    std::io::copy(&mut file, &mut hasher)
        .map_err(|err| AskError::model(format!("{}: {err}", path.display())))?;
    let digest = hasher.finalize();
    let mut out = [0u8; 32];
    out.copy_from_slice(&digest);
    Ok(out)
}

pub fn fingerprint_bytes(fingerprint: &str) -> [u8; 64] {
    let mut out = [0u8; 64];
    let bytes = fingerprint.as_bytes();
    let len = bytes.len().min(out.len());
    out[..len].copy_from_slice(&bytes[..len]);
    out
}

pub fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut text = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        text.push(HEX[(byte >> 4) as usize] as char);
        text.push(HEX[(byte & 0xf) as usize] as char);
    }
    text
}
