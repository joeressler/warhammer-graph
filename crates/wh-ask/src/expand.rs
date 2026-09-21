use std::collections::HashSet;

use crate::bundle::LoadedBundle;

const EXPAND_KINDS: &[&str] = &[
    "FACTION_HAS_DATASHEET",
    "FACTION_HAS_DETACHMENT",
    "FACTION_HAS_ABILITY",
    "FACTION_HAS_STRATAGEM",
    "FACTION_HAS_ENHANCEMENT",
    "FACTION_HAS_DETACHMENT_ABILITY",
    "DATASHEET_HAS_KEYWORD",
    "DATASHEET_HAS_ABILITY",
    "DETACHMENT_HAS_ABILITY",
    "DETACHMENT_HAS_STRATAGEM",
    "DETACHMENT_HAS_ENHANCEMENT",
    "DATASHEET_HAS_DETACHMENT_ABILITY",
];

const EXTRA_DATASHEET_SEEDS: &[&str] = &[
    "Keyword",
    "Ability",
    "Stratagem",
    "Enhancement",
    "Detachment",
    "DetachmentAbility",
];

/// Seeds stay first. The tail is factions, then keywords, abilities, and
/// detachments, then any other nodes. The tail is capped so the list is at
/// most 32 nodes unless the seeds themselves exceed that.
pub struct ContextNodes {
    pub seeds: Vec<String>,
    pub nodes: Vec<String>,
}

pub fn expand(bundle: &LoadedBundle, seed_passages: &[usize]) -> ContextNodes {
    let seeds = seed_passages
        .iter()
        .filter_map(|index| bundle.passages.get(*index))
        .map(|passage| passage.node_id.clone())
        .collect::<Vec<_>>();

    let mut factions = Vec::new();
    for seed in &seeds {
        push_kind(
            bundle,
            &adjacent(bundle, seed, EXPAND_KINDS),
            "Faction",
            &mut factions,
        );
    }

    let mut datasheets = Vec::new();
    for seed in &seeds {
        if kind(bundle, seed) == "Datasheet" {
            push_unique(&mut datasheets, seed.clone());
        }
    }
    let mut extras = Vec::new();
    for seed in &seeds {
        if EXTRA_DATASHEET_SEEDS.contains(&kind(bundle, seed)) {
            push_kind(
                bundle,
                &adjacent(bundle, seed, EXPAND_KINDS),
                "Datasheet",
                &mut extras,
            );
        }
    }
    extras.retain(|id| !datasheets.contains(id));
    extras.truncate(8);
    datasheets.extend(extras.iter().cloned());

    let mut keywords = Vec::new();
    let mut abilities = Vec::new();
    let mut detachments = Vec::new();
    for datasheet in &datasheets {
        push_kind(
            bundle,
            &adjacent(bundle, datasheet, &["DATASHEET_HAS_KEYWORD"]),
            "Keyword",
            &mut keywords,
        );
        push_kind(
            bundle,
            &adjacent(bundle, datasheet, &["DATASHEET_HAS_ABILITY"]),
            "Ability",
            &mut abilities,
        );
        for ability in adjacent(bundle, datasheet, &["DATASHEET_HAS_DETACHMENT_ABILITY"]) {
            push_kind(
                bundle,
                &adjacent(bundle, &ability, &["DETACHMENT_HAS_ABILITY"]),
                "Detachment",
                &mut detachments,
            );
        }
    }
    for seed in &seeds {
        match kind(bundle, seed) {
            "Stratagem" => push_kind(
                bundle,
                &adjacent(bundle, seed, &["DETACHMENT_HAS_STRATAGEM"]),
                "Detachment",
                &mut detachments,
            ),
            "Enhancement" => push_kind(
                bundle,
                &adjacent(bundle, seed, &["DETACHMENT_HAS_ENHANCEMENT"]),
                "Detachment",
                &mut detachments,
            ),
            _ => {}
        }
    }

    let mut others = Vec::new();
    for datasheet in &datasheets {
        if !seeds.contains(datasheet) {
            push_unique(&mut others, datasheet.clone());
        }
    }

    let mut nodes = seeds.clone();
    let mut seen = seeds.iter().cloned().collect::<HashSet<_>>();
    for group in [factions, keywords, abilities, detachments, others] {
        for id in group {
            if seen.insert(id.clone()) {
                nodes.push(id);
            }
        }
    }
    let seed_len = seeds.len();
    if nodes.len() > 32 {
        if seed_len >= 32 {
            nodes.truncate(seed_len);
        } else {
            nodes.truncate(32);
        }
    }
    ContextNodes { seeds, nodes }
}

fn kind<'a>(bundle: &'a LoadedBundle, id: &str) -> &'a str {
    bundle.node(id).map(|node| node.kind.as_str()).unwrap_or("")
}

fn adjacent(bundle: &LoadedBundle, id: &str, kinds: &[&str]) -> Vec<String> {
    bundle
        .edges
        .iter()
        .filter(|edge| kinds.iter().any(|kind| edge.kind == *kind))
        .filter_map(|edge| {
            if edge.from == id {
                Some(edge.to.clone())
            } else if edge.to == id {
                Some(edge.from.clone())
            } else {
                None
            }
        })
        .collect()
}

fn push_kind(bundle: &LoadedBundle, ids: &[String], expected: &str, out: &mut Vec<String>) {
    for id in ids {
        if kind(bundle, id) == expected {
            push_unique(out, id.clone());
        }
    }
}

fn push_unique(out: &mut Vec<String>, id: String) {
    if !out.contains(&id) {
        out.push(id);
    }
}
