//! The server as an agent meets it: the real binary, spoken to in JSON-RPC over
//! stdio, against a small hand-built bundle.

use std::collections::BTreeMap;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::sync::OnceLock;
use std::thread;
use std::time::{Duration, Instant};

use serde_json::{json, Value};
use wh_graph::{
    write_graph_db, Attrs, BundleManifest, EdgeRecord, GraphNode, Passage, FORMAT_VERSION,
    GRAPH_DB,
};

const BIN: &str = env!("CARGO_BIN_EXE_wh-mcp");

// ---------------------------------------------------------------- fixture

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
            let flag = if *is_faction { "true" } else { "false" };
            self.edge("DATASHEET_HAS_KEYWORD", id, keyword, &[("is_faction_keyword", flag)]);
        }
    }

    fn write(self, dir: &Path) {
        let _ = fs::remove_dir_all(dir);
        fs::create_dir_all(dir).unwrap();
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
            node_counts_by_kind: counts(self.nodes.iter().map(|n| n.kind.as_str())),
            edge_counts_by_kind: counts(self.edges.iter().map(|e| e.kind.as_str())),
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
        fs::write(dir.join("manifest.json"), serde_json::to_string(&manifest).unwrap()).unwrap();
    }
}

fn counts<'a>(kinds: impl Iterator<Item = &'a str>) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for kind in kinds {
        *counts.entry(kind.to_string()).or_insert(0) += 1;
    }
    counts
}

/// Space Marines with a chapter, Orks with a Warboss and a detachment, and a
/// large faction used to test the reply caps. Built once, then only read.
fn bundle_dir() -> &'static Path {
    static DIR: OnceLock<PathBuf> = OnceLock::new();
    DIR.get_or_init(|| {
        let dir = std::env::temp_dir().join(format!("wh-mcp-fixture-{}", std::process::id()));
        world().write(&dir);
        dir
    })
}

fn world() -> Fixture {
    let mut f = Fixture::default();
    f.node("10ed:faction:SM", "Faction", "Space Marines", "Space Marines", &[]);
    f.node("10ed:faction:ORK", "Faction", "Orks", "Orks", &[]);
    f.node("10ed:faction:CROWD", "Faction", "Crowd Army", "Crowd Army", &[]);
    f.node("10ed:keyword:astartes", "Keyword", "Adeptus Astartes", "Adeptus Astartes", &[]);
    f.node("10ed:keyword:ultramarines", "Keyword", "Ultramarines", "Ultramarines", &[]);
    f.node("10ed:keyword:infantry", "Keyword", "Infantry", "Infantry", &[]);
    f.node("10ed:keyword:crowd", "Keyword", "Crowd", "Crowd", &[]);

    let astartes = ("10ed:keyword:astartes", true);
    f.datasheet("10ed:datasheet:INT", "Intercessor Squad", "10ed:faction:SM", &[astartes]);
    f.datasheet(
        "10ed:datasheet:UMH",
        "Ultramarines Hero",
        "10ed:faction:SM",
        &[astartes, ("10ed:keyword:ultramarines", true)],
    );
    f.datasheet("10ed:datasheet:DRN1", "Dreadnought", "10ed:faction:SM", &[]);
    f.datasheet("10ed:datasheet:DRN2", "Dreadnought", "10ed:faction:ORK", &[]);
    f.datasheet(
        "10ed:datasheet:WB",
        "Warboss",
        "10ed:faction:ORK",
        &[("10ed:keyword:infantry", false)],
    );
    f.datasheet("10ed:datasheet:BOY", "Boyz", "10ed:faction:ORK", &[]);
    f.datasheet("10ed:datasheet:NOB", "Nobz", "10ed:faction:ORK", &[]);
    for index in 0..350 {
        let id = format!("10ed:datasheet:C{index:03}");
        f.datasheet(&id, &format!("Crowd Unit {index:03}"), "10ed:faction:CROWD", &[("10ed:keyword:crowd", false)]);
    }

    // The Warboss's datasheet text, as the export prints it.
    let wb = f.nodes.iter_mut().find(|n| n.id == "10ed:datasheet:WB").unwrap();
    wb.text = "Warboss\nBattleline\nThis model is equipped with: power klaw.\nThis model has a transport capacity of 12 ORKS INFANTRY models.\n1-3\nWhile this model has 1-3 wounds remaining, subtract 1 from its Hit rolls.".to_string();

    f.edge("DATASHEET_CAN_LEAD", "10ed:datasheet:WB", "10ed:datasheet:BOY", &[]);
    f.edge("DATASHEET_CAN_LEAD", "10ed:datasheet:WB", "10ed:datasheet:NOB", &[]);
    f.edge("DATASHEET_CAN_LEAD", "10ed:datasheet:NOB", "10ed:datasheet:BOY", &[]);

    f.node("10ed:ability:FNP", "Ability", "Feel No Pain", "Feel No Pain\nA legend line.\nRoll one D6 per wound.", &[]);
    f.edge("DATASHEET_HAS_ABILITY", "10ed:datasheet:WB", "10ed:ability:FNP", &[("type", "Core"), ("parameter", "5+"), ("model", "")]);
    f.edge("DATASHEET_HAS_ABILITY", "10ed:datasheet:BOY", "10ed:ability:FNP", &[("type", "Core"), ("parameter", "6+"), ("model", "")]);
    f.node("10ed:datasheet_ability:WB:3", "Ability", "Might is Right", "Might is Right\nDatasheet\nAdd 1 to the Hit roll.", &[]);
    f.edge("DATASHEET_HAS_ABILITY", "10ed:datasheet:WB", "10ed:datasheet_ability:WB:3", &[("type", "Datasheet"), ("parameter", ""), ("model", "")]);

    f.node("10ed:wargear:WB:2", "Wargear", "Power Klaw - strike", "Power Klaw - strike\nMelee Melee 6 2 14 -3 4\nlance", &[]);
    f.edge("DATASHEET_HAS_WARGEAR", "10ed:datasheet:WB", "10ed:wargear:WB:2", &[]);
    f.node(
        "10ed:datasheet_model:WB:1",
        "Model",
        "Warboss",
        "Warboss",
        &[("M", "6\""), ("T", "5"), ("Sv", "4+"), ("inv_sv", "5"), ("inv_sv_descr", "* Against ranged attacks only"), ("W", "6"), ("Ld", "6+"), ("OC", "1")],
    );
    f.edge("DATASHEET_HAS_MODEL", "10ed:datasheet:WB", "10ed:datasheet_model:WB:1", &[]);
    f.node("10ed:datasheet_model_cost:WB:1", "ModelCost", "Cost 1", "1 model\n75", &[]);
    f.edge("DATASHEET_HAS_COST", "10ed:datasheet:WB", "10ed:datasheet_model_cost:WB:1", &[]);
    f.node("10ed:datasheet_unit_composition:WB:1", "UnitComposition", "Unit composition 1", "1 Warboss", &[]);
    f.edge("DATASHEET_HAS_COMPOSITION", "10ed:datasheet:WB", "10ed:datasheet_unit_composition:WB:1", &[]);
    f.node("10ed:datasheet_option:WB:1", "Option", "Option 1", "\u{2022}\nThis model can be equipped with 1 attack squig.", &[]);
    f.edge("DATASHEET_HAS_OPTION", "10ed:datasheet:WB", "10ed:datasheet_option:WB:1", &[]);

    f.node("10ed:detachment:WAAAGH", "Detachment", "War Horde", "War Horde", &[]);
    f.node("10ed:detachment:KULT", "Detachment", "Kult of Speed", "Kult of Speed", &[]);
    f.edge("FACTION_HAS_DETACHMENT", "10ed:faction:ORK", "10ed:detachment:WAAAGH", &[]);
    f.edge("FACTION_HAS_DETACHMENT", "10ed:faction:ORK", "10ed:detachment:KULT", &[]);
    f.node("10ed:stratagem:S1", "Stratagem", "ARMOURED DUELLISTS", "ARMOURED DUELLISTS\nBattle Tactic\n1\nWHEN: Fight phase.", &[]);
    f.edge("DETACHMENT_HAS_STRATAGEM", "10ed:detachment:WAAAGH", "10ed:stratagem:S1", &[]);
    f.edge("FACTION_HAS_STRATAGEM", "10ed:faction:ORK", "10ed:stratagem:S1", &[]);
    f.node("10ed:detachment_ability:R1", "DetachmentAbility", "Get Stuck In", "Get Stuck In\nWar Horde\nMelee weapons gain Sustained Hits 1.", &[]);
    f.edge("DETACHMENT_HAS_ABILITY", "10ed:detachment:WAAAGH", "10ed:detachment_ability:R1", &[]);
    f.edge("DATASHEET_HAS_DETACHMENT_ABILITY", "10ed:datasheet:WB", "10ed:detachment_ability:R1", &[]);
    f.edge("DATASHEET_HAS_DETACHMENT_ABILITY", "10ed:datasheet:BOY", "10ed:detachment_ability:R1", &[]);
    for (id, detachment) in [("10ed:enhancement:E1", "10ed:detachment:WAAAGH"), ("10ed:enhancement:E2", "10ed:detachment:KULT")] {
        f.node(id, "Enhancement", "Brutal Fist", "Brutal Fist\n15\nBearer hits harder.", &[]);
        f.edge("DETACHMENT_HAS_ENHANCEMENT", detachment, id, &[]);
        f.edge("FACTION_HAS_ENHANCEMENT", "10ed:faction:ORK", id, &[]);
    }
    f.edge("ENHANCEMENT_APPLIES_TO_DATASHEET", "10ed:enhancement:E1", "10ed:datasheet:WB", &[]);
    f.edge("ENHANCEMENT_APPLIES_TO_DATASHEET", "10ed:enhancement:E1", "10ed:datasheet:NOB", &[]);
    f.edge("ENHANCEMENT_APPLIES_TO_DATASHEET", "10ed:enhancement:E2", "10ed:datasheet:WB", &[]);
    f.node("10ed:ability:WAAAGH", "Ability", "Waaagh!", "Waaagh!\nThe war cry.\nOnce per battle, call a Waaagh!", &[]);
    f.edge("FACTION_HAS_ABILITY", "10ed:faction:ORK", "10ed:ability:WAAAGH", &[]);
    f
}

// ---------------------------------------------------------------- session

/// A running `wh-mcp`, spoken to one JSON line at a time.
struct Session {
    child: Child,
    stdin: Option<ChildStdin>,
    lines: Receiver<String>,
    next_id: u64,
    initialize: Value,
}

/// A tool reply: the text block, whether it is flagged an error, and the JSON in it.
struct Reply {
    is_error: bool,
    json: Value,
}

impl Session {
    fn start() -> Session {
        let mut child = Command::new(BIN)
            .arg("--bundle")
            .arg(bundle_dir())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("start wh-mcp");
        let stdin = child.stdin.take();
        let stdout = child.stdout.take().unwrap();
        let (sender, lines) = mpsc::channel();
        thread::spawn(move || {
            for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                if sender.send(line).is_err() {
                    break;
                }
            }
        });
        let mut session = Session { child, stdin, lines, next_id: 1, initialize: Value::Null };
        session.initialize = session.request(
            "initialize",
            json!({
                "protocolVersion": "2025-06-18",
                "capabilities": {},
                "clientInfo": { "name": "wh-mcp-test", "version": "0" }
            }),
        );
        session.send(&json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }));
        session
    }

    fn send(&mut self, message: &Value) {
        let stdin = self.stdin.as_mut().expect("stdin is open");
        writeln!(stdin, "{message}").unwrap();
        stdin.flush().unwrap();
    }

    /// Send a request and return the whole reply message, `result` or `error`.
    /// Every line the server writes to stdout must be a JSON-RPC message.
    fn exchange(&mut self, method: &str, params: Value) -> Value {
        let id = self.next_id;
        self.next_id += 1;
        self.send(&json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params }));
        loop {
            let line = self
                .lines
                .recv_timeout(Duration::from_secs(30))
                .unwrap_or_else(|_| panic!("no reply to {method} within 30 seconds"));
            let message: Value = serde_json::from_str(&line)
                .unwrap_or_else(|error| panic!("stdout carried a non-JSON line ({error}): {line}"));
            assert_eq!(message["jsonrpc"], "2.0", "not a JSON-RPC message: {line}");
            if message["id"] == json!(id) {
                return message;
            }
        }
    }

    /// Like `exchange`, but a protocol error fails the test.
    fn request(&mut self, method: &str, params: Value) -> Value {
        let message = self.exchange(method, params);
        assert!(message.get("error").is_none(), "{method} failed: {message}");
        message["result"].clone()
    }

    fn call(&mut self, tool: &str, arguments: Value) -> Reply {
        let result = self.request("tools/call", json!({ "name": tool, "arguments": arguments }));
        let text = result["content"][0]["text"].as_str().expect("a text block").to_string();
        Reply {
            is_error: result["isError"].as_bool().unwrap_or(false),
            json: serde_json::from_str(&text).unwrap_or_else(|_| panic!("reply is not JSON: {text}")),
        }
    }

    /// Close stdin, as a client does when it is done, and wait for the exit.
    fn finish(mut self) -> Option<i32> {
        drop(self.stdin.take());
        let started = Instant::now();
        while started.elapsed() < Duration::from_secs(10) {
            if let Some(status) = self.child.try_wait().unwrap() {
                return status.code();
            }
            thread::sleep(Duration::from_millis(50));
        }
        None
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn names(value: &Value) -> Vec<String> {
    value
        .as_array()
        .unwrap_or_else(|| panic!("expected an array, got {value}"))
        .iter()
        .map(|item| item["name"].as_str().unwrap_or_default().to_string())
        .collect()
}

fn keys(value: &Value) -> Vec<String> {
    let mut keys = value.as_object().unwrap().keys().cloned().collect::<Vec<_>>();
    keys.sort();
    keys
}

fn strings(items: &[&str]) -> Vec<String> {
    items.iter().map(|item| item.to_string()).collect()
}

// ---------------------------------------------------------------- tests

#[test]
fn the_handshake_names_the_server_and_tells_the_agent_how_to_use_it() {
    let session = Session::start();
    let init = &session.initialize;
    assert_eq!(init["serverInfo"]["name"], "wh-mcp");
    assert!(init["capabilities"]["tools"].is_object());
    let instructions = init["instructions"].as_str().unwrap();
    for needle in ["get_unit", "get_roster", "get_detachment", "call again with one id", "Wahapedia"] {
        assert!(instructions.contains(needle), "instructions should say: {needle}");
    }
}

#[test]
fn all_fifteen_tools_are_listed_read_only_with_descriptions_and_schemas() {
    let mut session = Session::start();
    let result = session.request("tools/list", json!({}));
    let tools = result["tools"].as_array().unwrap();
    let mut listed = tools.iter().map(|t| t["name"].as_str().unwrap().to_string()).collect::<Vec<_>>();
    listed.sort();
    assert_eq!(
        listed,
        strings(&[
            "bundle_info", "find_units", "get_detachment", "get_faction_rules", "get_neighbors",
            "get_node", "get_roster", "get_subgraph", "get_unit", "list_detachments",
            "list_factions", "search", "units_for_rule", "units_with_ability", "units_with_keyword",
        ])
    );
    for tool in tools {
        let name = tool["name"].as_str().unwrap();
        assert!(tool["description"].as_str().unwrap().len() > 40, "{name} needs a real description");
        assert_eq!(tool["inputSchema"]["type"], "object", "{name} schema");
        assert_eq!(tool["annotations"]["readOnlyHint"], true, "{name} is read-only");
    }
}

#[test]
fn get_unit_returns_the_full_datasheet_by_default() {
    let mut session = Session::start();
    let reply = session.call("get_unit", json!({ "unit": "Warboss" }));
    assert!(!reply.is_error);
    let unit = &reply.json;
    assert_eq!(
        keys(unit),
        strings(&[
            "abilities", "composition", "keywords", "leads", "led_by", "models", "options",
            "points", "text", "unit", "wargear"
        ])
    );
    assert_eq!(unit["unit"]["name"], "Warboss");
    assert_eq!(unit["unit"]["faction"], "Orks");
    assert_eq!(unit["models"][0]["invulnerable_save"], "5+");
    assert_eq!(unit["models"][0]["invulnerable_save_note"], "Against ranged attacks only");
    assert_eq!(unit["points"][0]["points"], 75);
    assert!(unit["text"].as_str().unwrap().contains("transport capacity of 12"));
    assert_eq!(names(&unit["leads"]), ["Boyz", "Nobz"]);
    assert_eq!(unit["wargear"][0]["stats"]["strength"], "14");
    assert_eq!(unit["wargear"][0]["keywords"], json!(["lance"]));
    assert_eq!(unit["abilities"][0]["parameter"], "5+");
    assert_eq!(unit["options"][0]["text"], "This model can be equipped with 1 attack squig.");
}

#[test]
fn sections_pick_parts_and_unlock_the_opt_in_ones() {
    let mut session = Session::start();
    let some = session.call("get_unit", json!({ "unit": "Warboss", "sections": ["models", "enhancements"] }));
    assert_eq!(keys(&some.json), strings(&["enhancements", "models", "unit"]));
    assert_eq!(some.json["enhancements"].as_array().unwrap().len(), 2);

    let rules = session.call("get_unit", json!({ "unit": "Boyz", "sections": ["detachment_rules"] }));
    assert_eq!(names(&rules.json["detachment_rules"]), ["Get Stuck In"]);

    // An empty list means "no preference", not "nothing".
    let empty = session.call("get_unit", json!({ "unit": "Warboss", "sections": [] }));
    assert_eq!(keys(&empty.json).len(), 11);
}

#[test]
fn a_bad_argument_is_a_tool_error_that_lists_the_valid_values() {
    let mut session = Session::start();
    let result = session.request(
        "tools/call",
        json!({ "name": "get_unit", "arguments": { "unit": "Warboss", "sections": ["bogus"] } }),
    );
    assert_eq!(result["isError"], true, "{result}");
    let text = result["content"][0]["text"].as_str().unwrap();
    assert!(text.contains("unknown variant `bogus`"), "{text}");
    for valid in ["models", "wargear", "enhancements"] {
        assert!(text.contains(valid), "the error should list `{valid}`: {text}");
    }

    let missing = session.request("tools/call", json!({ "name": "get_unit", "arguments": {} }));
    assert_eq!(missing["isError"], true, "a missing required argument: {missing}");
}

#[test]
fn a_library_error_is_a_tool_error_the_agent_can_act_on() {
    let mut session = Session::start();

    let typo = session.call("get_unit", json!({ "unit": "Warbos" }));
    assert!(typo.is_error);
    assert!(typo.json["error"].as_str().unwrap().contains("no Datasheet named"));
    assert!(typo.json["suggestions"].as_array().unwrap().contains(&json!("Warboss")));

    let shared = session.call("get_unit", json!({ "unit": "Dreadnought" }));
    assert!(shared.is_error);
    let details = shared.json["candidates"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["detail"].as_str().unwrap().to_string())
        .collect::<Vec<_>>();
    assert_eq!(details, strings(&["Space Marines", "Orks"]));
    // The id the error offers works.
    let by_id = session.call("get_unit", json!({ "unit": "10ed:datasheet:DRN2" }));
    assert!(!by_id.is_error);
    assert_eq!(by_id.json["unit"]["faction"], "Orks");

    let wrong_kind = session.call("get_unit", json!({ "unit": "10ed:faction:SM" }));
    assert!(wrong_kind.is_error);
    assert!(wrong_kind.json["error"].as_str().unwrap().contains("is a Faction"));
}

#[test]
fn rosters_and_factions_and_detachments() {
    let mut session = Session::start();
    let factions = session.call("list_factions", json!({}));
    assert_eq!(names(&factions.json), ["Crowd Army", "Orks", "Space Marines"]);

    let chapter = session.call("get_roster", json!({ "subject": "Ultramarines" }));
    assert_eq!(chapter.json["subject"]["name"], "Ultramarines");
    assert_eq!(names(&chapter.json["units"]), ["Ultramarines Hero", "Intercessor Squad"]);

    let unknown = session.call("get_roster", json!({ "subject": "Squats" }));
    assert!(unknown.is_error);
    assert!(unknown.json["suggestions"].as_array().unwrap().contains(&json!("Space Marines")));

    let detachments = session.call("list_detachments", json!({ "faction": "Orks" }));
    assert_eq!(names(&detachments.json), ["Kult of Speed", "War Horde"]);
}

#[test]
fn detachment_and_faction_rules_default_to_the_cheap_parts() {
    let mut session = Session::start();
    let all = session.call("get_detachment", json!({ "detachment": "War Horde" }));
    assert_eq!(keys(&all.json), strings(&["enhancements", "rules", "stratagems"]));
    assert_eq!(names(&all.json["stratagems"]), ["ARMOURED DUELLISTS"]);
    assert_eq!(all.json["rules"][0]["text"], "War Horde\nMelee weapons gain Sustained Hits 1.");

    let one = session.call("get_detachment", json!({ "detachment": "War Horde", "sections": ["rules"] }));
    assert_eq!(keys(&one.json), strings(&["rules"]));

    let faction = session.call("get_faction_rules", json!({ "faction": "Orks" }));
    assert_eq!(keys(&faction.json), strings(&["abilities"]));
    assert_eq!(faction.json["abilities"][0]["name"], "Waaagh!");

    let long = session.call("get_faction_rules", json!({ "faction": "Orks", "sections": ["stratagems", "enhancements"] }));
    assert_eq!(keys(&long.json), strings(&["enhancements", "stratagems"]));
    assert_eq!(long.json["enhancements"].as_array().unwrap().len(), 2);
}

#[test]
fn units_are_found_by_keyword_ability_and_rule() {
    let mut session = Session::start();
    let keyword = session.call("units_with_keyword", json!({ "keyword": "Infantry" }));
    assert_eq!(keyword.json["total"], 1);
    assert_eq!(keyword.json["truncated"], false);
    assert_eq!(names(&keyword.json["items"]), ["Warboss"]);

    let ability = session.call("units_with_ability", json!({ "ability": "Feel No Pain" }));
    let rows = ability.json["items"].as_array().unwrap();
    assert_eq!(rows[0]["unit"]["name"], "Warboss");
    assert_eq!(rows[0]["parameter"], "5+");
    assert_eq!(rows[1]["parameter"], "6+");

    let rule = session.call("units_for_rule", json!({ "kind": "detachment_rule", "name": "Get Stuck In" }));
    assert_eq!(names(&rule.json), ["Warboss", "Boyz"]);

    let shared = session.call("units_for_rule", json!({ "kind": "enhancement", "name": "Brutal Fist" }));
    assert!(shared.is_error);
    let details = shared.json["candidates"].as_array().unwrap().iter().map(|c| c["detail"].as_str().unwrap().to_string()).collect::<Vec<_>>();
    assert_eq!(details, strings(&["War Horde", "Kult of Speed"]));
    let by_id = session.call("units_for_rule", json!({ "kind": "enhancement", "name": "10ed:enhancement:E1" }));
    assert_eq!(names(&by_id.json), ["Warboss", "Nobz"]);
}

#[test]
fn search_and_find_units_clamp_their_limits() {
    let mut session = Session::start();
    let hits = session.call("search", json!({ "query": "warboss", "kinds": ["Datasheet"] }));
    assert_eq!(hits.json[0]["node"]["name"], "Warboss");
    assert_eq!(hits.json.as_array().unwrap().len(), 1);

    let zero = session.call("search", json!({ "query": "unit", "limit": 0 }));
    assert_eq!(zero.json.as_array().unwrap().len(), 1, "limit 0 is raised to 1");
    let huge = session.call("search", json!({ "query": "unit", "limit": 100000 }));
    assert_eq!(huge.json.as_array().unwrap().len(), 50, "the cap is 50");

    let units = session.call("find_units", json!({ "name": "Dreadnought" }));
    assert_eq!(units.json.as_array().unwrap().len(), 2);
}

#[test]
fn the_graph_tools_walk_edges_with_agent_sized_defaults() {
    let mut session = Session::start();
    let node = session.call("get_node", json!({ "id": "10ed:datasheet_model:WB:1" }));
    assert_eq!(node.json["kind"], "Model");
    assert_eq!(node.json["attrs"]["W"], "6");
    assert!(session.call("get_node", json!({ "id": "10ed:nope" })).is_error);

    let page = session.call("get_neighbors", json!({ "id": "10ed:datasheet:WB", "edge_kinds": ["DATASHEET_CAN_LEAD"] }));
    assert_eq!(page.json["total"], 2);
    assert_eq!(page.json["items"][0]["direction"], "outgoing");
    let capped = session.call("get_neighbors", json!({ "id": "10ed:datasheet:WB", "limit": 0 }));
    assert_eq!(capped.json["items"].as_array().unwrap().len(), 1);
    assert!(capped.json["total"].as_u64().unwrap() > 1);

    let bad = session.call("get_neighbors", json!({ "id": "10ed:datasheet:WB", "edge_kinds": ["NOT_A_KIND"] }));
    assert!(bad.is_error);
    assert!(bad.json["suggestions"].as_array().unwrap().contains(&json!("DATASHEET_CAN_LEAD")));

    let graph = session.call("get_subgraph", json!({ "seeds": ["10ed:datasheet:WB"], "edge_kinds": ["DATASHEET_CAN_LEAD"] }));
    assert_eq!(graph.json["nodes"].as_array().unwrap().len(), 3);
    assert_eq!(graph.json["edges"].as_array().unwrap().len(), 3);
    assert_eq!(graph.json["truncated"], false);
    assert!(session.call("get_subgraph", json!({ "seeds": [] })).is_error);
}

#[test]
fn long_lists_are_cut_to_a_limit_that_says_how_much_was_left_out() {
    let mut session = Session::start();
    // 350 units carry the Crowd keyword. The default limit is 200.
    let cut = session.call("units_with_keyword", json!({ "keyword": "Crowd" }));
    assert_eq!(cut.json["total"], 350);
    assert_eq!(cut.json["truncated"], true);
    assert_eq!(cut.json["items"].as_array().unwrap().len(), 200);

    let all = session.call("units_with_keyword", json!({ "keyword": "Crowd", "limit": 1000 }));
    assert_eq!(all.json["items"].as_array().unwrap().len(), 350);
    assert_eq!(all.json["truncated"], false);

    let few = session.call("units_with_keyword", json!({ "keyword": "Crowd", "limit": 0 }));
    assert_eq!(few.json["items"].as_array().unwrap().len(), 1, "limit 0 is raised to 1");
    assert_eq!(few.json["total"], 350, "the total is always the full count");
}

#[test]
fn a_huge_subgraph_is_cut_to_a_size_a_model_can_read() {
    let mut session = Session::start();
    let crowd = session.call(
        "get_subgraph",
        json!({ "seeds": ["10ed:keyword:crowd"], "edge_kinds": ["DATASHEET_HAS_KEYWORD"], "depth": 9 }),
    );
    assert!(!crowd.is_error);
    assert_eq!(crowd.json["edges"].as_array().unwrap().len(), 100);
    assert!(crowd.json["nodes"].as_array().unwrap().len() <= 150);
    assert_eq!(crowd.json["truncated"], true);
}

#[test]
fn bundle_info_lists_the_valid_kinds() {
    let mut session = Session::start();
    let info = session.call("bundle_info", json!({}));
    assert_eq!(info.json["edition"], "10ed");
    assert!(info.json["edge_counts_by_kind"]["DATASHEET_CAN_LEAD"].as_u64().unwrap() >= 3);
    assert!(info.json["node_counts_by_kind"]["Datasheet"].as_u64().unwrap() >= 7);
}

#[test]
fn closing_stdin_ends_the_server_cleanly() {
    let session = Session::start();
    assert_eq!(session.finish(), Some(0));
}

#[test]
fn the_command_line_reports_problems_on_stderr_with_the_right_exit_code() {
    let none = Command::new(BIN).env_remove("WH_BUNDLE").output().unwrap();
    assert_eq!(none.status.code(), Some(2));
    assert!(none.stdout.is_empty(), "stdout is reserved for the protocol");
    assert!(String::from_utf8_lossy(&none.stderr).contains("no bundle given"));

    let missing = Command::new(BIN).args(["--bundle", "definitely-not-a-bundle"]).output().unwrap();
    assert_eq!(missing.status.code(), Some(3));
    assert!(missing.stdout.is_empty());

    let from_env = Command::new(BIN).env("WH_BUNDLE", "definitely-not-a-bundle").output().unwrap();
    assert_eq!(from_env.status.code(), Some(3), "WH_BUNDLE is read");

    let help = Command::new(BIN).arg("--help").output().unwrap();
    assert_eq!(help.status.code(), Some(0));
    assert!(String::from_utf8_lossy(&help.stdout).contains("--bundle"));
    let version = Command::new(BIN).arg("--version").output().unwrap();
    assert!(String::from_utf8_lossy(&version.stdout).starts_with("wh-mcp "));
}
