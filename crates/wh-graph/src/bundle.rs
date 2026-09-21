use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use petgraph::stable_graph::StableGraph;

use crate::build::{build_graph, canonical_attrs, GraphBundle};
use crate::corpus::load_corpus;
use crate::error::GraphError;
use crate::model::{BundleManifest, EdgeRecord, GraphEdge, GraphNode, Passage};

pub struct Summary {
    pub bundle: String,
    pub format_version: u32,
    pub nodes: usize,
    pub edges: usize,
    pub passages: usize,
    pub corpus_fingerprint: String,
}

pub fn build_bundle(
    corpus_dir: &Path,
    out: &Path,
    out_label: &str,
    dry_run: bool,
) -> Result<Summary, GraphError> {
    let corpus = load_corpus(corpus_dir)?;
    let graph = build_graph(&corpus);
    let postcard = encode_postcard(&graph.nodes, &graph.edges)?;
    check_graph(
        &graph.nodes,
        &graph.edges,
        &graph.passages,
        &graph.manifest,
        &postcard,
    )?;
    if !dry_run {
        write_bundle(out, &graph, &postcard)?;
    }
    Ok(summary_from(&graph, out_label))
}

pub fn validate_bundle(bundle: &Path, label: &str) -> Result<Summary, GraphError> {
    let loaded = read_bundle(bundle)?;
    check_versions(&loaded.manifest, bundle)?;
    check_graph(
        &loaded.nodes,
        &loaded.edges,
        &loaded.passages,
        &loaded.manifest,
        &loaded.postcard,
    )?;
    Ok(Summary {
        bundle: label.to_string(),
        format_version: loaded.manifest.format_version,
        nodes: loaded.nodes.len(),
        edges: loaded.edges.len(),
        passages: loaded.passages.len(),
        corpus_fingerprint: loaded.manifest.corpus_fingerprint,
    })
}

struct LoadedBundle {
    manifest: BundleManifest,
    nodes: Vec<GraphNode>,
    edges: Vec<EdgeRecord>,
    passages: Vec<Passage>,
    postcard: Vec<u8>,
}

fn summary_from(graph: &GraphBundle, label: &str) -> Summary {
    Summary {
        bundle: label.to_string(),
        format_version: graph.manifest.format_version,
        nodes: graph.nodes.len(),
        edges: graph.edges.len(),
        passages: graph.passages.len(),
        corpus_fingerprint: graph.manifest.corpus_fingerprint.clone(),
    }
}

fn write_bundle(out: &Path, graph: &GraphBundle, postcard: &[u8]) -> Result<(), GraphError> {
    if out.exists() && !out.is_dir() {
        return Err(GraphError::read(out, "bundle path is not a directory"));
    }
    let parent = parent_dir(out);
    fs::create_dir_all(parent).map_err(|err| GraphError::read(parent, err))?;
    let temp = hidden_sibling(out, "tmp");
    if temp.exists() {
        fs::remove_dir_all(&temp).map_err(|err| GraphError::read(&temp, err))?;
    }
    fs::create_dir(&temp).map_err(|err| GraphError::read(&temp, err))?;
    let published = (|| {
        write_files(&temp, graph, postcard)?;
        validate_bundle(&temp, &temp.display().to_string())?;
        publish_dir(&temp, out)
    })();
    if temp.exists() {
        let _ = fs::remove_dir_all(&temp);
    }
    published
}

fn write_files(dir: &Path, graph: &GraphBundle, postcard: &[u8]) -> Result<(), GraphError> {
    write_manifest(&dir.join("manifest.json"), &graph.manifest)?;
    write_jsonl(&dir.join("nodes.jsonl"), &graph.nodes)?;
    write_jsonl(&dir.join("edges.jsonl"), &graph.edges)?;
    write_jsonl(&dir.join("passages.jsonl"), &graph.passages)?;
    fs::write(dir.join("graph.postcard"), postcard)
        .map_err(|err| GraphError::read(&dir.join("graph.postcard"), err))?;
    Ok(())
}

fn write_manifest(path: &Path, manifest: &BundleManifest) -> Result<(), GraphError> {
    let mut bytes =
        serde_json::to_vec_pretty(manifest).map_err(|err| GraphError::read(path, err))?;
    bytes.push(b'\n');
    fs::write(path, bytes).map_err(|err| GraphError::read(path, err))
}

fn write_jsonl<T: serde::Serialize>(path: &Path, rows: &[T]) -> Result<(), GraphError> {
    let mut buffer = Vec::new();
    for row in rows {
        serde_json::to_writer(&mut buffer, row).map_err(|err| GraphError::read(path, err))?;
        buffer.push(b'\n');
    }
    fs::write(path, buffer).map_err(|err| GraphError::read(path, err))
}

fn publish_dir(temp: &Path, out: &Path) -> Result<(), GraphError> {
    if out.exists() {
        let backup = hidden_sibling(out, "bak");
        if backup.exists() {
            fs::remove_dir_all(&backup).map_err(|err| GraphError::read(&backup, err))?;
        }
        fs::rename(out, &backup).map_err(|err| GraphError::read(out, err))?;
        if let Err(err) = fs::rename(temp, out) {
            let _ = fs::rename(&backup, out);
            return Err(GraphError::read(out, err));
        }
        fs::remove_dir_all(&backup).map_err(|err| GraphError::read(&backup, err))?;
        return Ok(());
    }
    fs::rename(temp, out).map_err(|err| GraphError::read(out, err))
}

fn parent_dir(path: &Path) -> &Path {
    match path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent,
        _ => Path::new("."),
    }
}

fn hidden_sibling(path: &Path, label: &str) -> PathBuf {
    let name = path.file_name().unwrap_or_default().to_string_lossy();
    parent_dir(path).join(format!(".{name}.{label}-{}", std::process::id()))
}

fn read_bundle(bundle: &Path) -> Result<LoadedBundle, GraphError> {
    let manifest_path = bundle.join("manifest.json");
    let manifest_bytes =
        fs::read(&manifest_path).map_err(|err| GraphError::read(&manifest_path, err))?;
    let manifest: BundleManifest = serde_json::from_slice(&manifest_bytes)
        .map_err(|err| GraphError::read(&manifest_path, format!("malformed JSON: {err}")))?;
    let postcard_path = bundle.join("graph.postcard");
    let postcard = fs::read(&postcard_path).map_err(|err| GraphError::read(&postcard_path, err))?;
    Ok(LoadedBundle {
        manifest,
        nodes: read_jsonl(&bundle.join("nodes.jsonl"))?,
        edges: read_jsonl(&bundle.join("edges.jsonl"))?,
        passages: read_jsonl(&bundle.join("passages.jsonl"))?,
        postcard,
    })
}

fn read_jsonl<T: serde::de::DeserializeOwned>(path: &Path) -> Result<Vec<T>, GraphError> {
    let bytes = fs::read(path).map_err(|err| GraphError::read(path, err))?;
    let text = std::str::from_utf8(&bytes)
        .map_err(|err| GraphError::read(path, format!("not UTF-8: {err}")))?;
    let mut lines: Vec<&str> = text.split('\n').collect();
    if lines.last().is_some_and(|line| line.is_empty()) {
        lines.pop();
    }
    let mut rows = Vec::with_capacity(lines.len());
    for (index, line) in lines.iter().enumerate() {
        if line.is_empty() {
            return Err(GraphError::read(path, format!("blank line {}", index + 1)));
        }
        let row = serde_json::from_str(line).map_err(|err| {
            GraphError::read(path, format!("malformed JSON on line {}: {err}", index + 1))
        })?;
        rows.push(row);
    }
    Ok(rows)
}

fn check_versions(manifest: &BundleManifest, bundle: &Path) -> Result<(), GraphError> {
    if manifest.format_version != 1 {
        return Err(GraphError::read(
            &bundle.join("manifest.json"),
            format!(
                "format_version is not 1 (found {})",
                manifest.format_version
            ),
        ));
    }
    if manifest.corpus_schema_version != 1 {
        return Err(GraphError::read(
            &bundle.join("manifest.json"),
            format!(
                "corpus_schema_version is not 1 (found {})",
                manifest.corpus_schema_version
            ),
        ));
    }
    Ok(())
}

fn check_graph(
    nodes: &[GraphNode],
    edges: &[EdgeRecord],
    passages: &[Passage],
    manifest: &BundleManifest,
    postcard: &[u8],
) -> Result<(), GraphError> {
    let node_ids = nodes
        .iter()
        .map(|node| node.id.as_str())
        .collect::<HashSet<_>>();
    for edge in edges {
        if !node_ids.contains(edge.from.as_str()) {
            return Err(GraphError::invalid(
                &edge.from,
                format!("edge endpoint is not a node id ({})", edge.id),
            ));
        }
        if !node_ids.contains(edge.to.as_str()) {
            return Err(GraphError::invalid(
                &edge.to,
                format!("edge endpoint is not a node id ({})", edge.id),
            ));
        }
    }

    for node in nodes.iter().filter(|node| node.kind == "Datasheet") {
        let incoming = edges
            .iter()
            .filter(|edge| edge.kind == "FACTION_HAS_DATASHEET" && edge.to == node.id)
            .count();
        if incoming != 1 {
            return Err(GraphError::invalid(
                &node.id,
                "Datasheet must be the target of exactly one FACTION_HAS_DATASHEET edge",
            ));
        }
    }

    let mut normalized = HashSet::new();
    for node in nodes.iter().filter(|node| node.kind == "Keyword") {
        let key = node
            .attrs
            .text("normalized")
            .ok_or_else(|| GraphError::invalid(&node.id, "Keyword is missing attrs.normalized"))?;
        if !normalized.insert(key.to_string()) {
            return Err(GraphError::invalid(
                &node.id,
                "Keyword normalized label is shared with another Keyword node",
            ));
        }
    }

    if passages.len() != nodes.len() {
        return Err(GraphError::invalid(
            first_node_id(nodes),
            "passage_count does not equal the node count",
        ));
    }
    for (node, passage) in nodes.iter().zip(passages.iter()) {
        if passage.node_id != node.id
            || passage.title != node.label
            || passage.text != node.text
            || passage.wahapedia_link != node.source_url
        {
            return Err(GraphError::invalid(
                &node.id,
                "passage does not match the node",
            ));
        }
    }

    let decoded: StableGraph<GraphNode, GraphEdge> =
        postcard::from_bytes(postcard).map_err(|err| {
            GraphError::Read(format!(
                "graph.postcard: cannot decode postcard snapshot: {err}"
            ))
        })?;
    compare_postcard(nodes, edges, &decoded)?;

    if manifest.node_count != nodes.len()
        || manifest.edge_count != edges.len()
        || manifest.passage_count != passages.len()
        || manifest.node_counts_by_kind != counts(nodes.iter().map(|node| node.kind.as_str()))
        || manifest.edge_counts_by_kind != counts(edges.iter().map(|edge| edge.kind.as_str()))
    {
        return Err(GraphError::invalid(
            first_node_id(nodes),
            "manifest counts do not match the bundle files",
        ));
    }
    Ok(())
}

fn encode_postcard(nodes: &[GraphNode], edges: &[EdgeRecord]) -> Result<Vec<u8>, GraphError> {
    let mut graph = StableGraph::<GraphNode, GraphEdge>::default();
    let mut index = HashMap::new();
    for node in nodes {
        let node_index = graph.add_node(node.clone());
        index.insert(node.id.clone(), node_index);
    }
    for edge in edges {
        let from = index
            .get(&edge.from)
            .copied()
            .ok_or_else(|| GraphError::invalid(&edge.from, "edge endpoint is not a node id"))?;
        let to = index
            .get(&edge.to)
            .copied()
            .ok_or_else(|| GraphError::invalid(&edge.to, "edge endpoint is not a node id"))?;
        graph.add_edge(from, to, edge.to_graph_edge());
    }
    postcard::to_allocvec(&graph)
        .map_err(|err| GraphError::Read(format!("graph.postcard: cannot encode snapshot: {err}")))
}

fn compare_postcard(
    nodes: &[GraphNode],
    edges: &[EdgeRecord],
    graph: &StableGraph<GraphNode, GraphEdge>,
) -> Result<(), GraphError> {
    let json_nodes = nodes.iter().map(node_payload).collect::<HashSet<_>>();
    let post_nodes = graph
        .node_weights()
        .map(node_payload)
        .collect::<HashSet<_>>();
    if json_nodes.len() != nodes.len() || json_nodes != post_nodes {
        let missing = nodes
            .iter()
            .find(|node| !post_nodes.contains(&node_payload(node)));
        let node_id = missing
            .map(|node| node.id.as_str())
            .unwrap_or_else(|| first_node_id(nodes));
        return Err(GraphError::invalid(
            node_id,
            "postcard node payload does not match nodes.jsonl",
        ));
    }

    let json_edges = edges.iter().map(edge_payload).collect::<HashSet<_>>();
    let post_edges = graph
        .edge_weights()
        .map(graph_edge_payload)
        .collect::<HashSet<_>>();
    if json_edges.len() != edges.len() || json_edges != post_edges {
        let missing = edges
            .iter()
            .find(|edge| !post_edges.contains(&edge_payload(edge)));
        let node_id = missing
            .map(|edge| edge.from.as_str())
            .unwrap_or_else(|| first_node_id(nodes));
        return Err(GraphError::invalid(
            node_id,
            "postcard edge payload does not match edges.jsonl",
        ));
    }
    Ok(())
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

fn counts<'a>(kinds: impl Iterator<Item = &'a str>) -> std::collections::BTreeMap<String, usize> {
    let mut counts = std::collections::BTreeMap::new();
    for kind in kinds {
        *counts.entry(kind.to_string()).or_insert(0) += 1;
    }
    counts
}

fn first_node_id(nodes: &[GraphNode]) -> &str {
    nodes
        .first()
        .map(|node| node.id.as_str())
        .unwrap_or("bundle")
}
