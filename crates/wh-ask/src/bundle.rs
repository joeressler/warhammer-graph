use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::fs;
use std::path::Path;

use wh_graph::{
    BundleManifest, EdgeRecord, GraphError, GraphNode, GraphStore, Passage, FORMAT_VERSION,
    GRAPH_DB,
};

use crate::error::{AskError, Candidate};
use crate::names::{fold, name_key};
use crate::types::NodeRef;

/// Case-folded passage text, computed once so search does not fold 26k passages per call.
pub(crate) struct Folded {
    pub title: String,
    pub key: String,
    pub text: String,
}

/// A format-version-2 bundle opened from `graph.db`. It is read-only.
///
/// Nodes, passages, and edges are read into memory once at open. Every lookup
/// after that is a hash probe, so a call costs the node's degree and not a
/// query over all 165k edges.
pub struct Bundle {
    manifest: BundleManifest,
    store: GraphStore,
    pub(crate) passages: Vec<Passage>,
    pub(crate) folded: Vec<Folded>,
    by_kind: BTreeMap<String, Vec<usize>>,
    labels: HashMap<(String, String), Vec<String>>,
    edges: Vec<EdgeRecord>,
    /// Node id to the indexes in `edges` of every edge that touches it.
    incident: HashMap<String, Vec<u32>>,
}

impl Bundle {
    /// Open a bundle directory holding `manifest.json` and `graph.db`.
    pub fn open(dir: &Path) -> Result<Self, AskError> {
        let manifest_path = dir.join("manifest.json");
        let manifest_bytes =
            fs::read(&manifest_path).map_err(|err| AskError::bundle(&manifest_path, err))?;
        let manifest: BundleManifest = serde_json::from_slice(&manifest_bytes).map_err(|err| {
            AskError::bundle(&manifest_path, format!("malformed JSON: {err}"))
        })?;
        if manifest.format_version != FORMAT_VERSION {
            return Err(AskError::bundle(
                &manifest_path,
                format!(
                    "format_version is not {FORMAT_VERSION} (found {})",
                    manifest.format_version
                ),
            ));
        }

        let db_path = dir.join(GRAPH_DB);
        let store = GraphStore::open(&db_path).map_err(store_err)?;
        let found_version = store
            .meta("format_version")
            .map_err(store_err)?
            .unwrap_or_default();
        if found_version != FORMAT_VERSION.to_string() {
            return Err(AskError::bundle(
                &db_path,
                format!("rebuild the bundle; format_version is not {FORMAT_VERSION}"),
            ));
        }
        let fingerprint = store
            .meta("corpus_fingerprint")
            .map_err(store_err)?
            .unwrap_or_default();
        if fingerprint != manifest.corpus_fingerprint {
            return Err(AskError::bundle(
                &db_path,
                "rebuild the bundle; corpus_fingerprint does not match manifest.json",
            ));
        }
        let passages = store.passages().map_err(store_err)?;

        let mut folded = Vec::with_capacity(passages.len());
        let mut by_kind: BTreeMap<String, Vec<usize>> = BTreeMap::new();
        let mut labels: HashMap<(String, String), Vec<String>> = HashMap::new();
        for (index, passage) in passages.iter().enumerate() {
            folded.push(Folded {
                title: fold(&passage.title),
                key: name_key(&passage.title),
                text: fold(&passage.text),
            });
            let Some(node) = store.node(&passage.node_id) else {
                continue;
            };
            by_kind.entry(node.kind.clone()).or_default().push(index);
            labels
                .entry((node.kind.clone(), name_key(&node.label)))
                .or_default()
                .push(node.id.clone());
        }

        let edges = store.dump_edges().map_err(store_err)?;
        let mut incident: HashMap<String, Vec<u32>> = HashMap::new();
        for (index, edge) in edges.iter().enumerate() {
            incident.entry(edge.from.clone()).or_default().push(index as u32);
            if edge.to != edge.from {
                incident.entry(edge.to.clone()).or_default().push(index as u32);
            }
        }

        Ok(Self {
            manifest,
            store,
            passages,
            folded,
            by_kind,
            labels,
            edges,
            incident,
        })
    }

    /// The bundle manifest: format version, edition, and node and edge counts by kind.
    /// The keys of `node_counts_by_kind` and `edge_counts_by_kind` are the kinds
    /// that calls such as [`Bundle::search`] and [`Bundle::neighbors`] accept.
    pub fn manifest(&self) -> &BundleManifest {
        &self.manifest
    }

    pub(crate) fn node(&self, id: &str) -> Option<&GraphNode> {
        self.store.node(id)
    }

    /// Nodes of one kind, in bundle order.
    pub(crate) fn nodes_of<'a>(&'a self, kind: &str) -> impl Iterator<Item = &'a GraphNode> + 'a {
        self.by_kind
            .get(kind)
            .into_iter()
            .flatten()
            .filter_map(|index| self.store.node(&self.passages[*index].node_id))
    }

    pub(crate) fn passage_indexes(&self, kind: &str) -> &[usize] {
        self.by_kind.get(kind).map(Vec::as_slice).unwrap_or(&[])
    }

    /// Ids whose label is `name` (articles and punctuation do not matter).
    pub(crate) fn ids_named(&self, kind: &str, name: &str) -> &[String] {
        self.labels
            .get(&(kind.to_string(), name_key(name)))
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    pub(crate) fn edge_kinds(&self) -> Vec<&str> {
        self.manifest
            .edge_counts_by_kind
            .keys()
            .map(String::as_str)
            .collect()
    }

    pub(crate) fn node_ref(&self, node: &GraphNode) -> NodeRef {
        NodeRef {
            id: node.id.clone(),
            kind: node.kind.clone(),
            name: node.label.clone(),
            wahapedia_link: node.source_url.clone(),
        }
    }

    /// Every edge within `depth` hops of the seeds over `kinds`, plus any edge of
    /// those kinds that joins two nodes already reached. In bundle order.
    pub(crate) fn expand_edges(
        &self,
        seeds: &[String],
        kinds: &[&str],
        depth: u32,
    ) -> Vec<&EdgeRecord> {
        let kinds = kinds.iter().copied().collect::<HashSet<_>>();
        let mut reached = seeds.iter().map(String::as_str).collect::<HashSet<_>>();
        let mut frontier = reached.iter().copied().collect::<Vec<_>>();
        for _ in 0..depth {
            let mut next = Vec::new();
            for id in frontier {
                for edge in self.touching(id) {
                    if !kinds.contains(edge.kind.as_str()) {
                        continue;
                    }
                    for end in [edge.from.as_str(), edge.to.as_str()] {
                        if reached.insert(end) {
                            next.push(end);
                        }
                    }
                }
            }
            if next.is_empty() {
                break;
            }
            frontier = next;
        }
        let mut found = BTreeSet::new();
        for id in &reached {
            if let Some(indexes) = self.incident.get(*id) {
                for index in indexes {
                    let edge = &self.edges[*index as usize];
                    if kinds.contains(edge.kind.as_str())
                        && reached.contains(edge.from.as_str())
                        && reached.contains(edge.to.as_str())
                    {
                        found.insert(*index);
                    }
                }
            }
        }
        found
            .into_iter()
            .map(|index| &self.edges[index as usize])
            .collect()
    }

    fn touching<'a>(&'a self, id: &str) -> impl Iterator<Item = &'a EdgeRecord> + 'a {
        self.incident
            .get(id)
            .into_iter()
            .flatten()
            .map(|index| &self.edges[*index as usize])
    }

    /// Edges of one kind that touch `id`, in bundle order.
    pub(crate) fn edges_at(&self, id: &str, kind: &str) -> Vec<&EdgeRecord> {
        self.touching(id).filter(|edge| edge.kind == kind).collect()
    }

    pub(crate) fn faction_datasheets(&self, faction_id: &str) -> Vec<String> {
        self.edges_at(faction_id, "FACTION_HAS_DATASHEET")
            .into_iter()
            .filter(|edge| edge.from == faction_id)
            .filter(|edge| self.node(&edge.to).is_some_and(|node| node.kind == "Datasheet"))
            .map(|edge| edge.to.clone())
            .collect()
    }

    /// `(datasheet id, keyword id, is_faction_keyword)` in bundle order.
    pub(crate) fn datasheet_keywords(&self) -> Vec<(String, String, bool)> {
        self.edges
            .iter()
            .filter(|edge| edge.kind == "DATASHEET_HAS_KEYWORD")
            .map(|edge| {
                (
                    edge.from.clone(),
                    edge.to.clone(),
                    edge.attrs.text("is_faction_keyword") == Some("true"),
                )
            })
            .collect()
    }

    /// Resolve an id or a name to exactly one node of `kind`.
    ///
    /// An id passes through. A name must match one label; several matches give
    /// [`AskError::Ambiguous`] so the caller can pick an id.
    pub(crate) fn resolve(&self, kind: &str, input: &str) -> Result<String, AskError> {
        let input = input.trim();
        if let Some(node) = self.node(input) {
            if node.kind == kind {
                return Ok(node.id.clone());
            }
            return Err(AskError::invalid(format!(
                "{input} is a {}, not a {kind}",
                node.kind
            )));
        }
        let ids = self.ids_named(kind, input);
        match ids {
            [] => Err(AskError::not_found(
                format!("{kind} named \"{input}\""),
                self.suggest(kind, input),
            )),
            [only] => Ok(only.clone()),
            many => Err(AskError::Ambiguous {
                what: format!("{kind} \"{input}\""),
                candidates: many
                    .iter()
                    .filter_map(|id| self.node(id))
                    .map(|node| Candidate {
                        id: node.id.clone(),
                        name: node.label.clone(),
                        detail: self.describe(node),
                    })
                    .collect(),
            }),
        }
    }

    /// Every id carrying the name, or the id itself. Several are fine here.
    pub(crate) fn resolve_all(&self, kind: &str, input: &str) -> Result<Vec<String>, AskError> {
        let input = input.trim();
        if let Some(node) = self.node(input) {
            if node.kind == kind {
                return Ok(vec![node.id.clone()]);
            }
            return Err(AskError::invalid(format!(
                "{input} is a {}, not a {kind}",
                node.kind
            )));
        }
        let ids = self.ids_named(kind, input);
        if ids.is_empty() {
            return Err(AskError::not_found(
                format!("{kind} named \"{input}\""),
                self.suggest(kind, input),
            ));
        }
        Ok(ids.to_vec())
    }

    fn suggest(&self, kind: &str, input: &str) -> Vec<String> {
        let mut names = Vec::new();
        for hit in self.search(input, &[kind], 5) {
            if !names.contains(&hit.node.name) {
                names.push(hit.node.name);
            }
        }
        names
    }

    /// What tells same-named nodes apart: a unit's faction, a detachment's
    /// faction, and the detachment (or faction) that owns a rule.
    fn describe(&self, node: &GraphNode) -> String {
        let owned_by = |kinds: &[&str]| {
            kinds.iter().find_map(|kind| {
                self.edges_at(&node.id, kind)
                    .into_iter()
                    .find(|edge| edge.to == node.id)
                    .and_then(|edge| self.node(&edge.from))
                    .map(|owner| owner.label.clone())
            })
        };
        match node.kind.as_str() {
            "Datasheet" => node
                .attrs
                .text("faction_node")
                .and_then(|id| self.node(id))
                .map(|faction| faction.label.clone()),
            "Detachment" => owned_by(&["FACTION_HAS_DETACHMENT"]).map(|faction| {
                match node.attrs.text("type").map(str::trim).filter(|kind| !kind.is_empty()) {
                    Some(kind) => format!("{faction} ({kind})"),
                    None => faction,
                }
            }),
            "Enhancement" => owned_by(&["DETACHMENT_HAS_ENHANCEMENT", "FACTION_HAS_ENHANCEMENT"]),
            "Stratagem" => owned_by(&["DETACHMENT_HAS_STRATAGEM", "FACTION_HAS_STRATAGEM"]),
            "DetachmentAbility" => {
                owned_by(&["DETACHMENT_HAS_ABILITY", "FACTION_HAS_DETACHMENT_ABILITY"])
            }
            _ => None,
        }
        .unwrap_or_default()
    }
}

fn store_err(err: GraphError) -> AskError {
    AskError::Bundle(err.to_string())
}
