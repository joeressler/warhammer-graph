use std::collections::HashSet;

use serde::Serialize;

use crate::bundle::LoadedBundle;
use crate::expand::ContextNodes;

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Citation {
    pub node_id: String,
    pub title: String,
    pub wahapedia_link: Option<String>,
    pub neighbors: Vec<String>,
}

/// Keep cited prompt nodes, in prompt order. When the model cites nothing that
/// is in the prompt, list the seed nodes instead.
pub fn citations(bundle: &LoadedBundle, answer: &str, context: &ContextNodes) -> Vec<Citation> {
    let prompt_ids = context.nodes.iter().cloned().collect::<HashSet<_>>();
    let cited = bracket_ids(answer)
        .into_iter()
        .filter(|id| prompt_ids.contains(id))
        .collect::<HashSet<_>>();
    let ids = if cited.is_empty() {
        context.seeds.clone()
    } else {
        context
            .nodes
            .iter()
            .filter(|id| cited.contains(*id))
            .cloned()
            .collect()
    };
    ids.iter().map(|id| citation_for(bundle, id)).collect()
}

fn citation_for(bundle: &LoadedBundle, id: &str) -> Citation {
    let passage = bundle.passages.iter().find(|passage| passage.node_id == id);
    let node = bundle.node(id);
    let title = passage
        .map(|passage| passage.title.clone())
        .or_else(|| node.map(|node| node.label.clone()))
        .unwrap_or_default();
    let link = passage
        .and_then(|passage| passage.wahapedia_link.clone())
        .or_else(|| node.and_then(|node| node.source_url.clone()));
    Citation {
        node_id: id.to_string(),
        title,
        wahapedia_link: link,
        neighbors: neighbor_labels(bundle, id),
    }
}

fn neighbor_labels(bundle: &LoadedBundle, id: &str) -> Vec<String> {
    let mut labels = Vec::new();
    for edge in &bundle.edges {
        let other = if edge.from == id {
            Some(edge.to.as_str())
        } else if edge.to == id {
            Some(edge.from.as_str())
        } else {
            None
        };
        let Some(other) = other else {
            continue;
        };
        if let Some(node) = bundle.node(other) {
            labels.push(node.label.clone());
            if labels.len() == 8 {
                break;
            }
        }
    }
    labels
}

fn bracket_ids(answer: &str) -> Vec<String> {
    let mut ids = Vec::new();
    let mut rest = answer;
    while let Some(start) = rest.find('[') {
        rest = &rest[start + 1..];
        let Some(end) = rest.find(']') else {
            break;
        };
        let inner = &rest[..end];
        if is_node_id(inner) {
            ids.push(inner.to_string());
        }
        rest = &rest[end + 1..];
    }
    ids
}

/// `10ed:` plus a kind or table segment and the rest of the id. Edge ids are not citations.
fn is_node_id(value: &str) -> bool {
    let Some(rest) = value.strip_prefix("10ed:") else {
        return false;
    };
    if rest.starts_with("edge:") || rest == "edge" {
        return false;
    }
    let Some((kind, tail)) = rest.split_once(':') else {
        return false;
    };
    if kind.is_empty()
        || !kind
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
    {
        return false;
    }
    !tail.is_empty()
        && tail
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.' | '~' | '%' | ':'))
}
