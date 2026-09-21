use std::collections::{BTreeMap, HashMap, HashSet};

use sha2::{Digest, Sha256};

use crate::attrs::Attrs;
use crate::corpus::{field, hex_encode, ref_id, Corpus, Entity};
use crate::model::{BundleManifest, EdgeRecord, GraphNode, Passage};
use crate::textutil::{
    cmp_integer_strings, integer_token, join_parts, normalize_keyword, part, quote_segment,
    with_line,
};

struct KeywordAccum {
    label: String,
    normalized: String,
}

#[derive(Clone)]
struct Profile {
    entity_id: String,
    line_in_wargear: String,
    name: String,
    range: String,
    weapon_type: String,
    attacks: String,
    skill: String,
    strength: String,
    ap: String,
    damage: String,
    description: String,
    datasheet_ref: Option<String>,
}

struct WargearGroup {
    datasheet_id: String,
    line: String,
    profiles: Vec<Profile>,
}

pub struct GraphBundle {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<EdgeRecord>,
    pub passages: Vec<Passage>,
    pub manifest: BundleManifest,
}

pub fn build_graph(corpus: &Corpus) -> GraphBundle {
    let by_id = corpus
        .entities
        .iter()
        .map(|entity| (entity.id.as_str(), entity))
        .collect::<HashMap<_, _>>();
    let mut keywords: Vec<KeywordAccum> = Vec::new();
    let mut keyword_at: HashMap<String, usize> = HashMap::new();
    let mut groups: Vec<WargearGroup> = Vec::new();
    let mut group_at: HashMap<(String, String), usize> = HashMap::new();
    let mut nodes = Vec::new();

    for entity in &corpus.entities {
        match entity.table.as_str() {
            "datasheet_keyword" => record_keyword(entity, &mut keywords, &mut keyword_at),
            "datasheet_wargear" => record_wargear(entity, &mut groups, &mut group_at),
            "datasheet_ability" if ref_id(entity, "ability_id").is_some() => {}
            "datasheet_stratagem"
            | "datasheet_enhancement"
            | "datasheet_detachment_ability"
            | "datasheet_leader" => {}
            "faction"
            | "source"
            | "datasheet"
            | "datasheet_model"
            | "ability"
            | "datasheet_ability"
            | "datasheet_option"
            | "datasheet_unit_composition"
            | "datasheet_model_cost"
            | "stratagem"
            | "enhancement"
            | "detachment"
            | "detachment_ability" => {
                nodes.push(entity_node(entity, &by_id));
            }
            _ => {}
        }
    }

    for group in &groups {
        nodes.push(wargear_node(group, &by_id));
    }
    for keyword in &keywords {
        nodes.push(keyword_node(keyword));
    }

    let node_ids = nodes
        .iter()
        .map(|node| node.id.clone())
        .collect::<HashSet<_>>();
    let mut edges = Vec::new();
    let mut seen = HashSet::new();
    for entity in &corpus.entities {
        for edge in edges_for_entity(entity, &keyword_at) {
            push_edge(&mut edges, &mut seen, &node_ids, edge);
        }
    }
    for group in &groups {
        if let Some(edge) = wargear_edge(group, &node_ids) {
            push_edge(&mut edges, &mut seen, &node_ids, edge);
        }
    }

    let passages = nodes
        .iter()
        .map(|node| Passage {
            node_id: node.id.clone(),
            title: node.label.clone(),
            text: node.text.clone(),
            wahapedia_link: node.source_url.clone(),
        })
        .collect::<Vec<_>>();

    let manifest = manifest_for(corpus, &nodes, &edges, passages.len());
    GraphBundle {
        nodes,
        edges,
        passages,
        manifest,
    }
}

fn record_keyword(
    entity: &Entity,
    keywords: &mut Vec<KeywordAccum>,
    keyword_at: &mut HashMap<String, usize>,
) {
    let (label, normalized) = normalize_keyword(field(entity, "keyword"));
    if keyword_at.contains_key(&normalized) {
        return;
    }
    keyword_at.insert(normalized.clone(), keywords.len());
    keywords.push(KeywordAccum { label, normalized });
}

fn record_wargear(
    entity: &Entity,
    groups: &mut Vec<WargearGroup>,
    group_at: &mut HashMap<(String, String), usize>,
) {
    let key = (
        field(entity, "datasheet_id").to_string(),
        field(entity, "line").to_string(),
    );
    let profile = Profile {
        entity_id: entity.id.clone(),
        line_in_wargear: field(entity, "line_in_wargear").to_string(),
        name: field(entity, "name").to_string(),
        range: field(entity, "range").to_string(),
        weapon_type: field(entity, "type").to_string(),
        attacks: field(entity, "A").to_string(),
        skill: field(entity, "BS_WS").to_string(),
        strength: field(entity, "S").to_string(),
        ap: field(entity, "AP").to_string(),
        damage: field(entity, "D").to_string(),
        description: field(entity, "description").to_string(),
        datasheet_ref: ref_id(entity, "datasheet_id").map(str::to_string),
    };
    if let Some(index) = group_at.get(&key) {
        groups[*index].profiles.push(profile);
        return;
    }
    group_at.insert(key.clone(), groups.len());
    groups.push(WargearGroup {
        datasheet_id: key.0,
        line: key.1,
        profiles: vec![profile],
    });
}

fn entity_node(entity: &Entity, by_id: &HashMap<&str, &Entity>) -> GraphNode {
    let (kind, label, text, attrs) = match entity.table.as_str() {
        "faction" => (
            "Faction",
            field(entity, "name").to_string(),
            join_parts([part("name", field(entity, "name"))]),
            Attrs::default(),
        ),
        "source" => (
            "Source",
            field(entity, "name").to_string(),
            join_parts([
                part("type", field(entity, "type")),
                part("name", field(entity, "name")),
                part("edition", field(entity, "edition")),
                part("version", field(entity, "version")),
            ]),
            Attrs::default(),
        ),
        "datasheet" => (
            "Datasheet",
            field(entity, "name").to_string(),
            datasheet_text(entity),
            datasheet_attrs(entity),
        ),
        "datasheet_model" => (
            "Model",
            field(entity, "name").to_string(),
            model_text(entity),
            model_attrs(entity),
        ),
        "ability" => (
            "Ability",
            field(entity, "name").to_string(),
            join_parts([
                part("name", field(entity, "name")),
                part("legend", field(entity, "legend")),
                part("description", field(entity, "description")),
            ]),
            Attrs::default(),
        ),
        "datasheet_ability" => {
            let name = field(entity, "name");
            let label = if name.is_empty() {
                "Ability".to_string()
            } else {
                name.to_string()
            };
            (
                "Ability",
                label,
                join_parts([
                    part("name", field(entity, "name")),
                    part("type", field(entity, "type")),
                    part("parameter", field(entity, "parameter")),
                    part("description", field(entity, "description")),
                ]),
                Attrs::default(),
            )
        }
        "datasheet_option" => (
            "Option",
            with_line("Option", field(entity, "line")),
            join_parts([
                part("button", field(entity, "button")),
                part("description", field(entity, "description")),
            ]),
            Attrs::default(),
        ),
        "datasheet_unit_composition" => (
            "UnitComposition",
            with_line("Unit composition", field(entity, "line")),
            join_parts([part("description", field(entity, "description"))]),
            Attrs::default(),
        ),
        "datasheet_model_cost" => (
            "ModelCost",
            with_line("Cost", field(entity, "line")),
            join_parts([
                part("description", field(entity, "description")),
                part("cost", field(entity, "cost")),
            ]),
            Attrs::default(),
        ),
        "stratagem" => (
            "Stratagem",
            field(entity, "name").to_string(),
            join_parts([
                part("name", field(entity, "name")),
                part("type", field(entity, "type")),
                part("cp_cost", field(entity, "cp_cost")),
                part("turn", field(entity, "turn")),
                part("phase", field(entity, "phase")),
                part("detachment", field(entity, "detachment")),
                part("description", field(entity, "description")),
            ]),
            Attrs::default(),
        ),
        "enhancement" => (
            "Enhancement",
            field(entity, "name").to_string(),
            join_parts([
                part("name", field(entity, "name")),
                part("cost", field(entity, "cost")),
                part("detachment", field(entity, "detachment")),
                part("legend", field(entity, "legend")),
                part("description", field(entity, "description")),
            ]),
            Attrs::default(),
        ),
        "detachment" => (
            "Detachment",
            field(entity, "name").to_string(),
            join_parts([
                part("name", field(entity, "name")),
                part("type", field(entity, "type")),
                part("legend", field(entity, "legend")),
            ]),
            Attrs::default(),
        ),
        "detachment_ability" => (
            "DetachmentAbility",
            field(entity, "name").to_string(),
            join_parts([
                part("name", field(entity, "name")),
                part("detachment", field(entity, "detachment")),
                part("legend", field(entity, "legend")),
                part("description", field(entity, "description")),
            ]),
            Attrs::default(),
        ),
        _ => ("", String::new(), String::new(), Attrs::default()),
    };
    GraphNode {
        id: entity.id.clone(),
        kind: kind.to_string(),
        label,
        text,
        attrs,
        source_url: entity_source_url(entity, by_id),
    }
}

fn datasheet_text(entity: &Entity) -> String {
    join_parts([
        part("name", field(entity, "name")),
        part("role", field(entity, "role")),
        part("loadout", field(entity, "loadout")),
        part("transport", field(entity, "transport")),
        part("leader_head", field(entity, "leader_head")),
        part("leader_footer", field(entity, "leader_footer")),
        part("damaged_w", field(entity, "damaged_w")),
        part("damaged_description", field(entity, "damaged_description")),
    ])
}

fn datasheet_attrs(entity: &Entity) -> Attrs {
    let mut attrs = Attrs::default();
    attrs.insert_text("role", field(entity, "role"));
    attrs.insert_text("virtual", field(entity, "virtual"));
    attrs.insert_text("faction_id", field(entity, "faction_id"));
    attrs.insert_text("faction_node", ref_id(entity, "faction_id").unwrap_or(""));
    attrs
}

fn model_text(entity: &Entity) -> String {
    join_parts([
        part("name", field(entity, "name")),
        part("M", field(entity, "M")),
        part("T", field(entity, "T")),
        part("Sv", field(entity, "Sv")),
        part("inv_sv", field(entity, "inv_sv")),
        part("inv_sv_descr", field(entity, "inv_sv_descr")),
        part("W", field(entity, "W")),
        part("Ld", field(entity, "Ld")),
        part("OC", field(entity, "OC")),
        part("base_size", field(entity, "base_size")),
        part("base_size_descr", field(entity, "base_size_descr")),
    ])
}

fn model_attrs(entity: &Entity) -> Attrs {
    let mut attrs = Attrs::default();
    for key in ["line", "M", "T", "Sv", "W", "Ld", "OC"] {
        attrs.insert_text(key, field(entity, key));
    }
    attrs
}

fn keyword_node(keyword: &KeywordAccum) -> GraphNode {
    let mut attrs = Attrs::default();
    attrs.insert_text("normalized", &keyword.normalized);
    let id = format!("10ed:keyword:{}", quote_segment(&keyword.normalized));
    GraphNode {
        id,
        kind: "Keyword".to_string(),
        label: keyword.label.clone(),
        text: keyword.label.clone(),
        attrs,
        source_url: None,
    }
}

fn wargear_node(group: &WargearGroup, by_id: &HashMap<&str, &Entity>) -> GraphNode {
    let mut profiles = group.profiles.clone();
    sort_profiles(&mut profiles);
    let label = profiles
        .first()
        .map(|profile| profile.name.clone())
        .unwrap_or_default();
    let mut parts = Vec::new();
    if !label.is_empty() {
        parts.push(label.clone());
    }
    for profile in &profiles {
        let stats = [
            profile.range.as_str(),
            profile.weapon_type.as_str(),
            profile.attacks.as_str(),
            profile.skill.as_str(),
            profile.strength.as_str(),
            profile.ap.as_str(),
            profile.damage.as_str(),
        ]
        .into_iter()
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
        if !stats.is_empty() {
            parts.push(stats);
        }
        let description = part("description", &profile.description);
        if !description.is_empty() {
            parts.push(description);
        }
    }
    let mut attrs = Attrs::default();
    attrs.insert_text("line", &group.line);
    attrs.insert_list(
        "profile_entity_ids",
        profiles
            .iter()
            .map(|profile| profile.entity_id.clone())
            .collect(),
    );
    GraphNode {
        id: format!(
            "10ed:wargear:{}:{}",
            quote_segment(&group.datasheet_id),
            quote_segment(&group.line)
        ),
        kind: "Wargear".to_string(),
        label,
        text: parts.join("\n"),
        attrs,
        source_url: wargear_source_url(group, by_id),
    }
}

fn sort_profiles(profiles: &mut [Profile]) {
    let numeric = profiles
        .iter()
        .all(|profile| integer_token(&profile.line_in_wargear).is_some());
    if numeric {
        profiles.sort_by(|left, right| {
            cmp_integer_strings(&left.line_in_wargear, &right.line_in_wargear)
        });
    } else {
        profiles.sort_by(|left, right| left.line_in_wargear.cmp(&right.line_in_wargear));
    }
}

fn entity_source_url(entity: &Entity, by_id: &HashMap<&str, &Entity>) -> Option<String> {
    if let Some(link) = non_empty(field(entity, "link")) {
        return Some(link.to_string());
    }
    for key in ["datasheet_id", "leader_id"] {
        if let Some(id) = ref_id(entity, key) {
            if let Some(link) = link_of(by_id, id) {
                return Some(link);
            }
        }
    }
    if let Some(id) = ref_id(entity, "faction_id") {
        return link_of(by_id, id);
    }
    None
}

fn wargear_source_url(group: &WargearGroup, by_id: &HashMap<&str, &Entity>) -> Option<String> {
    group.profiles.iter().find_map(|profile| {
        profile
            .datasheet_ref
            .as_deref()
            .and_then(|id| link_of(by_id, id))
    })
}

fn link_of(by_id: &HashMap<&str, &Entity>, id: &str) -> Option<String> {
    let entity = by_id.get(id)?;
    non_empty(field(entity, "link")).map(str::to_string)
}

fn non_empty(value: &str) -> Option<&str> {
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

struct PendingEdge {
    kind: &'static str,
    from: String,
    to: String,
    attrs: Attrs,
}

fn edges_for_entity(entity: &Entity, keyword_at: &HashMap<String, usize>) -> Vec<PendingEdge> {
    match entity.table.as_str() {
        "datasheet" => {
            let mut edges = Vec::new();
            if let Some(faction) = ref_id(entity, "faction_id") {
                edges.push(bare("FACTION_HAS_DATASHEET", faction, &entity.id));
            }
            if let Some(source) = ref_id(entity, "source_id") {
                edges.push(bare("DATASHEET_FROM_SOURCE", &entity.id, source));
            }
            edges
        }
        "stratagem" => optional_pair(
            entity,
            "FACTION_HAS_STRATAGEM",
            "faction_id",
            "DETACHMENT_HAS_STRATAGEM",
            "detachment_id",
        ),
        "ability" => optional_from_ref(entity, "FACTION_HAS_ABILITY", "faction_id"),
        "enhancement" => optional_pair(
            entity,
            "FACTION_HAS_ENHANCEMENT",
            "faction_id",
            "DETACHMENT_HAS_ENHANCEMENT",
            "detachment_id",
        ),
        "detachment" => optional_from_ref(entity, "FACTION_HAS_DETACHMENT", "faction_id"),
        "detachment_ability" => optional_pair(
            entity,
            "FACTION_HAS_DETACHMENT_ABILITY",
            "faction_id",
            "DETACHMENT_HAS_ABILITY",
            "detachment_id",
        ),
        "datasheet_model" => line_edge(entity, "DATASHEET_HAS_MODEL"),
        "datasheet_ability" => ability_edge(entity),
        "datasheet_keyword" => keyword_edge(entity, keyword_at),
        "datasheet_option" => line_edge(entity, "DATASHEET_HAS_OPTION"),
        "datasheet_unit_composition" => line_edge(entity, "DATASHEET_HAS_COMPOSITION"),
        "datasheet_model_cost" => line_edge(entity, "DATASHEET_HAS_COST"),
        "datasheet_leader" => both_refs(entity, "DATASHEET_CAN_LEAD", "leader_id", "attached_id"),
        "datasheet_stratagem" => both_refs(
            entity,
            "DATASHEET_USES_STRATAGEM",
            "datasheet_id",
            "stratagem_id",
        ),
        "datasheet_enhancement" => both_refs(
            entity,
            "ENHANCEMENT_APPLIES_TO_DATASHEET",
            "enhancement_id",
            "datasheet_id",
        ),
        "datasheet_detachment_ability" => both_refs(
            entity,
            "DATASHEET_HAS_DETACHMENT_ABILITY",
            "datasheet_id",
            "detachment_ability_id",
        ),
        _ => Vec::new(),
    }
}

fn bare(kind: &'static str, from: &str, to: &str) -> PendingEdge {
    PendingEdge {
        kind,
        from: from.to_string(),
        to: to.to_string(),
        attrs: Attrs::default(),
    }
}

fn optional_from_ref(entity: &Entity, kind: &'static str, column: &str) -> Vec<PendingEdge> {
    ref_id(entity, column)
        .map(|target| vec![bare(kind, target, &entity.id)])
        .unwrap_or_default()
}

fn optional_pair(
    entity: &Entity,
    faction_kind: &'static str,
    faction_column: &str,
    other_kind: &'static str,
    other_column: &str,
) -> Vec<PendingEdge> {
    let mut edges = optional_from_ref(entity, faction_kind, faction_column);
    if let Some(target) = ref_id(entity, other_column) {
        edges.push(bare(other_kind, target, &entity.id));
    }
    edges
}

fn line_edge(entity: &Entity, kind: &'static str) -> Vec<PendingEdge> {
    let Some(datasheet) = ref_id(entity, "datasheet_id") else {
        return Vec::new();
    };
    let mut attrs = Attrs::default();
    attrs.insert_text("line", field(entity, "line"));
    vec![PendingEdge {
        kind,
        from: datasheet.to_string(),
        to: entity.id.clone(),
        attrs,
    }]
}

fn ability_edge(entity: &Entity) -> Vec<PendingEdge> {
    let Some(datasheet) = ref_id(entity, "datasheet_id") else {
        return Vec::new();
    };
    let target = ref_id(entity, "ability_id").unwrap_or(entity.id.as_str());
    let mut attrs = Attrs::default();
    attrs.insert_text("line", field(entity, "line"));
    attrs.insert_text("model", field(entity, "model"));
    attrs.insert_text("type", field(entity, "type"));
    attrs.insert_text("parameter", field(entity, "parameter"));
    vec![PendingEdge {
        kind: "DATASHEET_HAS_ABILITY",
        from: datasheet.to_string(),
        to: target.to_string(),
        attrs,
    }]
}

fn keyword_edge(entity: &Entity, keyword_at: &HashMap<String, usize>) -> Vec<PendingEdge> {
    let Some(datasheet) = ref_id(entity, "datasheet_id") else {
        return Vec::new();
    };
    let (_, normalized) = normalize_keyword(field(entity, "keyword"));
    if !keyword_at.contains_key(&normalized) {
        return Vec::new();
    }
    let mut attrs = Attrs::default();
    attrs.insert_text("model", field(entity, "model"));
    attrs.insert_text("is_faction_keyword", field(entity, "is_faction_keyword"));
    vec![PendingEdge {
        kind: "DATASHEET_HAS_KEYWORD",
        from: datasheet.to_string(),
        to: format!("10ed:keyword:{}", quote_segment(&normalized)),
        attrs,
    }]
}

fn both_refs(
    entity: &Entity,
    kind: &'static str,
    from_column: &str,
    to_column: &str,
) -> Vec<PendingEdge> {
    match (ref_id(entity, from_column), ref_id(entity, to_column)) {
        (Some(from), Some(to)) => vec![bare(kind, from, to)],
        _ => Vec::new(),
    }
}

fn wargear_edge(group: &WargearGroup, node_ids: &HashSet<String>) -> Option<PendingEdge> {
    let datasheet = group
        .profiles
        .iter()
        .find_map(|profile| profile.datasheet_ref.clone())?;
    let to = format!(
        "10ed:wargear:{}:{}",
        quote_segment(&group.datasheet_id),
        quote_segment(&group.line)
    );
    if !node_ids.contains(&datasheet) || !node_ids.contains(&to) {
        return None;
    }
    let mut attrs = Attrs::default();
    attrs.insert_text("line", &group.line);
    Some(PendingEdge {
        kind: "DATASHEET_HAS_WARGEAR",
        from: datasheet,
        to,
        attrs,
    })
}

fn push_edge(
    edges: &mut Vec<EdgeRecord>,
    seen: &mut HashSet<String>,
    node_ids: &HashSet<String>,
    pending: PendingEdge,
) {
    if !node_ids.contains(&pending.from) || !node_ids.contains(&pending.to) {
        return;
    }
    let canonical = canonical_attrs(&pending.attrs);
    let dedupe_key = format!(
        "{}\n{}\n{}\n{canonical}",
        pending.kind, pending.from, pending.to
    );
    if !seen.insert(dedupe_key) {
        return;
    }
    let id = edge_id(pending.kind, &pending.from, &pending.to, &canonical);
    edges.push(EdgeRecord {
        id,
        kind: pending.kind.to_string(),
        from: pending.from,
        to: pending.to,
        attrs: pending.attrs,
    });
}

pub fn canonical_attrs(attrs: &Attrs) -> String {
    serde_json::to_string(attrs).expect("attribute maps are JSON values")
}

fn edge_id(kind: &str, from: &str, to: &str, canonical: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(from.as_bytes());
    hasher.update(b"\n");
    hasher.update(to.as_bytes());
    hasher.update(b"\n");
    hasher.update(canonical.as_bytes());
    let digest = hasher.finalize();
    let hex = hex_encode(&digest);
    format!("10ed:edge:{kind}:{}", &hex[..16])
}

fn manifest_for(
    corpus: &Corpus,
    nodes: &[GraphNode],
    edges: &[EdgeRecord],
    passage_count: usize,
) -> BundleManifest {
    let mut node_counts_by_kind = BTreeMap::new();
    for node in nodes {
        *node_counts_by_kind.entry(node.kind.clone()).or_insert(0) += 1;
    }
    let mut edge_counts_by_kind = BTreeMap::new();
    for edge in edges {
        *edge_counts_by_kind.entry(edge.kind.clone()).or_insert(0) += 1;
    }
    BundleManifest {
        format_version: 1,
        corpus_schema_version: 1,
        edition: corpus.edition.clone(),
        corpus_fingerprint: corpus.fingerprint.clone(),
        last_update: corpus.last_update.clone(),
        node_count: nodes.len(),
        edge_count: edges.len(),
        passage_count,
        node_counts_by_kind,
        edge_counts_by_kind,
    }
}
