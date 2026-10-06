//! Raw graph navigation: nodes, their neighbors, and subgraphs around seeds.

use std::collections::HashSet;

use crate::bundle::Bundle;
use crate::error::AskError;
use crate::types::{Direction, Neighbor, NodeDetail, Page, Subgraph};

const MAX_NEIGHBORS: usize = 500;
const MAX_EDGES: usize = 2000;
const MAX_DEPTH: u32 = 3;

impl Bundle {
    /// One node by id: its text and attributes as well as its name. An unknown
    /// id is [`AskError::NotFound`].
    pub fn node_detail(&self, id: &str) -> Result<NodeDetail, AskError> {
        let node = self
            .node(id.trim())
            .ok_or_else(|| AskError::not_found(format!("node with id \"{id}\""), Vec::new()))?;
        Ok(NodeDetail {
            id: node.id.clone(),
            kind: node.kind.clone(),
            name: node.label.clone(),
            text: node.text.clone(),
            attrs: node.attrs.clone(),
            wahapedia_link: node.source_url.clone(),
        })
    }

    /// Nodes one edge away, in either direction. `edge_kinds` limits the edge
    /// kinds followed (empty means all). `limit` is capped at 500; `total` is
    /// the count before the cap.
    ///
    /// Keywords and factions touch thousands of nodes, so pass `edge_kinds` for those.
    pub fn neighbors(
        &self,
        id: &str,
        edge_kinds: &[&str],
        limit: usize,
    ) -> Result<Page<Neighbor>, AskError> {
        let id = id.trim();
        if self.node(id).is_none() {
            return Err(AskError::not_found(
                format!("node with id \"{id}\""),
                Vec::new(),
            ));
        }
        let limit = limit.clamp(1, MAX_NEIGHBORS);
        let kinds = self.checked_kinds(edge_kinds)?;
        let mut items = Vec::new();
        let mut total = 0;
        let edges = self.expand_edges(&[id.to_string()], &kinds, 1);
        for edge in edges {
            let (direction, other) = if edge.from == id {
                (Direction::Outgoing, edge.to.as_str())
            } else if edge.to == id {
                (Direction::Incoming, edge.from.as_str())
            } else {
                continue;
            };
            let Some(node) = self.node(other) else {
                continue;
            };
            total += 1;
            if items.len() < limit {
                items.push(Neighbor {
                    edge_kind: edge.kind.clone(),
                    direction,
                    edge_attrs: edge.attrs.clone(),
                    node: self.node_ref(node),
                });
            }
        }
        Ok(Page { total, items })
    }

    /// Everything within `depth` edges (1 to 3) of the seeds, over `edge_kinds`
    /// (empty means all). Edges are capped at 2000 and `truncated` says so.
    pub fn subgraph(
        &self,
        seeds: &[&str],
        edge_kinds: &[&str],
        depth: u32,
    ) -> Result<Subgraph, AskError> {
        if seeds.is_empty() {
            return Err(AskError::invalid("subgraph needs at least one seed id"));
        }
        for seed in seeds {
            if self.node(seed).is_none() {
                return Err(AskError::not_found(
                    format!("node with id \"{seed}\""),
                    Vec::new(),
                ));
            }
        }
        let kinds = self.checked_kinds(edge_kinds)?;
        let seed_ids = seeds.iter().map(|seed| seed.to_string()).collect::<Vec<_>>();
        let mut edges = self.expand_edges(&seed_ids, &kinds, depth.clamp(1, MAX_DEPTH));
        let truncated = edges.len() > MAX_EDGES;
        edges.truncate(MAX_EDGES);
        let edges = edges.into_iter().cloned().collect::<Vec<_>>();
        let mut seen = HashSet::new();
        let mut nodes = Vec::new();
        let ends = seeds.iter().copied().chain(
            edges
                .iter()
                .flat_map(|edge| [edge.from.as_str(), edge.to.as_str()]),
        );
        for id in ends {
            if !seen.insert(id) {
                continue;
            }
            if let Some(node) = self.node(id) {
                nodes.push(self.node_ref(node));
            }
        }
        Ok(Subgraph {
            nodes,
            edges,
            truncated,
        })
    }

    /// The caller's edge kinds, or every kind in the bundle. An unknown kind is
    /// an error so a typo is not read as "no edges".
    fn checked_kinds<'a>(&'a self, requested: &[&'a str]) -> Result<Vec<&'a str>, AskError> {
        let known = self.edge_kinds();
        if requested.is_empty() {
            return Ok(known);
        }
        for kind in requested {
            if !known.contains(kind) {
                return Err(AskError::not_found(
                    format!("edge kind \"{kind}\""),
                    known.iter().map(|item| item.to_string()).collect(),
                ));
            }
        }
        Ok(requested.to_vec())
    }
}
