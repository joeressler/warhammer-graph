use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Serialize;
use wh_ask::{
    run_query, AskError, ChatModel, EmbedIdentity, Embedder, QueryRequest, Retrieval, QUERY_EXAMPLE,
};
use wh_graph::{Attrs, BundleManifest, EdgeRecord, GraphNode, Passage};

struct ScriptChat {
    prompt: String,
    answer: String,
}

impl ChatModel for ScriptChat {
    fn context_tokens(&self) -> usize {
        1_000_000
    }

    fn count_tokens(&mut self, text: &str) -> Result<usize, AskError> {
        Ok(text.chars().count())
    }

    fn complete(&mut self, prompt: &str) -> Result<String, AskError> {
        self.prompt = prompt.to_string();
        Ok(self.answer.clone())
    }
}

struct FixedEmbed {
    passage_calls: Vec<String>,
}

impl Embedder for FixedEmbed {
    fn embed_passage(&mut self, node_id: &str, _text: &str) -> Result<Vec<f32>, AskError> {
        self.passage_calls.push(node_id.to_string());
        Ok(vector_for(node_id))
    }

    fn embed_query(&mut self, _question: &str) -> Result<Vec<f32>, AskError> {
        Ok(vector_for("10ed:datasheet:EXDS"))
    }

    fn dimension(&self) -> u32 {
        4
    }
}

fn vector_for(node_id: &str) -> Vec<f32> {
    match node_id {
        "10ed:datasheet:EXDS" => vec![1.0, 0.0, 0.0, 0.0],
        "10ed:faction:EX1" => vec![0.0, 1.0, 0.0, 0.0],
        "10ed:keyword:example" => vec![0.0, 0.0, 1.0, 0.0],
        _ => vec![0.0, 0.0, 0.0, 1.0],
    }
}

fn identity() -> EmbedIdentity {
    EmbedIdentity {
        sha256: [7u8; 32],
        len: 3,
        mtime_secs: 9,
    }
}

fn request<'a>(
    bundle: &'a Path,
    config: &'a Path,
    question: &'a str,
    top_k: usize,
) -> QueryRequest<'a> {
    QueryRequest {
        bundle_dir: bundle,
        question,
        top_k,
        context_reserve: 1024,
        config_dir: config,
    }
}

fn node(id: &str, kind: &str, label: &str, text: &str, link: Option<&str>) -> GraphNode {
    GraphNode {
        id: id.to_string(),
        kind: kind.to_string(),
        label: label.to_string(),
        text: text.to_string(),
        attrs: Attrs::default(),
        source_url: link.map(str::to_string),
    }
}

fn edge(id: &str, kind: &str, from: &str, to: &str) -> EdgeRecord {
    EdgeRecord {
        id: id.to_string(),
        kind: kind.to_string(),
        from: from.to_string(),
        to: to.to_string(),
        attrs: Attrs::default(),
    }
}

fn write_bundle(dir: &Path, keywords: usize, distractor: bool) {
    fs::create_dir_all(dir).unwrap();
    let faction = node(
        "10ed:faction:EX1",
        "Faction",
        "Example Faction",
        "Example Faction",
        None,
    );
    let datasheet = node(
        "10ed:datasheet:EXDS",
        "Datasheet",
        "Example Unit",
        "Example Unit fights.",
        Some("https://example.invalid/example-unit"),
    );
    let mut nodes = vec![faction.clone(), datasheet.clone()];
    let mut edges = vec![edge(
        "10ed:edge:FACTION_HAS_DATASHEET:0123456789abcdef",
        "FACTION_HAS_DATASHEET",
        &faction.id,
        &datasheet.id,
    )];
    if keywords == 0 {
        let keyword = node(
            "10ed:keyword:example",
            "Keyword",
            "Example",
            "Example",
            None,
        );
        edges.push(edge(
            "10ed:edge:DATASHEET_HAS_KEYWORD:0123456789abcdef",
            "DATASHEET_HAS_KEYWORD",
            &datasheet.id,
            &keyword.id,
        ));
        nodes.push(keyword);
    } else {
        for index in 0..keywords {
            let id = format!("10ed:keyword:k{index:02}");
            let label = format!("Keyword {index:02}");
            edges.push(edge(
                &format!("10ed:edge:DATASHEET_HAS_KEYWORD:{index:016x}"),
                "DATASHEET_HAS_KEYWORD",
                &datasheet.id,
                &id,
            ));
            nodes.push(node(&id, "Keyword", &label, &label, None));
        }
    }
    if distractor {
        nodes.push(node(
            "10ed:source:OTHER",
            "Source",
            "Other Note",
            "example unit is mentioned",
            None,
        ));
    }
    let passages = nodes
        .iter()
        .map(|item| Passage {
            node_id: item.id.clone(),
            title: item.label.clone(),
            text: item.text.clone(),
            wahapedia_link: item.source_url.clone(),
        })
        .collect::<Vec<_>>();
    let manifest = BundleManifest {
        format_version: 1,
        corpus_schema_version: 1,
        edition: "10ed".to_string(),
        corpus_fingerprint: "ab".repeat(32),
        last_update: "2020-01-01".to_string(),
        node_count: nodes.len(),
        edge_count: edges.len(),
        passage_count: passages.len(),
        node_counts_by_kind: counts(nodes.iter().map(|item| item.kind.as_str())),
        edge_counts_by_kind: counts(edges.iter().map(|item| item.kind.as_str())),
    };
    write_json(&dir.join("manifest.json"), &manifest);
    write_jsonl(&dir.join("nodes.jsonl"), &nodes);
    write_jsonl(&dir.join("edges.jsonl"), &edges);
    write_jsonl(&dir.join("passages.jsonl"), &passages);
}

fn counts<'a>(kinds: impl Iterator<Item = &'a str>) -> std::collections::BTreeMap<String, usize> {
    let mut counts = std::collections::BTreeMap::new();
    for kind in kinds {
        *counts.entry(kind.to_string()).or_insert(0) += 1;
    }
    counts
}

fn write_jsonl<T: Serialize>(path: &Path, rows: &[T]) {
    let mut text = String::new();
    for row in rows {
        text.push_str(&serde_json::to_string(row).unwrap());
        text.push('\n');
    }
    fs::write(path, text).unwrap();
}

fn write_json<T: Serialize>(path: &Path, value: &T) {
    let mut text = serde_json::to_string(value).unwrap();
    text.push('\n');
    fs::write(path, text).unwrap();
}

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("wh-ask-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn node_ids(prompt: &str) -> Vec<String> {
    prompt
        .lines()
        .filter_map(|line| line.strip_prefix("node_id: "))
        .map(str::to_string)
        .collect()
}

#[test]
fn label_search_ranks_example_unit_first() {
    let root = temp_dir("label");
    let bundle = root.join("bundle");
    let config = root.join("config");
    write_bundle(&bundle, 0, true);
    let mut chat = ScriptChat {
        prompt: String::new(),
        answer: "labeled".to_string(),
    };
    let response = run_query(
        &request(&bundle, &config, "example unit", 8),
        &mut chat,
        None,
    )
    .unwrap();
    assert_eq!(response.retrieval, Retrieval::Label);
    assert_eq!(node_ids(&chat.prompt)[0], "10ed:datasheet:EXDS");
    assert!(!config.join("indexes").exists());
    assert!(!bundle.join("graph.postcard").exists());
}

#[test]
fn embedding_expansion_includes_keyword_and_faction() {
    let root = temp_dir("embed");
    let bundle = root.join("bundle");
    let config = root.join("config");
    write_bundle(&bundle, 0, false);
    let mut chat = ScriptChat {
        prompt: String::new(),
        answer: "embedded".to_string(),
    };
    let mut embed = FixedEmbed {
        passage_calls: Vec::new(),
    };
    let response = run_query(
        &request(&bundle, &config, "which unit", 1),
        &mut chat,
        Some((&identity(), &mut embed)),
    )
    .unwrap();
    assert_eq!(response.retrieval, Retrieval::Embedding);
    let ids = node_ids(&chat.prompt);
    assert_eq!(ids[0], "10ed:datasheet:EXDS");
    assert!(ids.iter().any(|id| id == "10ed:keyword:example"));
    assert!(ids.iter().any(|id| id == "10ed:faction:EX1"));
}

#[test]
fn expansion_stops_at_32_nodes_and_keeps_the_seed() {
    let root = temp_dir("cap");
    let bundle = root.join("bundle");
    let config = root.join("config");
    write_bundle(&bundle, 40, false);
    let mut chat = ScriptChat {
        prompt: String::new(),
        answer: "capped".to_string(),
    };
    run_query(
        &request(&bundle, &config, "example unit", 1),
        &mut chat,
        None,
    )
    .unwrap();
    let ids = node_ids(&chat.prompt);
    assert_eq!(ids.len(), 32);
    assert_eq!(ids[0], "10ed:datasheet:EXDS");
    assert!(ids.iter().any(|id| id == "10ed:faction:EX1"));
    assert!(ids.iter().any(|id| id == "10ed:keyword:k00"));
    assert!(ids.iter().any(|id| id == "10ed:keyword:k29"));
    assert!(!ids.iter().any(|id| id == "10ed:keyword:k39"));
}

#[test]
fn citations_drop_ids_that_were_not_in_the_prompt() {
    let root = temp_dir("cite");
    let bundle = root.join("bundle");
    let config = root.join("config");
    write_bundle(&bundle, 0, false);
    let mut chat = ScriptChat {
        prompt: String::new(),
        answer: "Use [10ed:datasheet:EXDS] and [10ed:faction:MISSING] and [10ed:edge:FACTION_HAS_DATASHEET:0123456789abcdef].".to_string(),
    };
    let response = run_query(
        &request(&bundle, &config, "example unit", 1),
        &mut chat,
        None,
    )
    .unwrap();
    assert_eq!(response.citations.len(), 1);
    assert_eq!(response.citations[0].node_id, "10ed:datasheet:EXDS");
    assert_eq!(response.answer, chat.answer);
    assert!(response.citations[0]
        .neighbors
        .iter()
        .any(|label| label == "Example Faction" || label == "Example"));
}

#[test]
fn citations_fall_back_to_seeds_when_the_model_cites_nothing() {
    let root = temp_dir("nocite");
    let bundle = root.join("bundle");
    let config = root.join("config");
    write_bundle(&bundle, 0, false);
    let mut chat = ScriptChat {
        prompt: String::new(),
        answer: "The passages do not contain the answer.".to_string(),
    };
    let response = run_query(
        &request(&bundle, &config, "example unit", 1),
        &mut chat,
        None,
    )
    .unwrap();
    assert_eq!(
        response
            .citations
            .iter()
            .map(|citation| citation.node_id.as_str())
            .collect::<Vec<_>>(),
        vec!["10ed:datasheet:EXDS"]
    );
    assert_eq!(
        response.citations[0].wahapedia_link.as_deref(),
        Some("https://example.invalid/example-unit")
    );
}

#[test]
fn index_reuses_matching_passage_hash_and_rebuilds_when_it_changes() {
    let root = temp_dir("index");
    let bundle = root.join("bundle");
    let config = root.join("config");
    write_bundle(&bundle, 0, false);
    let mut embed = FixedEmbed {
        passage_calls: Vec::new(),
    };
    let mut chat = ScriptChat {
        prompt: String::new(),
        answer: "once".to_string(),
    };
    let id = identity();
    run_query(
        &request(&bundle, &config, "which unit", 1),
        &mut chat,
        Some((&id, &mut embed)),
    )
    .unwrap();
    let built = embed.passage_calls.len();
    assert!(built > 0);
    let index = find_index(&config);
    let bytes = fs::read(&index).unwrap();
    assert!(bytes.starts_with(b"WHASK1"));
    run_query(
        &request(&bundle, &config, "which unit", 1),
        &mut chat,
        Some((&id, &mut embed)),
    )
    .unwrap();
    assert_eq!(embed.passage_calls.len(), built);
    let mut passages = fs::read_to_string(bundle.join("passages.jsonl")).unwrap();
    passages = passages.replace("Example Unit fights.", "Example Unit fights!");
    fs::write(bundle.join("passages.jsonl"), passages).unwrap();
    run_query(
        &request(&bundle, &config, "which unit", 1),
        &mut chat,
        Some((&id, &mut embed)),
    )
    .unwrap();
    assert!(embed.passage_calls.len() > built);
}

#[test]
fn corrupt_postcard_is_ignored() {
    let root = temp_dir("postcard");
    let bundle = root.join("bundle");
    let config = root.join("config");
    write_bundle(&bundle, 0, false);
    fs::write(bundle.join("graph.postcard"), b"not a postcard").unwrap();
    let mut chat = ScriptChat {
        prompt: String::new(),
        answer: "still loaded".to_string(),
    };
    let response = run_query(
        &request(&bundle, &config, "example unit", 1),
        &mut chat,
        None,
    )
    .unwrap();
    assert_eq!(response.answer, "still loaded");
    assert_eq!(node_ids(&chat.prompt)[0], "10ed:datasheet:EXDS");
}

#[test]
fn format_version_two_exits_3() {
    let root = temp_dir("version");
    let bundle = root.join("bundle");
    fs::create_dir_all(&bundle).unwrap();
    let manifest = BundleManifest {
        format_version: 2,
        corpus_schema_version: 1,
        edition: "10ed".to_string(),
        corpus_fingerprint: "ab".repeat(32),
        last_update: "2020-01-01".to_string(),
        node_count: 0,
        edge_count: 0,
        passage_count: 0,
        node_counts_by_kind: std::collections::BTreeMap::new(),
        edge_counts_by_kind: std::collections::BTreeMap::new(),
    };
    write_json(&bundle.join("manifest.json"), &manifest);
    let Some(bin) = option_env!("CARGO_BIN_EXE_wh_ask") else {
        return;
    };
    let output = Command::new(bin)
        .args([
            "query",
            "--bundle",
            bundle.to_str().unwrap(),
            "--chat-model",
            "missing.gguf",
            "--question",
            "example",
        ])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(3));
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("format_version"));
}

#[test]
fn top_k_out_of_range_exits_2_with_a_working_command() {
    let Some(bin) = option_env!("CARGO_BIN_EXE_wh_ask") else {
        return;
    };
    let output = Command::new(bin)
        .args([
            "query",
            "--bundle",
            "./bundle",
            "--chat-model",
            "./models/chat.gguf",
            "--question",
            "example",
            "--top-k",
            "0",
        ])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains(QUERY_EXAMPLE));
}

#[test]
fn help_includes_the_query_example() {
    let Some(bin) = option_env!("CARGO_BIN_EXE_wh_ask") else {
        return;
    };
    let output = Command::new(bin).arg("--help").output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Which example units are in Example Faction?"));
    let query_help = Command::new(bin)
        .args(["query", "--help"])
        .output()
        .unwrap();
    let stdout = String::from_utf8(query_help.stdout).unwrap();
    assert!(stdout.contains("--top-k"));
    assert!(stdout.contains("Which example units are in Example Faction?"));
}

fn find_index(config: &Path) -> PathBuf {
    let indexes = config.join("indexes");
    for passage in fs::read_dir(&indexes).unwrap() {
        let passage = passage.unwrap().path();
        for model in fs::read_dir(&passage).unwrap() {
            let candidate = model.unwrap().path().join("index.bin");
            if candidate.is_file() {
                return candidate;
            }
        }
    }
    panic!("index.bin was not written under {}", indexes.display());
}
