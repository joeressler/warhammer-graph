use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_wh-graph"))
}

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn copy_dir(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let dest = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_dir(&entry.path(), &dest);
        } else {
            fs::copy(entry.path(), dest).unwrap();
        }
    }
}

fn run(args: &[&str]) -> std::process::Output {
    let output = bin().args(args).output().unwrap();
    output
}

fn read_jsonl(path: &Path) -> Vec<Value> {
    let text = fs::read_to_string(path).unwrap();
    text.lines()
        .filter(|line| !line.is_empty())
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

fn nodes(bundle: &Path) -> Vec<Value> {
    read_jsonl(&bundle.join("nodes.jsonl"))
}

fn edges(bundle: &Path) -> Vec<Value> {
    read_jsonl(&bundle.join("edges.jsonl"))
}

fn kind<'a>(rows: &'a [Value], kind: &str) -> Vec<&'a Value> {
    rows.iter().filter(|row| row["kind"] == kind).collect()
}

#[test]
fn help_lists_examples() {
    let output = run(&["--help"]);
    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("wh-graph build --corpus ./corpus --out ./bundle"));
    assert!(stdout.contains("wh-graph validate --bundle ./bundle"));
    let build_help = run(&["build", "--help"]);
    let build_stdout = String::from_utf8(build_help.stdout).unwrap();
    assert!(build_stdout.contains("Examples"));
    assert!(build_stdout.contains("--dry-run --output json"));
}

#[test]
fn missing_flag_exits_2_with_a_working_command() {
    let output = run(&["build", "--corpus", "./corpus"]);
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("wh-graph build --corpus ./corpus --out ./bundle"));
}

#[test]
fn bad_output_exits_2() {
    let output = run(&["validate", "--bundle", "./bundle", "--output", "yaml"]);
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("bad flag: --output"));
    assert!(stderr.contains("wh-graph validate --bundle ./bundle"));
}

#[test]
fn rich_corpus_builds_the_spec_graph() {
    let tmp = tempfile();
    let corpus = tmp.join("corpus");
    copy_dir(&fixture("rich"), &corpus);
    let first = tmp.join("bundle-a");
    let second = tmp.join("bundle-b");
    let built = run(&[
        "build",
        "--corpus",
        corpus.to_str().unwrap(),
        "--out",
        first.to_str().unwrap(),
    ]);
    assert_eq!(
        built.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&built.stderr)
    );
    let again = run(&[
        "build",
        "--corpus",
        corpus.to_str().unwrap(),
        "--out",
        second.to_str().unwrap(),
    ]);
    assert_eq!(
        again.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&again.stderr)
    );
    for name in [
        "nodes.jsonl",
        "edges.jsonl",
        "passages.jsonl",
        "graph.postcard",
        "manifest.json",
    ] {
        assert_eq!(
            fs::read(first.join(name)).unwrap(),
            fs::read(second.join(name)).unwrap(),
            "{name} differs between builds"
        );
    }
    let overwrite = first.join("nodes.jsonl");
    let before = fs::read(&overwrite).unwrap();
    let third = run(&[
        "build",
        "--corpus",
        corpus.to_str().unwrap(),
        "--out",
        first.to_str().unwrap(),
    ]);
    assert_eq!(third.status.code(), Some(0));
    assert_eq!(before, fs::read(overwrite).unwrap());

    let manifest = fs::read_to_string(first.join("manifest.json")).unwrap();
    assert!(manifest.starts_with("{\n  \"format_version\": 1,\n  \"corpus_schema_version\": 1,\n"));
    assert!(manifest.contains("\"passage_count\":"));
    let node_rows = nodes(&first);
    let edge_rows = edges(&first);
    assert_eq!(node_rows.last().unwrap()["kind"], "Keyword");
    assert_eq!(node_rows[node_rows.len() - 2]["kind"], "Wargear");
    assert_eq!(edge_rows.last().unwrap()["kind"], "DATASHEET_HAS_WARGEAR");
    let datasheet = kind(&node_rows, "Datasheet");
    assert_eq!(datasheet.len(), 1);
    assert_eq!(datasheet[0]["attrs"]["faction_id"], "EX1");
    assert_eq!(datasheet[0]["attrs"]["faction_node"], "10ed:faction:EX1");
    assert_eq!(
        datasheet[0]["source_url"],
        "https://example.invalid/datasheets/example"
    );
    let keywords = kind(&node_rows, "Keyword");
    assert_eq!(keywords.len(), 1);
    assert_eq!(keywords[0]["id"], "10ed:keyword:example");
    assert_eq!(keywords[0]["label"], "Example");
    assert_eq!(keywords[0]["attrs"]["normalized"], "example");
    assert!(keywords[0]["source_url"].is_null());
    let keyword_edges = kind(&edge_rows, "DATASHEET_HAS_KEYWORD");
    assert_eq!(keyword_edges.len(), 2);
    let models = keyword_edges
        .iter()
        .map(|edge| edge["attrs"]["model"].as_str().unwrap().to_string())
        .collect::<Vec<_>>();
    assert_eq!(
        sorted(models),
        vec!["Alpha".to_string(), "Beta".to_string()]
    );

    let abilities = kind(&node_rows, "Ability");
    assert_eq!(abilities.len(), 2);
    assert!(abilities
        .iter()
        .any(|node| node["id"] == "10ed:datasheet_ability:EXDS:1"));
    assert!(abilities
        .iter()
        .any(|node| node["id"] == "10ed:ability:EXAB"));
    assert!(!abilities
        .iter()
        .any(|node| node["id"] == "10ed:datasheet_ability:EXDS:2"));
    let inline = abilities
        .iter()
        .find(|node| node["id"] == "10ed:datasheet_ability:EXDS:1")
        .unwrap();
    assert_eq!(inline["text"], "Inline Ability\nExample rule.\n\ncontinued");
    let shared = kind(&edge_rows, "DATASHEET_HAS_ABILITY")
        .into_iter()
        .find(|edge| edge["to"] == "10ed:ability:EXAB")
        .unwrap();
    assert_eq!(shared["attrs"]["type"], "Aura");
    assert_eq!(shared["attrs"]["parameter"], "6");

    let wargear = kind(&node_rows, "Wargear");
    assert_eq!(wargear.len(), 1);
    assert_eq!(wargear[0]["label"], "Second Profile");
    assert_eq!(
        wargear[0]["attrs"]["profile_entity_ids"],
        serde_json::json!([
            "10ed:datasheet_wargear:EXDS:1:2",
            "10ed:datasheet_wargear:EXDS:1:10"
        ])
    );
    assert_eq!(
        wargear[0]["text"],
        "Second Profile\n24 Melee 1 3+ 4 0 1\n12 Ranged 2 4+ 5 -1 2\nExample profile."
    );
    assert_eq!(
        wargear[0]["source_url"],
        "https://example.invalid/datasheets/example"
    );

    let lead = kind(&edge_rows, "DATASHEET_CAN_LEAD");
    assert_eq!(lead.len(), 1);
    assert_eq!(lead[0]["from"], "10ed:datasheet:EXDS");
    assert_eq!(lead[0]["to"], "10ed:datasheet:EXDS");

    let validated = run(&["validate", "--bundle", first.to_str().unwrap()]);
    assert_eq!(
        validated.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&validated.stderr)
    );

    let dry = tmp.join("dry");
    let dry_run = run(&[
        "build",
        "--corpus",
        corpus.to_str().unwrap(),
        "--out",
        dry.to_str().unwrap(),
        "--dry-run",
        "--output",
        "json",
    ]);
    assert_eq!(
        dry_run.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&dry_run.stderr)
    );
    assert!(!dry.exists());
    let payload: Value = serde_json::from_slice(&dry_run.stdout).unwrap();
    assert_eq!(payload["format_version"], 1);
    assert_eq!(payload["nodes"], node_rows.len() as u64);
    assert_eq!(payload["passages"], node_rows.len() as u64);
}

#[test]
fn null_faction_ref_exits_4_and_leaves_out_unchanged() {
    let tmp = tempfile();
    let corpus = tmp.join("corpus");
    copy_dir(&fixture("null-faction"), &corpus);
    let out = tmp.join("bundle");
    fs::create_dir_all(&out).unwrap();
    fs::write(out.join("marker.txt"), b"keep").unwrap();
    let built = run(&[
        "build",
        "--corpus",
        corpus.to_str().unwrap(),
        "--out",
        out.to_str().unwrap(),
    ]);
    assert_eq!(
        built.status.code(),
        Some(4),
        "{}",
        String::from_utf8_lossy(&built.stderr)
    );
    let stderr = String::from_utf8(built.stderr).unwrap();
    assert!(stderr.contains("10ed:datasheet:EXDS"));
    assert!(stderr.contains("FACTION_HAS_DATASHEET"));
    assert_eq!(fs::read(out.join("marker.txt")).unwrap(), b"keep");
    assert!(!out.join("nodes.jsonl").exists());
    assert!(!tmp.join(".bundle.tmp").exists());
    let leftovers = fs::read_dir(&tmp)
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect::<Vec<_>>();
    assert!(
        leftovers.iter().all(|name| {
            let text = name.to_string_lossy();
            !text.contains(".tmp-") && !text.contains(".bak-")
        }),
        "{leftovers:?}"
    );
}

#[test]
fn schema_version_2_exits_3() {
    let tmp = tempfile();
    let corpus = tmp.join("corpus");
    copy_dir(&fixture("rich"), &corpus);
    let manifest_path = corpus.join("manifest.json");
    let manifest = fs::read_to_string(&manifest_path)
        .unwrap()
        .replace("\"schema_version\":1", "\"schema_version\":2");
    fs::write(&manifest_path, manifest).unwrap();
    let out = tmp.join("bundle");
    fs::create_dir_all(&out).unwrap();
    fs::write(out.join("marker.txt"), b"keep").unwrap();
    let built = run(&[
        "build",
        "--corpus",
        corpus.to_str().unwrap(),
        "--out",
        out.to_str().unwrap(),
    ]);
    assert_eq!(
        built.status.code(),
        Some(3),
        "{}",
        String::from_utf8_lossy(&built.stderr)
    );
    let stderr = String::from_utf8(built.stderr).unwrap();
    assert!(stderr.contains("schema_version"));
    assert_eq!(fs::read(out.join("marker.txt")).unwrap(), b"keep");
}

#[test]
fn validate_rejects_a_missing_edge_target() {
    let tmp = tempfile();
    let corpus = tmp.join("corpus");
    copy_dir(&fixture("rich"), &corpus);
    let bundle = tmp.join("bundle");
    let built = run(&[
        "build",
        "--corpus",
        corpus.to_str().unwrap(),
        "--out",
        bundle.to_str().unwrap(),
    ]);
    assert_eq!(
        built.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&built.stderr)
    );
    let edge_path = bundle.join("edges.jsonl");
    let mut rows = read_jsonl(&edge_path);
    rows[0]["to"] = Value::String("10ed:missing:nope".to_string());
    let mut body = String::new();
    for row in &rows {
        body.push_str(&serde_json::to_string(row).unwrap());
        body.push('\n');
    }
    fs::write(&edge_path, body).unwrap();
    let validated = run(&["validate", "--bundle", bundle.to_str().unwrap()]);
    assert_eq!(
        validated.status.code(),
        Some(4),
        "{}",
        String::from_utf8_lossy(&validated.stderr)
    );
    let stderr = String::from_utf8(validated.stderr).unwrap();
    assert!(stderr.contains("10ed:missing:nope"));
}

fn sorted(mut values: Vec<String>) -> Vec<String> {
    values.sort();
    values
}

fn tempfile() -> PathBuf {
    let path =
        std::env::temp_dir().join(format!("wh-graph-test-{}-{}", std::process::id(), unique()));
    fs::create_dir_all(&path).unwrap();
    path
}

fn unique() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(1);
    NEXT.fetch_add(1, Ordering::Relaxed)
}
