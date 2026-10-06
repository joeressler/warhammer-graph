use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::Path;

use cozo::{DataValue, DbInstance, NamedRows, ScriptMutability};

use crate::build::canonical_attrs;
use crate::error::GraphError;
use crate::model::{BundleManifest, EdgeRecord, GraphNode, Passage};

pub const GRAPH_DB: &str = "graph.db";
pub const FORMAT_VERSION: u32 = 2;

pub struct GraphStore {
    db: DbInstance,
    nodes: HashMap<String, GraphNode>,
}

impl GraphStore {
    pub fn open(path: &Path) -> Result<Self, GraphError> {
        if !path.is_file() {
            return Err(GraphError::read(path, "missing Cozo graph store"));
        }
        let header = std::fs::read(path).map_err(|err| GraphError::read(path, err))?;
        if !header.starts_with(b"SQLite format 3") {
            return Err(GraphError::read(path, "not a Cozo SQLite graph store"));
        }
        let db = open_db(path)?;
        let nodes = load_nodes(&db, path)?;
        Ok(Self { db, nodes })
    }

    pub fn node(&self, id: &str) -> Option<&GraphNode> {
        self.nodes.get(id)
    }

    pub fn passages(&self) -> Result<Vec<Passage>, GraphError> {
        let rows = query(
            &self.db,
            "?[seq, node_id, title, text, link] := *passage{seq, node_id, title, text, link}\n:order seq",
            BTreeMap::new(),
        )?;
        rows.rows
            .iter()
            .map(|row| {
                Ok(Passage {
                    node_id: required_str(row, 1)?,
                    title: required_str(row, 2)?,
                    text: required_str(row, 3)?,
                    wahapedia_link: optional_str(row, 4)?,
                })
            })
            .collect()
    }

    pub fn adjacent(&self, id: &str, kinds: &[&str]) -> Result<Vec<String>, GraphError> {
        if kinds.is_empty() {
            return Ok(Vec::new());
        }
        let script = format!(
            "{}\nother[seq, id] := *graph_edge{{seq, kind, from_id: $id, to_id: id}}, kind_ok[kind]\nother[seq, id] := *graph_edge{{seq, kind, from_id: id, to_id: $id}}, kind_ok[kind]\n?[seq, id] := other[seq, id]\n:order seq",
            kind_ok_clause(kinds)
        );
        let mut params = BTreeMap::new();
        params.insert("id".to_string(), DataValue::from(id));
        let rows = query(&self.db, &script, params)?;
        rows.rows.iter().map(|row| required_str(row, 1)).collect()
    }

    pub fn neighbor_labels(&self, id: &str, limit: usize) -> Result<Vec<String>, GraphError> {
        let mut labels = self.neighbor_labels_for(std::slice::from_ref(&id.to_string()), limit)?;
        Ok(labels.remove(id).unwrap_or_default())
    }

    pub fn neighbor_labels_for(
        &self,
        ids: &[String],
        limit: usize,
    ) -> Result<HashMap<String, Vec<String>>, GraphError> {
        let mut labels = HashMap::new();
        for id in ids {
            labels.entry(id.clone()).or_insert_with(Vec::new);
        }
        if ids.is_empty() || limit == 0 {
            return Ok(labels);
        }
        let wanted = ids.iter().map(String::as_str).collect::<HashSet<_>>();
        let rows = query(
            &self.db,
            "?[seq, from_id, to_id] := *graph_edge{seq, from_id, to_id}\n:order seq",
            BTreeMap::new(),
        )?;
        for row in &rows.rows {
            let from = required_str(row, 1)?;
            let to = required_str(row, 2)?;
            push_neighbor_label(&mut labels, &wanted, &from, &to, limit, &self.nodes);
            push_neighbor_label(&mut labels, &wanted, &to, &from, limit, &self.nodes);
            if labels.values().all(|found| found.len() >= limit) {
                break;
            }
        }
        Ok(labels)
    }

    pub fn context_edges(&self, ids: &HashSet<&str>) -> Result<Vec<EdgeRecord>, GraphError> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let script = format!(
            "{}\n?[seq, id, kind, from_id, to_id, attrs_json] := keep[from_id], keep[to_id], *graph_edge{{seq, id, kind, from_id, to_id, attrs_json}}\n:order seq",
            keep_clause(ids.iter().copied())
        );
        let rows = query(&self.db, &script, BTreeMap::new())?;
        rows.rows.iter().map(|row| edge_from_row(row)).collect()
    }

    pub fn expand_edges(
        &self,
        seeds: &[String],
        kinds: &[&str],
        depth: u32,
    ) -> Result<Vec<EdgeRecord>, GraphError> {
        if seeds.is_empty() || kinds.is_empty() || depth == 0 {
            return Ok(Vec::new());
        }
        let mut hops = String::from("reach[id] := seed[id]\nhop0[id] := seed[id]\n");
        for step in 1..=depth {
            let prev = step - 1;
            hops.push_str(&format!(
                "hop{step}[id] := hop{prev}[cur], nbr[cur, id]\nreach[id] := hop{step}[id]\n"
            ));
        }
        let script = format!(
            "{}\n{}\nnbr[a, b] := *graph_edge{{kind, from_id: a, to_id: b}}, kind_ok[kind]\nnbr[a, b] := *graph_edge{{kind, from_id: b, to_id: a}}, kind_ok[kind]\n{hops}?[seq, id, kind, from_id, to_id, attrs_json] := reach[from_id], reach[to_id], *graph_edge{{seq, id, kind, from_id, to_id, attrs_json}}, kind_ok[kind]\n:order seq",
            seed_clause(seeds),
            kind_ok_clause(kinds)
        );
        let rows = query(&self.db, &script, BTreeMap::new())?;
        rows.rows.iter().map(|row| edge_from_row(row)).collect()
    }

    /// `(datasheet id, keyword id, is_faction_keyword)` in edge-file order.
    pub fn datasheet_keywords(&self) -> Result<Vec<(String, String, bool)>, GraphError> {
        let rows = query(
            &self.db,
            "?[seq, from_id, to_id, attrs_json] := *graph_edge{seq, kind: 'DATASHEET_HAS_KEYWORD', from_id, to_id, attrs_json}\n:order seq",
            BTreeMap::new(),
        )?;
        rows.rows
            .iter()
            .map(|row| {
                let attrs = parse_attrs(&required_str(row, 3)?)?;
                Ok((
                    required_str(row, 1)?,
                    required_str(row, 2)?,
                    attrs.text("is_faction_keyword") == Some("true"),
                ))
            })
            .collect()
    }

    pub fn faction_datasheets(&self, faction_id: &str) -> Result<Vec<String>, GraphError> {
        let mut params = BTreeMap::new();
        params.insert("id".to_string(), DataValue::from(faction_id));
        let rows = query(
            &self.db,
            "?[seq, to_id] := *graph_edge{seq, kind: 'FACTION_HAS_DATASHEET', from_id: $id, to_id}, *node{id: to_id, kind: 'Datasheet'}\n:order seq",
            params,
        )?;
        rows.rows.iter().map(|row| required_str(row, 1)).collect()
    }

    pub fn meta(&self, key: &str) -> Result<Option<String>, GraphError> {
        let mut params = BTreeMap::new();
        params.insert("key".to_string(), DataValue::from(key));
        let rows = query(&self.db, "?[value] := *meta{key: $key, value}", params)?;
        match rows.rows.first() {
            Some(row) => Ok(Some(required_str(row, 0)?)),
            None => Ok(None),
        }
    }

    pub fn dump_nodes(&self) -> Result<Vec<GraphNode>, GraphError> {
        dump_nodes(&self.db)
    }

    pub fn dump_edges(&self) -> Result<Vec<EdgeRecord>, GraphError> {
        dump_edges(&self.db)
    }

    pub fn dump_passages(&self) -> Result<Vec<Passage>, GraphError> {
        self.passages()
    }
}

pub fn write_graph_db(
    path: &Path,
    nodes: &[GraphNode],
    edges: &[EdgeRecord],
    passages: &[Passage],
    manifest: &BundleManifest,
    nodes_sha256: &str,
    passages_sha256: &str,
) -> Result<(), GraphError> {
    if path.exists() {
        std::fs::remove_file(path).map_err(|err| GraphError::read(path, err))?;
    }
    {
        let db = open_db(path)?;
        db.run_default(":create node {id => kind, label, text, source_url, attrs_json}")
            .map_err(|err| GraphError::read(path, err))?;
        db.run_default(":create graph_edge {seq => id, kind, from_id, to_id, attrs_json}")
            .map_err(|err| GraphError::read(path, err))?;
        db.run_default(":create passage {seq => node_id, title, text, link}")
            .map_err(|err| GraphError::read(path, err))?;
        db.run_default(":create meta {key => value}")
            .map_err(|err| GraphError::read(path, err))?;
        let mut tables = BTreeMap::new();
        tables.insert("node".to_string(), node_rows(nodes));
        tables.insert("graph_edge".to_string(), edge_rows(edges));
        tables.insert("passage".to_string(), passage_rows(passages));
        tables.insert(
            "meta".to_string(),
            NamedRows::new(
                vec!["key".to_string(), "value".to_string()],
                vec![
                    meta_row("format_version", &manifest.format_version.to_string()),
                    meta_row("corpus_fingerprint", &manifest.corpus_fingerprint),
                    meta_row("nodes_sha256", nodes_sha256),
                    meta_row("passages_sha256", passages_sha256),
                ],
            ),
        );
        db.import_relations(tables)
            .map_err(|err| GraphError::read(path, err))?;
    }
    Ok(())
}

pub fn compare_store(
    store: &GraphStore,
    nodes: &[GraphNode],
    edges: &[EdgeRecord],
    passages: &[Passage],
) -> Result<(), GraphError> {
    let stored_nodes = store.dump_nodes()?;
    if stored_nodes.len() != nodes.len() {
        return Err(GraphError::invalid(
            first_node_id(nodes),
            "Cozo node payload does not match nodes.jsonl",
        ));
    }
    let stored = stored_nodes
        .iter()
        .map(node_payload)
        .collect::<HashSet<_>>();
    for node in nodes {
        if !stored.contains(&node_payload(node)) {
            return Err(GraphError::invalid(
                &node.id,
                "Cozo node payload does not match nodes.jsonl",
            ));
        }
    }

    let stored_edges = store.dump_edges()?;
    if stored_edges.len() != edges.len() {
        return Err(GraphError::invalid(
            first_node_id(nodes),
            "Cozo edge payload does not match edges.jsonl",
        ));
    }
    for (index, (json_edge, stored_edge)) in edges.iter().zip(stored_edges.iter()).enumerate() {
        if edge_payload(json_edge) != edge_payload(stored_edge) {
            return Err(GraphError::invalid(
                &json_edge.from,
                format!("Cozo edge seq {index} does not match edges.jsonl"),
            ));
        }
    }

    let stored_passages = store.dump_passages()?;
    if stored_passages != *passages {
        return Err(GraphError::invalid(
            first_node_id(nodes),
            "Cozo passage rows do not match passages.jsonl",
        ));
    }
    Ok(())
}

fn open_db(path: &Path) -> Result<DbInstance, GraphError> {
    DbInstance::new("sqlite", path, "").map_err(|err| GraphError::read(path, err))
}

fn query(
    db: &DbInstance,
    script: &str,
    params: BTreeMap<String, DataValue>,
) -> Result<NamedRows, GraphError> {
    db.run_script(script, params, ScriptMutability::Immutable)
        .map_err(|err| GraphError::Read(format!("graph.db: {err}")))
}

fn load_nodes(db: &DbInstance, path: &Path) -> Result<HashMap<String, GraphNode>, GraphError> {
    let mut nodes = HashMap::new();
    for node in dump_nodes_from(db, path)? {
        nodes.insert(node.id.clone(), node);
    }
    Ok(nodes)
}

fn dump_nodes(db: &DbInstance) -> Result<Vec<GraphNode>, GraphError> {
    dump_nodes_from(db, Path::new("graph.db"))
}

fn dump_nodes_from(db: &DbInstance, path: &Path) -> Result<Vec<GraphNode>, GraphError> {
    let rows = db
        .run_script(
            "?[id, kind, label, text, source_url, attrs_json] := *node{id, kind, label, text, source_url, attrs_json}",
            BTreeMap::new(),
            ScriptMutability::Immutable,
        )
        .map_err(|err| GraphError::read(path, err))?;
    rows.rows.iter().map(|row| node_from_row(row)).collect()
}

fn dump_edges(db: &DbInstance) -> Result<Vec<EdgeRecord>, GraphError> {
    dump_edges_from(db, Path::new(GRAPH_DB))
}

fn dump_edges_from(db: &DbInstance, path: &Path) -> Result<Vec<EdgeRecord>, GraphError> {
    let rows = db
        .run_script(
            "?[seq, id, kind, from_id, to_id, attrs_json] := *graph_edge{seq, id, kind, from_id, to_id, attrs_json}\n:order seq",
            BTreeMap::new(),
            ScriptMutability::Immutable,
        )
        .map_err(|err| GraphError::read(path, err))?;
    rows.rows.iter().map(|row| edge_from_row(row)).collect()
}

fn node_rows(nodes: &[GraphNode]) -> NamedRows {
    NamedRows::new(
        vec![
            "id".to_string(),
            "kind".to_string(),
            "label".to_string(),
            "text".to_string(),
            "source_url".to_string(),
            "attrs_json".to_string(),
        ],
        nodes
            .iter()
            .map(|node| {
                vec![
                    DataValue::from(node.id.as_str()),
                    DataValue::from(node.kind.as_str()),
                    DataValue::from(node.label.as_str()),
                    DataValue::from(node.text.as_str()),
                    optional_url(&node.source_url),
                    DataValue::from(canonical_attrs(&node.attrs)),
                ]
            })
            .collect(),
    )
}

fn edge_rows(edges: &[EdgeRecord]) -> NamedRows {
    NamedRows::new(
        vec![
            "seq".to_string(),
            "id".to_string(),
            "kind".to_string(),
            "from_id".to_string(),
            "to_id".to_string(),
            "attrs_json".to_string(),
        ],
        edges
            .iter()
            .enumerate()
            .map(|(seq, edge)| {
                vec![
                    DataValue::from(seq as i64),
                    DataValue::from(edge.id.as_str()),
                    DataValue::from(edge.kind.as_str()),
                    DataValue::from(edge.from.as_str()),
                    DataValue::from(edge.to.as_str()),
                    DataValue::from(canonical_attrs(&edge.attrs)),
                ]
            })
            .collect(),
    )
}

fn passage_rows(passages: &[Passage]) -> NamedRows {
    NamedRows::new(
        vec![
            "seq".to_string(),
            "node_id".to_string(),
            "title".to_string(),
            "text".to_string(),
            "link".to_string(),
        ],
        passages
            .iter()
            .enumerate()
            .map(|(seq, passage)| {
                vec![
                    DataValue::from(seq as i64),
                    DataValue::from(passage.node_id.as_str()),
                    DataValue::from(passage.title.as_str()),
                    DataValue::from(passage.text.as_str()),
                    optional_url(&passage.wahapedia_link),
                ]
            })
            .collect(),
    )
}

fn meta_row(key: &str, value: &str) -> Vec<DataValue> {
    vec![DataValue::from(key), DataValue::from(value)]
}

fn optional_url(value: &Option<String>) -> DataValue {
    match value {
        Some(text) => DataValue::from(text.as_str()),
        None => DataValue::Null,
    }
}

fn node_from_row(row: &[DataValue]) -> Result<GraphNode, GraphError> {
    Ok(GraphNode {
        id: required_str(row, 0)?,
        kind: required_str(row, 1)?,
        label: required_str(row, 2)?,
        text: required_str(row, 3)?,
        source_url: optional_str(row, 4)?,
        attrs: parse_attrs(&required_str(row, 5)?)?,
    })
}

fn edge_from_row(row: &[DataValue]) -> Result<EdgeRecord, GraphError> {
    Ok(EdgeRecord {
        id: required_str(row, 1)?,
        kind: required_str(row, 2)?,
        from: required_str(row, 3)?,
        to: required_str(row, 4)?,
        attrs: parse_attrs(&required_str(row, 5)?)?,
    })
}

fn parse_attrs(json: &str) -> Result<crate::attrs::Attrs, GraphError> {
    serde_json::from_str(json)
        .map_err(|err| GraphError::Read(format!("graph.db: malformed attrs JSON: {err}")))
}

fn required_str(row: &[DataValue], index: usize) -> Result<String, GraphError> {
    row.get(index)
        .and_then(DataValue::get_str)
        .map(str::to_string)
        .ok_or_else(|| GraphError::Read("graph.db: expected a string column".to_string()))
}

fn optional_str(row: &[DataValue], index: usize) -> Result<Option<String>, GraphError> {
    match row.get(index) {
        Some(DataValue::Null) | None => Ok(None),
        Some(value) => value
            .get_str()
            .map(|text| Some(text.to_string()))
            .ok_or_else(|| GraphError::Read("graph.db: expected a string or null".to_string())),
    }
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

fn cozo_str(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

fn seed_clause(ids: &[String]) -> String {
    relation_clause("seed", ids.iter().map(String::as_str))
}

fn keep_clause<'a>(ids: impl Iterator<Item = &'a str>) -> String {
    relation_clause("keep", ids)
}

fn kind_ok_clause(kinds: &[&str]) -> String {
    let rows = kinds
        .iter()
        .map(|kind| format!("[{}]", cozo_str(kind)))
        .collect::<Vec<_>>()
        .join(", ");
    format!("kind_ok[kind] <- [{rows}]")
}

fn relation_clause<'a>(name: &str, ids: impl Iterator<Item = &'a str>) -> String {
    let rows = ids
        .map(|id| format!("[{}]", cozo_str(id)))
        .collect::<Vec<_>>()
        .join(", ");
    format!("{name}[id] <- [{rows}]")
}

fn push_neighbor_label(
    labels: &mut HashMap<String, Vec<String>>,
    wanted: &HashSet<&str>,
    id: &str,
    other: &str,
    limit: usize,
    nodes: &HashMap<String, GraphNode>,
) {
    if !wanted.contains(id) {
        return;
    }
    let Some(found) = labels.get_mut(id) else {
        return;
    };
    if found.len() >= limit {
        return;
    }
    if let Some(node) = nodes.get(other) {
        found.push(node.label.clone());
    }
}

fn first_node_id(nodes: &[GraphNode]) -> &str {
    nodes
        .first()
        .map(|node| node.id.as_str())
        .unwrap_or("bundle")
}
