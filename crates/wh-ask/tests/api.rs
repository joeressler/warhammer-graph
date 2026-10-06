//! The public API against a small hand-built bundle.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use wh_ask::{AskError, Bundle, Direction};
use wh_graph::{
    write_graph_db, Attrs, BundleManifest, EdgeRecord, GraphNode, Passage, FORMAT_VERSION,
    GRAPH_DB,
};

#[derive(Default)]
struct Fixture {
    nodes: Vec<GraphNode>,
    edges: Vec<EdgeRecord>,
}

impl Fixture {
    fn node(&mut self, id: &str, kind: &str, label: &str, text: &str, attrs: &[(&str, &str)]) {
        let mut found = Attrs::default();
        for (key, value) in attrs {
            found.insert_text(key, *value);
        }
        self.nodes.push(GraphNode {
            id: id.to_string(),
            kind: kind.to_string(),
            label: label.to_string(),
            text: text.to_string(),
            attrs: found,
            source_url: Some(format!("https://example.invalid/{label}")),
        });
    }

    fn edge(&mut self, kind: &str, from: &str, to: &str, attrs: &[(&str, &str)]) {
        let mut found = Attrs::default();
        for (key, value) in attrs {
            found.insert_text(key, *value);
        }
        self.edges.push(EdgeRecord {
            id: format!("10ed:edge:{kind}:{:016x}", self.edges.len()),
            kind: kind.to_string(),
            from: from.to_string(),
            to: to.to_string(),
            attrs: found,
        });
    }

    fn faction(&mut self, id: &str, label: &str) {
        self.node(id, "Faction", label, label, &[]);
    }

    fn keyword(&mut self, id: &str, label: &str) {
        self.node(id, "Keyword", label, label, &[]);
    }

    fn datasheet(&mut self, id: &str, label: &str, faction: &str, keywords: &[(&str, bool)]) {
        self.node(
            id,
            "Datasheet",
            label,
            label,
            &[("faction_node", faction), ("role", "Battleline")],
        );
        self.edge("FACTION_HAS_DATASHEET", faction, id, &[]);
        for (keyword, is_faction) in keywords {
            self.edge(
                "DATASHEET_HAS_KEYWORD",
                id,
                keyword,
                &[("is_faction_keyword", if *is_faction { "true" } else { "false" })],
            );
        }
    }

    fn write(self, name: &str) -> (PathBuf, Bundle) {
        let dir = std::env::temp_dir().join(format!("wh-ask-api-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let passages = self
            .nodes
            .iter()
            .map(|node| Passage {
                node_id: node.id.clone(),
                title: node.label.clone(),
                text: node.text.clone(),
                wahapedia_link: node.source_url.clone(),
            })
            .collect::<Vec<_>>();
        let manifest = BundleManifest {
            format_version: FORMAT_VERSION,
            corpus_schema_version: 1,
            edition: "10ed".to_string(),
            corpus_fingerprint: "ab".repeat(32),
            last_update: "2020-01-01".to_string(),
            node_count: self.nodes.len(),
            edge_count: self.edges.len(),
            passage_count: passages.len(),
            node_counts_by_kind: counts(self.nodes.iter().map(|item| item.kind.as_str())),
            edge_counts_by_kind: counts(self.edges.iter().map(|item| item.kind.as_str())),
        };
        write_graph_db(
            &dir.join(GRAPH_DB),
            &self.nodes,
            &self.edges,
            &passages,
            &manifest,
            "nodes",
            "passages",
        )
        .unwrap();
        fs::write(
            dir.join("manifest.json"),
            serde_json::to_string(&manifest).unwrap(),
        )
        .unwrap();
        let bundle = Bundle::open(&dir).unwrap();
        (dir, bundle)
    }
}

fn counts<'a>(kinds: impl Iterator<Item = &'a str>) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for kind in kinds {
        *counts.entry(kind.to_string()).or_insert(0) += 1;
    }
    counts
}

fn names(units: &[wh_ask::UnitRef]) -> Vec<&str> {
    units.iter().map(|unit| unit.name.as_str()).collect()
}

/// Space Marines with two chapters and generic units, plus Chaos Daemons.
fn armies() -> Fixture {
    let mut f = Fixture::default();
    f.faction("10ed:faction:SM", "Space Marines");
    f.keyword("10ed:keyword:astartes", "Adeptus Astartes");
    f.keyword("10ed:keyword:ultramarines", "Ultramarines");
    f.keyword("10ed:keyword:bloodangels", "Blood Angels");
    f.keyword("10ed:keyword:infantry", "Infantry");
    let astartes = ("10ed:keyword:astartes", true);
    f.datasheet(
        "10ed:datasheet:INT",
        "Intercessor Squad",
        "10ed:faction:SM",
        &[astartes, ("10ed:keyword:infantry", false)],
    );
    f.datasheet(
        "10ed:datasheet:UMH",
        "Ultramarines Hero",
        "10ed:faction:SM",
        &[astartes, ("10ed:keyword:ultramarines", true)],
    );
    f.datasheet(
        "10ed:datasheet:BAK",
        "Blood Knight",
        "10ed:faction:SM",
        &[astartes, ("10ed:keyword:bloodangels", true)],
    );

    f.faction("10ed:faction:CD", "Chaos Daemons");
    f.keyword("10ed:keyword:daemonica", "Legiones Daemonica");
    f.keyword("10ed:keyword:khorne", "Khorne");
    f.keyword("10ed:keyword:nurgle", "Nurgle");
    let daemonica = ("10ed:keyword:daemonica", true);
    f.datasheet(
        "10ed:datasheet:BLD",
        "Bloodletters",
        "10ed:faction:CD",
        &[daemonica, ("10ed:keyword:khorne", false)],
    );
    f.datasheet(
        "10ed:datasheet:PLG",
        "Plaguebearers",
        "10ed:faction:CD",
        &[daemonica, ("10ed:keyword:nurgle", false)],
    );
    f.datasheet(
        "10ed:datasheet:GEN",
        "Generic Daemon",
        "10ed:faction:CD",
        &[daemonica],
    );
    f
}

#[test]
fn a_faction_roster_is_every_datasheet_it_has() {
    let (_dir, bundle) = armies().write("faction-roster");
    let roster = bundle.roster("Space Marines").unwrap();
    assert_eq!(roster.subject.name, "Space Marines");
    assert_eq!(
        names(&roster.units),
        ["Intercessor Squad", "Ultramarines Hero", "Blood Knight"]
    );
    assert_eq!(roster.units[0].faction.as_deref(), Some("Space Marines"));
    assert_eq!(roster.units[0].role.as_deref(), Some("Battleline"));
}

#[test]
fn a_chapter_roster_adds_the_generic_parent_units_but_not_other_chapters() {
    let (_dir, bundle) = armies().write("chapter-roster");
    let roster = bundle.roster("Ultramarines").unwrap();
    assert_eq!(roster.subject.kind, "Keyword");
    assert_eq!(names(&roster.units), ["Ultramarines Hero", "Intercessor Squad"]);
}

#[test]
fn a_daemon_god_roster_keeps_godless_daemons_and_drops_other_gods() {
    let (_dir, bundle) = armies().write("daemon-roster");
    let khorne = bundle.roster("Khorne").unwrap();
    assert_eq!(names(&khorne.units), ["Bloodletters", "Generic Daemon"]);
    let all = bundle.roster("Legiones Daemonica").unwrap();
    assert_eq!(
        names(&all.units),
        ["Bloodletters", "Plaguebearers", "Generic Daemon"]
    );
}

#[test]
fn a_roster_takes_an_id_and_a_name_with_extra_words() {
    let (_dir, bundle) = armies().write("roster-input");
    assert_eq!(bundle.roster("10ed:faction:CD").unwrap().units.len(), 3);
    assert_eq!(bundle.roster("the Space Marines faction").unwrap().units.len(), 3);
    assert!(matches!(
        bundle.roster("10ed:datasheet:INT"),
        Err(AskError::Invalid(_))
    ));
    match bundle.roster("Squats") {
        Err(AskError::NotFound { suggestions, .. }) => {
            assert!(suggestions.contains(&"Space Marines".to_string()));
        }
        other => panic!("expected NotFound, got {other:?}"),
    }
}

#[test]
fn a_shared_name_is_ambiguous_until_an_id_is_given() {
    let mut f = armies();
    f.datasheet("10ed:datasheet:DRN1", "Dreadnought", "10ed:faction:SM", &[]);
    f.datasheet("10ed:datasheet:DRN2", "Dreadnought", "10ed:faction:CD", &[]);
    let (_dir, bundle) = f.write("ambiguous");
    match bundle.unit("Dreadnought") {
        Err(AskError::Ambiguous { candidates, .. }) => {
            let details = candidates
                .iter()
                .map(|item| item.detail.as_str())
                .collect::<Vec<_>>();
            assert_eq!(details, ["Space Marines", "Chaos Daemons"]);
        }
        other => panic!("expected Ambiguous, got {other:?}"),
    }
    assert_eq!(bundle.find_units("dreadnought").len(), 2);
    let card = bundle.unit("10ed:datasheet:DRN2").unwrap();
    assert_eq!(card.unit.faction.as_deref(), Some("Chaos Daemons"));
}

#[test]
fn a_misspelled_unit_suggests_close_names() {
    let (_dir, bundle) = armies().write("suggest");
    match bundle.unit("Intercesor Squad") {
        Err(AskError::NotFound { suggestions, .. }) => {
            assert!(suggestions.contains(&"Intercessor Squad".to_string()), "{suggestions:?}");
        }
        other => panic!("expected NotFound, got {other:?}"),
    }
}

/// A Warboss who leads Boyz, with a core ability, her own ability, two weapon
/// profiles, a model line, and a points line.
fn warboss() -> Fixture {
    let mut f = Fixture::default();
    f.faction("10ed:faction:ORK", "Orks");
    f.keyword("10ed:keyword:infantry", "Infantry");
    f.datasheet(
        "10ed:datasheet:WB",
        "Warboss",
        "10ed:faction:ORK",
        &[("10ed:keyword:infantry", false)],
    );
    f.datasheet("10ed:datasheet:BOY", "Boyz", "10ed:faction:ORK", &[]);
    f.datasheet("10ed:datasheet:NOB", "Nobz", "10ed:faction:ORK", &[]);
    f.edge("DATASHEET_CAN_LEAD", "10ed:datasheet:WB", "10ed:datasheet:BOY", &[]);
    f.edge("DATASHEET_CAN_LEAD", "10ed:datasheet:WB", "10ed:datasheet:NOB", &[]);
    f.edge("DATASHEET_CAN_LEAD", "10ed:datasheet:NOB", "10ed:datasheet:BOY", &[]);

    f.node(
        "10ed:ability:FNP",
        "Ability",
        "Feel No Pain",
        "Feel No Pain\nCore\nEach time this model would lose a wound, roll one D6.",
        &[],
    );
    f.edge(
        "DATASHEET_HAS_ABILITY",
        "10ed:datasheet:WB",
        "10ed:ability:FNP",
        &[("type", "Core"), ("parameter", "5+"), ("model", "")],
    );
    f.edge(
        "DATASHEET_HAS_ABILITY",
        "10ed:datasheet:BOY",
        "10ed:ability:FNP",
        &[("type", "Core"), ("parameter", "6+"), ("model", "Boy")],
    );
    f.node(
        "10ed:datasheet_ability:WB:3",
        "Ability",
        "Might is Right",
        "Might is Right\nDatasheet\nWhile leading a unit, add 1 to the Hit roll.",
        &[],
    );
    f.edge(
        "DATASHEET_HAS_ABILITY",
        "10ed:datasheet:WB",
        "10ed:datasheet_ability:WB:3",
        &[("type", "Datasheet"), ("parameter", ""), ("model", "")],
    );

    f.node(
        "10ed:wargear:WB:2",
        "Wargear",
        "Power Klaw - strike",
        "Power Klaw - strike\nMelee Melee 6 2 14 -3 4",
        &[],
    );
    f.node(
        "10ed:wargear:WB:3",
        "Wargear",
        "Power Klaw - sweep",
        "Power Klaw - sweep\nMelee Melee 12 2 8 -2 2\nsustained hits 1, lance",
        &[],
    );
    f.edge("DATASHEET_HAS_WARGEAR", "10ed:datasheet:WB", "10ed:wargear:WB:2", &[]);
    f.edge("DATASHEET_HAS_WARGEAR", "10ed:datasheet:WB", "10ed:wargear:WB:3", &[]);

    f.node(
        "10ed:datasheet_model:WB:1",
        "Model",
        "Warboss",
        "Warboss",
        &[
            ("M", "6\""),
            ("T", "5"),
            ("Sv", "4+"),
            ("inv_sv", "5"),
            ("inv_sv_descr", "* Against ranged attacks only"),
            ("W", "6"),
            ("Ld", "6+"),
            ("OC", "1"),
        ],
    );
    f.edge("DATASHEET_HAS_MODEL", "10ed:datasheet:WB", "10ed:datasheet_model:WB:1", &[]);
    f.node(
        "10ed:datasheet_model:BOY:1",
        "Model",
        "Boy",
        "Boy",
        &[
            ("M", "6\""),
            ("T", "5"),
            ("Sv", "5+"),
            ("inv_sv", "-"),
            ("inv_sv_descr", ""),
            ("W", "1"),
            ("Ld", "7+"),
            ("OC", "2"),
        ],
    );
    f.edge("DATASHEET_HAS_MODEL", "10ed:datasheet:BOY", "10ed:datasheet_model:BOY:1", &[]);
    f.node("10ed:datasheet_model_cost:WB:1", "ModelCost", "Cost 1", "1 model\n75", &[]);
    f.edge(
        "DATASHEET_HAS_COST",
        "10ed:datasheet:WB",
        "10ed:datasheet_model_cost:WB:1",
        &[],
    );
    f
}

#[test]
fn a_unit_card_gathers_every_list() {
    let (_dir, bundle) = warboss().write("card");
    let card = bundle.unit("Warboss").unwrap();
    assert_eq!(card.unit.name, "Warboss");
    assert_eq!(card.keywords.len(), 1);
    assert_eq!(card.keywords[0].name, "Infantry");
    assert!(!card.keywords[0].is_faction_keyword);
    assert_eq!(card.models.len(), 1);
    assert_eq!(card.points.len(), 1);
    assert_eq!(card.abilities.len(), 2);
    assert_eq!(card.wargear.len(), 2);
    assert_eq!(names(&card.leads), ["Boyz", "Nobz"]);
    assert!(card.led_by.is_empty());
}

#[test]
fn model_stats_and_points_are_typed() {
    let (_dir, bundle) = warboss().write("stats");
    let model = &bundle.unit_models("Warboss").unwrap()[0];
    assert_eq!(
        (
            model.movement.as_str(),
            model.toughness.as_str(),
            model.save.as_str(),
            model.wounds.as_str(),
            model.leadership.as_str(),
            model.objective_control.as_str()
        ),
        ("6\"", "5", "4+", "6", "6+", "1")
    );
    assert_eq!(model.invulnerable_save.as_deref(), Some("5+"));
    assert_eq!(
        model.invulnerable_save_note.as_deref(),
        Some("Against ranged attacks only")
    );
    let boy = &bundle.unit_models("Boyz").unwrap()[0];
    assert_eq!(boy.invulnerable_save, None);
    assert_eq!(boy.invulnerable_save_note, None);
    let points = &bundle.unit_points("Warboss").unwrap()[0];
    assert_eq!(points.description, "1 model");
    assert_eq!(points.points, Some(75));
}

#[test]
fn abilities_are_scoped_to_the_unit_with_their_own_parameters() {
    let (_dir, bundle) = warboss().write("abilities");
    let abilities = bundle.unit_abilities("Warboss").unwrap();
    let names = abilities.iter().map(|item| item.name.as_str()).collect::<Vec<_>>();
    assert_eq!(names, ["Feel No Pain", "Might is Right"]);
    assert_eq!(abilities[0].scope.as_deref(), Some("Core"));
    assert_eq!(abilities[0].parameter.as_deref(), Some("5+"));
    assert_eq!(
        abilities[0].text,
        "Each time this model would lose a wound, roll one D6."
    );
    assert_eq!(abilities[1].parameter, None);
    assert_eq!(
        abilities[1].text,
        "While leading a unit, add 1 to the Hit roll."
    );
    // Boyz' own Feel No Pain is theirs alone.
    let boyz = bundle.unit_abilities("Boyz").unwrap();
    assert_eq!(boyz.len(), 1);
    assert_eq!(boyz[0].parameter.as_deref(), Some("6+"));
    assert_eq!(boyz[0].model.as_deref(), Some("Boy"));
}

#[test]
fn strike_and_sweep_are_separate_profiles_with_stats() {
    let (_dir, bundle) = warboss().write("wargear");
    let wargear = bundle.unit_wargear("Warboss").unwrap();
    assert_eq!(wargear.len(), 2);
    assert_eq!(wargear[0].name, "Power Klaw - strike");
    let strike = wargear[0].stats.as_ref().unwrap();
    assert_eq!((strike.attacks.as_str(), strike.strength.as_str(), strike.ap.as_str(), strike.damage.as_str()), ("6", "14", "-3", "4"));
    assert!(wargear[0].keywords.is_empty());
    assert_eq!(wargear[1].name, "Power Klaw - sweep");
    assert_eq!(wargear[1].stats.as_ref().unwrap().attacks, "12");
    assert_eq!(wargear[1].keywords, ["sustained hits 1", "lance"]);
}

#[test]
fn lead_direction_follows_the_edge() {
    let (_dir, bundle) = warboss().write("leads");
    assert_eq!(names(&bundle.unit_leads("Warboss").unwrap()), ["Boyz", "Nobz"]);
    assert!(bundle.unit_led_by("Warboss").unwrap().is_empty());
    assert_eq!(names(&bundle.unit_leads("Nobz").unwrap()), ["Boyz"]);
    assert_eq!(names(&bundle.unit_led_by("Nobz").unwrap()), ["Warboss"]);
    assert_eq!(names(&bundle.unit_led_by("Boyz").unwrap()), ["Nobz", "Warboss"]);
    assert!(bundle.unit_leads("Boyz").unwrap().is_empty());
}

#[test]
fn units_are_found_by_ability_and_by_keyword() {
    let (_dir, bundle) = warboss().write("holders");
    let holders = bundle.units_with_ability("feel no pain").unwrap();
    let rows = holders
        .iter()
        .map(|item| (item.unit.name.as_str(), item.parameter.as_deref()))
        .collect::<Vec<_>>();
    assert_eq!(rows, [("Warboss", Some("5+")), ("Boyz", Some("6+"))]);
    assert_eq!(names(&bundle.units_with_keyword("Infantry").unwrap()), ["Warboss"]);
    assert!(matches!(
        bundle.units_with_ability("Teleport Homer"),
        Err(AskError::NotFound { .. })
    ));
}

/// A detachment with a stratagem, an enhancement, and a rule.
fn detachment() -> Fixture {
    let mut f = Fixture::default();
    f.faction("10ed:faction:AC", "Adeptus Custodes");
    f.node("10ed:detachment:SH", "Detachment", "Shield Host", "Shield Host", &[]);
    f.edge("FACTION_HAS_DETACHMENT", "10ed:faction:AC", "10ed:detachment:SH", &[]);
    f.node(
        "10ed:stratagem:S1",
        "Stratagem",
        "ARCANE GUARD",
        "ARCANE GUARD\nBattle Tactic Stratagem\n1\nEither player's turn\nWHEN: Any phase.",
        &[],
    );
    f.edge("DETACHMENT_HAS_STRATAGEM", "10ed:detachment:SH", "10ed:stratagem:S1", &[]);
    f.edge("FACTION_HAS_STRATAGEM", "10ed:faction:AC", "10ed:stratagem:S1", &[]);
    f.node(
        "10ed:enhancement:E1",
        "Enhancement",
        "Auric Mantle",
        "Auric Mantle\n20\nBearer has +1 Toughness.",
        &[],
    );
    f.edge("DETACHMENT_HAS_ENHANCEMENT", "10ed:detachment:SH", "10ed:enhancement:E1", &[]);
    f.edge("FACTION_HAS_ENHANCEMENT", "10ed:faction:AC", "10ed:enhancement:E1", &[]);
    f.node(
        "10ed:detachment_ability:R1",
        "DetachmentAbility",
        "Martial Mastery",
        "Martial Mastery\nShield Host\nPick a bonus each battle round.",
        &[],
    );
    f.edge("DETACHMENT_HAS_ABILITY", "10ed:detachment:SH", "10ed:detachment_ability:R1", &[]);
    f
}

#[test]
fn a_detachment_lists_its_stratagems_enhancements_and_rules() {
    let (_dir, bundle) = detachment().write("detachment");
    let detachments = bundle.detachments("Adeptus Custodes").unwrap();
    assert_eq!(detachments.len(), 1);
    assert_eq!(detachments[0].name, "Shield Host");

    let stratagems = bundle.detachment_stratagems("Shield Host").unwrap();
    assert_eq!(stratagems.len(), 1);
    assert_eq!(stratagems[0].name, "ARCANE GUARD");
    assert!(stratagems[0].text.starts_with("Battle Tactic Stratagem"));

    let enhancements = bundle.detachment_enhancements("shield host").unwrap();
    assert_eq!(enhancements[0].name, "Auric Mantle");
    assert_eq!(bundle.faction_enhancements("Adeptus Custodes").unwrap().len(), 1);

    let rules = bundle.detachment_rules("Shield Host").unwrap();
    assert_eq!(rules[0].text, "Shield Host\nPick a bonus each battle round.");
}

#[test]
fn search_ranks_exact_titles_first_and_filters_by_kind() {
    let (_dir, bundle) = warboss().write("search");
    let hits = bundle.search("warboss", &[], 10);
    assert_eq!(hits[0].node.name, "Warboss");
    let datasheets = bundle.search("warboss", &["Datasheet"], 10);
    assert_eq!(datasheets.len(), 1);
    assert_eq!(datasheets[0].node.kind, "Datasheet");
    let by_text = bundle.search("roll one D6", &["Ability"], 10);
    assert_eq!(by_text[0].node.name, "Feel No Pain");
    assert!(bundle.search("zzzz", &[], 10).is_empty());
    assert!(bundle.search("  ", &[], 10).is_empty());
}

#[test]
fn neighbors_report_direction_and_check_edge_kinds() {
    let (_dir, bundle) = warboss().write("neighbors");
    let page = bundle
        .neighbors("10ed:datasheet:WB", &["DATASHEET_CAN_LEAD"], 10)
        .unwrap();
    assert_eq!(page.total, 2);
    assert!(page.items.iter().all(|item| item.direction == Direction::Outgoing));
    let incoming = bundle
        .neighbors("10ed:datasheet:BOY", &["DATASHEET_CAN_LEAD"], 10)
        .unwrap();
    assert_eq!(incoming.total, 2);
    assert!(incoming.items.iter().all(|item| item.direction == Direction::Incoming));

    let capped = bundle.neighbors("10ed:datasheet:WB", &[], 1).unwrap();
    assert_eq!(capped.items.len(), 1);
    assert!(capped.total > 1);

    assert!(matches!(
        bundle.neighbors("10ed:datasheet:WB", &["NOT_A_KIND"], 10),
        Err(AskError::NotFound { .. })
    ));
    assert!(matches!(
        bundle.neighbors("10ed:datasheet:NOPE", &[], 10),
        Err(AskError::NotFound { .. })
    ));
}

#[test]
fn a_subgraph_walks_depth_and_includes_edges_between_reached_nodes() {
    let (_dir, bundle) = warboss().write("subgraph");
    let seeds = ["10ed:datasheet:WB"];
    let one = bundle.subgraph(&seeds, &["DATASHEET_CAN_LEAD"], 1).unwrap();
    let ids = one.nodes.iter().map(|node| node.id.as_str()).collect::<Vec<_>>();
    assert_eq!(ids, ["10ed:datasheet:WB", "10ed:datasheet:BOY", "10ed:datasheet:NOB"]);
    // Nobz leads Boyz: both ends are reached, so that edge is included.
    assert_eq!(one.edges.len(), 3);
    assert!(!one.truncated);
    assert!(matches!(
        bundle.subgraph(&[], &[], 1),
        Err(AskError::Invalid(_))
    ));
}

#[test]
fn node_detail_returns_text_and_attrs() {
    let (_dir, bundle) = warboss().write("detail");
    let detail = bundle.node_detail("10ed:datasheet_model:WB:1").unwrap();
    assert_eq!(detail.kind, "Model");
    assert_eq!(detail.attrs.text("W"), Some("6"));
    assert!(matches!(
        bundle.node_detail("10ed:nope"),
        Err(AskError::NotFound { .. })
    ));
}

#[test]
fn results_serialize_to_json() {
    let (_dir, bundle) = warboss().write("json");
    let card = bundle.unit("Warboss").unwrap();
    let json = serde_json::to_value(&card).unwrap();
    assert_eq!(json["unit"]["name"], "Warboss");
    assert_eq!(json["points"][0]["points"], 75);
    assert_eq!(json["wargear"][0]["stats"]["kind"], "Melee");
    let page = bundle
        .neighbors("10ed:datasheet:WB", &["DATASHEET_CAN_LEAD"], 5)
        .unwrap();
    assert_eq!(serde_json::to_value(&page).unwrap()["items"][0]["direction"], "outgoing");
}

/// The Warboss fixture plus the rules content around it: the datasheet's own text,
/// composition and options, two enhancements named alike in different detachments,
/// a detachment rule, and a faction ability and stratagem.
fn orks_rules() -> Fixture {
    let mut f = warboss();
    let wb = f.nodes.iter_mut().find(|n| n.id == "10ed:datasheet:WB").unwrap();
    wb.text = "Warboss\nBattleline\nThis model is equipped with: power klaw.\nThis model has a transport capacity of 12 ORKS INFANTRY models.\n1-3\nWhile this model has 1-3 wounds remaining, subtract 1 from its Hit rolls.".to_string();

    f.node("10ed:datasheet_unit_composition:WB:1", "UnitComposition", "Unit composition 1", "1 Warboss", &[]);
    f.edge("DATASHEET_HAS_COMPOSITION", "10ed:datasheet:WB", "10ed:datasheet_unit_composition:WB:1", &[("line", "1")]);
    f.node("10ed:datasheet_option:WB:1", "Option", "Option 1", "\u{2022}\nThis model\u{2019}s big choppa can be replaced with 1 power klaw.", &[]);
    f.node("10ed:datasheet_option:WB:2", "Option", "Option 2", "\u{2022}\nThis model can be equipped with 1 attack squig.", &[]);
    f.edge("DATASHEET_HAS_OPTION", "10ed:datasheet:WB", "10ed:datasheet_option:WB:1", &[("line", "1")]);
    f.edge("DATASHEET_HAS_OPTION", "10ed:datasheet:WB", "10ed:datasheet_option:WB:2", &[("line", "2")]);

    f.node("10ed:detachment:WAAAGH", "Detachment", "War Horde", "War Horde", &[]);
    f.node("10ed:detachment:KULT", "Detachment", "Kult of Speed", "Kult of Speed", &[]);
    f.edge("FACTION_HAS_DETACHMENT", "10ed:faction:ORK", "10ed:detachment:WAAAGH", &[]);
    f.edge("FACTION_HAS_DETACHMENT", "10ed:faction:ORK", "10ed:detachment:KULT", &[]);
    for (id, detachment, label) in [
        ("10ed:enhancement:E1", "10ed:detachment:WAAAGH", "Brutal Fist"),
        ("10ed:enhancement:E2", "10ed:detachment:KULT", "Brutal Fist"),
    ] {
        f.node(id, "Enhancement", label, &format!("{label}\n15\nBearer hits harder."), &[]);
        f.edge("DETACHMENT_HAS_ENHANCEMENT", detachment, id, &[]);
        f.edge("FACTION_HAS_ENHANCEMENT", "10ed:faction:ORK", id, &[]);
    }
    f.edge("ENHANCEMENT_APPLIES_TO_DATASHEET", "10ed:enhancement:E1", "10ed:datasheet:WB", &[]);
    f.edge("ENHANCEMENT_APPLIES_TO_DATASHEET", "10ed:enhancement:E1", "10ed:datasheet:NOB", &[]);
    f.edge("ENHANCEMENT_APPLIES_TO_DATASHEET", "10ed:enhancement:E2", "10ed:datasheet:WB", &[]);

    f.node("10ed:detachment_ability:R1", "DetachmentAbility", "Get Stuck In", "Get Stuck In\nWar Horde\nMelee weapons gain Sustained Hits 1.", &[]);
    f.edge("DETACHMENT_HAS_ABILITY", "10ed:detachment:WAAAGH", "10ed:detachment_ability:R1", &[]);
    f.edge("DATASHEET_HAS_DETACHMENT_ABILITY", "10ed:datasheet:WB", "10ed:detachment_ability:R1", &[]);
    f.edge("DATASHEET_HAS_DETACHMENT_ABILITY", "10ed:datasheet:BOY", "10ed:detachment_ability:R1", &[]);

    f.node("10ed:ability:WAAAGH", "Ability", "Waaagh!", "Waaagh!\nThe infamous war cry of the Orks.\nOnce per battle, call a Waaagh!", &[]);
    f.edge("FACTION_HAS_ABILITY", "10ed:faction:ORK", "10ed:ability:WAAAGH", &[]);
    f.node("10ed:stratagem:CORE1", "Stratagem", "ARMOUR OF CONTEMPT", "ARMOUR OF CONTEMPT\nCore Stratagem\n1\nWHEN: Any phase.", &[]);
    f.edge("FACTION_HAS_STRATAGEM", "10ed:faction:ORK", "10ed:stratagem:CORE1", &[]);
    f
}

#[test]
fn a_datasheets_own_text_keeps_transport_and_the_damaged_profile() {
    let (_dir, bundle) = orks_rules().write("unit-text");
    let text = bundle.unit_text("Warboss").unwrap();
    assert!(text.starts_with("This model is equipped with: power klaw."), "{text}");
    assert!(text.contains("transport capacity of 12 ORKS INFANTRY"));
    assert!(text.contains("1-3\nWhile this model has 1-3 wounds remaining"));
    assert!(!text.contains("Battleline"), "the role line is UnitRef.role, not text");
    assert_eq!(bundle.unit("Warboss").unwrap().text, text);
    // A datasheet with only a name and role has no text of its own.
    assert_eq!(bundle.unit_text("Nobz").unwrap(), "");
}

#[test]
fn composition_and_options_are_typed_and_lose_their_bullet() {
    let (_dir, bundle) = orks_rules().write("composition");
    let composition = bundle.unit_composition("Warboss").unwrap();
    assert_eq!(composition.len(), 1);
    assert_eq!(composition[0].text, "1 Warboss");
    let options = bundle.unit_options("Warboss").unwrap();
    let texts = options.iter().map(|item| item.text.as_str()).collect::<Vec<_>>();
    assert_eq!(
        texts,
        [
            "This model\u{2019}s big choppa can be replaced with 1 power klaw.",
            "This model can be equipped with 1 attack squig."
        ]
    );
    let card = bundle.unit("Warboss").unwrap();
    assert_eq!(card.composition, composition);
    assert_eq!(card.options, options);
    assert!(bundle.unit_options("Boyz").unwrap().is_empty());
}

#[test]
fn enhancements_are_found_from_the_unit_and_from_the_enhancement() {
    let (_dir, bundle) = orks_rules().write("enhancements");
    let for_boss = bundle.unit_enhancements("Warboss").unwrap();
    assert_eq!(for_boss.len(), 2);
    assert!(for_boss.iter().all(|item| item.name == "Brutal Fist"));
    assert_eq!(for_boss[0].text, "15\nBearer hits harder.");
    assert_eq!(bundle.unit_enhancements("Nobz").unwrap().len(), 1);
    assert!(bundle.unit_enhancements("Boyz").unwrap().is_empty());

    assert_eq!(
        names(&bundle.enhancement_units("10ed:enhancement:E1").unwrap()),
        ["Warboss", "Nobz"]
    );
    assert_eq!(
        names(&bundle.enhancement_units("10ed:enhancement:E2").unwrap()),
        ["Warboss"]
    );
}

#[test]
fn a_shared_enhancement_name_is_ambiguous_and_names_each_detachment() {
    let (_dir, bundle) = orks_rules().write("enhancement-ambiguous");
    match bundle.enhancement_units("Brutal Fist") {
        Err(AskError::Ambiguous { candidates, .. }) => {
            let details = candidates.iter().map(|item| item.detail.as_str()).collect::<Vec<_>>();
            assert_eq!(details, ["War Horde", "Kult of Speed"]);
        }
        other => panic!("expected Ambiguous, got {other:?}"),
    }
}

#[test]
fn a_detachment_rule_and_the_units_it_names_are_linked_both_ways() {
    let (_dir, bundle) = orks_rules().write("rule-units");
    assert_eq!(
        names(&bundle.detachment_rule_units("Get Stuck In").unwrap()),
        ["Warboss", "Boyz"]
    );
    let rules = bundle.unit_detachment_rules("Boyz").unwrap();
    assert_eq!(rules.len(), 1);
    assert_eq!(rules[0].name, "Get Stuck In");
    assert!(bundle.unit_detachment_rules("Nobz").unwrap().is_empty());
}

#[test]
fn a_faction_has_its_own_abilities_and_stratagems() {
    let (_dir, bundle) = orks_rules().write("faction-rules");
    let abilities = bundle.faction_abilities("Orks").unwrap();
    assert_eq!(abilities.len(), 1);
    assert_eq!(abilities[0].name, "Waaagh!");
    // The legend is content, so it stays; only the name line is dropped.
    assert_eq!(
        abilities[0].text,
        "The infamous war cry of the Orks.\nOnce per battle, call a Waaagh!"
    );
    let stratagems = bundle.faction_stratagems("Orks").unwrap();
    assert_eq!(stratagems[0].name, "ARMOUR OF CONTEMPT");
    assert_eq!(bundle.faction_enhancements("Orks").unwrap().len(), 2);
}

#[test]
fn a_shared_detachment_name_names_its_faction() {
    let mut f = armies();
    f.node("10ed:detachment:A", "Detachment", "Combined Arms", "Combined Arms", &[]);
    f.node("10ed:detachment:B", "Detachment", "Combined Arms", "Combined Arms", &[]);
    // The same name twice in one faction: a Boarding Actions variant.
    f.node(
        "10ed:detachment:C",
        "Detachment",
        "Combined Arms",
        "Combined Arms\nBoarding Actions",
        &[("type", "Boarding Actions")],
    );
    f.edge("FACTION_HAS_DETACHMENT", "10ed:faction:SM", "10ed:detachment:A", &[]);
    f.edge("FACTION_HAS_DETACHMENT", "10ed:faction:CD", "10ed:detachment:B", &[]);
    f.edge("FACTION_HAS_DETACHMENT", "10ed:faction:SM", "10ed:detachment:C", &[]);
    let (_dir, bundle) = f.write("detachment-ambiguous");
    match bundle.detachment_stratagems("Combined Arms") {
        Err(AskError::Ambiguous { candidates, .. }) => {
            let details = candidates.iter().map(|item| item.detail.as_str()).collect::<Vec<_>>();
            assert_eq!(
                details,
                ["Space Marines", "Chaos Daemons", "Space Marines (Boarding Actions)"]
            );
        }
        other => panic!("expected Ambiguous, got {other:?}"),
    }
}

/// A server holds one open bundle and answers many requests from it.
#[test]
fn a_bundle_can_be_shared_between_threads() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<Bundle>();
}

#[test]
fn opening_a_missing_or_stale_bundle_is_a_bundle_error() {
    assert!(matches!(
        Bundle::open(Path::new("definitely-not-a-bundle")),
        Err(AskError::Bundle(_))
    ));
}
