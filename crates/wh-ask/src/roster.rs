//! Which datasheets a faction, chapter, or daemon god can field.
//!
//! A chapter keyword (Ultramarines) or a Chaos chapter faction (Death Guard)
//! also gets the generic parent datasheets those armies take alongside it, and
//! a Legiones Daemonica god gets the daemon units that carry no god keyword.

use std::collections::{HashMap, HashSet};

use crate::bundle::Bundle;
use crate::names::{fold, label_in, strip_articles};

const CSM_CHAPTERS: &[&str] = &[
    "death guard",
    "world eaters",
    "thousand sons",
    "emperor's children",
];

const DAEMON_GODS: &[&str] = &["khorne", "tzeentch", "nurgle", "slaanesh"];

const SM_PARENT: &str = "adeptus astartes";
const CSM_PARENT: &str = "heretic astartes";
const DAEMON_PARENT: &str = "legiones daemonica";

/// A name this module resolved: the faction or keyword node, and its datasheets.
pub(crate) struct RosterNodes {
    pub subject: String,
    pub datasheets: Vec<String>,
}

enum Subject {
    Faction(String),
    Keyword(String),
}

/// Find the faction or roster keyword `name` refers to. The whole label must
/// occur in `name`; an exact label wins, then the longest one.
pub(crate) fn roster(bundle: &Bundle, name: &str) -> Option<RosterNodes> {
    let haystack = strip_articles(&fold(name));
    let index = keyword_index(bundle);
    let sm_chapters = space_marine_chapters(&index);
    let mut best: Option<(usize, Subject)> = None;
    for node in bundle.nodes_of("Faction").chain(bundle.nodes_of("Keyword")) {
        let label = fold(&node.label);
        if !label_in(&haystack, &label) {
            continue;
        }
        let subject = match node.kind.as_str() {
            "Faction" => Subject::Faction(node.id.clone()),
            _ if roster_keyword(&label, &node.id, &sm_chapters) => {
                Subject::Keyword(node.id.clone())
            }
            _ => continue,
        };
        let mut len = label.chars().count();
        if strip_articles(&label) == haystack {
            len += 10_000;
        }
        if best.as_ref().is_none_or(|(best_len, _)| len > *best_len) {
            best = Some((len, subject));
        }
    }
    let (_, subject) = best?;
    let (subject_id, nodes) = nodes_for(bundle, &index, &sm_chapters, subject);
    let datasheets = nodes
        .into_iter()
        .filter(|id| bundle.node(id).is_some_and(|node| node.kind == "Datasheet"))
        .collect();
    Some(RosterNodes {
        subject: subject_id,
        datasheets,
    })
}

fn roster_keyword(label: &str, id: &str, sm_chapters: &HashSet<String>) -> bool {
    label == DAEMON_PARENT
        || DAEMON_GODS.contains(&label)
        || CSM_CHAPTERS.contains(&label)
        || sm_chapters.contains(id)
}

fn nodes_for(
    bundle: &Bundle,
    index: &KeywordIndex,
    sm_chapters: &HashSet<String>,
    subject: Subject,
) -> (String, Vec<String>) {
    match subject {
        Subject::Faction(faction) => {
            let label = bundle
                .node(&faction)
                .map(|node| fold(&node.label))
                .unwrap_or_default();
            let mut nodes = Vec::new();
            for id in bundle.faction_datasheets(&faction) {
                push_unique(&mut nodes, id);
            }
            if CSM_CHAPTERS.contains(&label.as_str()) {
                append_generic(index, CSM_PARENT, &mut nodes);
            }
            (faction, nodes)
        }
        Subject::Keyword(keyword) => {
            let label = index.labels.get(&keyword).cloned().unwrap_or_default();
            if label == DAEMON_PARENT {
                let nodes = daemon_datasheets(index, None);
                return (keyword, nodes);
            }
            if DAEMON_GODS.contains(&label.as_str()) {
                let nodes = daemon_datasheets(index, Some(label.as_str()));
                return (keyword, nodes);
            }
            let parent = if sm_chapters.contains(&keyword) {
                SM_PARENT
            } else {
                CSM_PARENT
            };
            let mut nodes = Vec::new();
            for id in &index.order {
                if index.faction_ids(id).iter().any(|kw| kw == &keyword) {
                    push_unique(&mut nodes, id.clone());
                }
            }
            append_generic(index, parent, &mut nodes);
            (keyword, nodes)
        }
    }
}

struct KeywordIndex {
    labels: HashMap<String, String>,
    faction: HashMap<String, Vec<String>>,
    all: HashMap<String, Vec<String>>,
    order: Vec<String>,
}

impl KeywordIndex {
    fn faction_ids(&self, datasheet: &str) -> &[String] {
        self.faction
            .get(datasheet)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    fn parent_id(&self, parent: &str) -> Option<&str> {
        self.labels
            .iter()
            .find(|(_, label)| label.as_str() == parent)
            .map(|(id, _)| id.as_str())
    }
}

fn keyword_index(bundle: &Bundle) -> KeywordIndex {
    let mut labels = HashMap::new();
    for node in bundle.nodes_of("Keyword") {
        labels.insert(node.id.clone(), fold(&node.label));
    }
    let mut faction: HashMap<String, Vec<String>> = HashMap::new();
    let mut all: HashMap<String, Vec<String>> = HashMap::new();
    let mut order = Vec::new();
    let mut seen = HashSet::new();
    for (datasheet, keyword, is_faction) in bundle.datasheet_keywords() {
        if seen.insert(datasheet.clone()) {
            order.push(datasheet.clone());
        }
        let label = labels.get(&keyword).map(String::as_str).unwrap_or("");
        if label.is_empty() {
            continue;
        }
        push_unique(all.entry(datasheet.clone()).or_default(), keyword.clone());
        if is_faction {
            push_unique(faction.entry(datasheet).or_default(), keyword);
        }
    }
    KeywordIndex {
        labels,
        faction,
        all,
        order,
    }
}

fn space_marine_chapters(index: &KeywordIndex) -> HashSet<String> {
    let Some(parent) = index.parent_id(SM_PARENT) else {
        return HashSet::new();
    };
    let mut chapters = HashSet::new();
    for ids in index.faction.values() {
        if !ids.iter().any(|id| id == parent) {
            continue;
        }
        for id in ids {
            if id != parent {
                chapters.insert(id.clone());
            }
        }
    }
    chapters
}

/// Datasheets whose only faction keyword is the parent: the generic units.
fn append_generic(index: &KeywordIndex, parent: &str, nodes: &mut Vec<String>) {
    let Some(parent_id) = index.parent_id(parent).map(str::to_string) else {
        return;
    };
    for id in &index.order {
        let ids = index.faction_ids(id);
        if ids.len() == 1 && ids[0] == parent_id {
            push_unique(nodes, id.clone());
        }
    }
}

/// Datasheets whose only faction keyword is Legiones Daemonica. `god` keeps
/// sheets that have that god keyword and sheets that have none of the four.
fn daemon_datasheets(index: &KeywordIndex, god: Option<&str>) -> Vec<String> {
    let Some(parent) = index.parent_id(DAEMON_PARENT).map(str::to_string) else {
        return Vec::new();
    };
    let mut nodes = Vec::new();
    for id in &index.order {
        let ids = index.faction_ids(id);
        if ids.len() != 1 || ids[0] != parent {
            continue;
        }
        if let Some(god) = god {
            let gods = index
                .all
                .get(id)
                .into_iter()
                .flatten()
                .filter_map(|keyword| index.labels.get(keyword))
                .filter(|label| DAEMON_GODS.contains(&label.as_str()))
                .collect::<Vec<_>>();
            if !gods.is_empty() && !gods.iter().any(|label| label.as_str() == god) {
                continue;
            }
        }
        push_unique(&mut nodes, id.clone());
    }
    nodes
}

fn push_unique(out: &mut Vec<String>, id: String) {
    if !out.contains(&id) {
        out.push(id);
    }
}
