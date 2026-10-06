# Getting started with wh-graph

`wh-graph` reads a corpus v1 directory and writes a CozoDB graph bundle. It does not download Wahapedia and it does not parse CSV. Build the corpus with [wh-corpus](wh-corpus.md) first.

The frozen contract is [02-rust-graph.md](../specs/02-rust-graph.md). This page is the install and command guide. [wh-ask](wh-ask.md) loads the bundle this tool writes.

## Install

You need Rust 1.83 or newer. The workspace `Cargo.toml` members are `crates/wh-graph` and `crates/wh-ask`. A normal Rust toolchain is enough, with no native build step.

From the repository root:

```bash
cargo build -p wh-graph
cargo run -p wh-graph -- --help
```

The binary is `target/debug/wh-graph`. `cargo build -p wh-graph --release` writes `target/release/wh-graph`.

## Build a bundle

```bash
cargo run -p wh-graph -- build --corpus ./corpus --out ./bundle
```

`build` writes a temporary directory and renames it onto `--out` only after validation passes. A failed build leaves an existing bundle in place. Building the same corpus again overwrites the bundle with identical `nodes.jsonl`, `edges.jsonl`, and `passages.jsonl` bytes.

Text output:

```text
bundle: ./bundle
format_version: 2
nodes: 100
edges: 250
passages: 100
corpus_fingerprint: <64 lowercase hex chars>
```

`--output json` prints one JSON object with those fields.

The bundle directory:

```text
bundle/manifest.json
bundle/nodes.jsonl
bundle/edges.jsonl
bundle/passages.jsonl
bundle/graph.db
```

`format_version` is `2`. `corpus_fingerprint` is the SHA-256 of `entities.jsonl`. `passages.jsonl` has one passage per node. The passage text is plain text with HTML stripped. `graph.db` is a CozoDB SQLite file of the same nodes, edges, and passages. `wh-ask` opens `manifest.json` and `graph.db` and reads them into memory; it does not read the JSONL files.

A Model node's `attrs` carry `M`, `T`, `Sv`, `inv_sv`, `inv_sv_descr`, `W`, `Ld`, and `OC` as the export prints them. An invulnerable save is a bare number such as `4`, or `-` for none. `wh-ask` turns those into `4+` and `None`. A bundle built before `inv_sv` was added lacks those attrs, so rebuild it to get invulnerable saves.

A corpus warning for an unresolved non-faction link does not fail the build. That edge is simply absent. A datasheet whose faction reference is null fails validation, the build exits 4, and `--out` is not replaced.

Preview counts without writing `--out`:

```bash
cargo run -p wh-graph -- build --corpus ./corpus --out ./bundle --dry-run --output json
```

Check a bundle that is already on disk:

```bash
cargo run -p wh-graph -- validate --bundle ./bundle
```

`validate` checks format version 2, that every edge endpoint exists, that each datasheet has exactly one faction edge, that keyword labels are unique after normalization, and that the Cozo payloads match the JSONL.

## Commands

```text
wh-graph build --corpus ./corpus --out ./bundle
wh-graph build --corpus ./corpus --out ./bundle --dry-run --output json
wh-graph validate --bundle ./bundle
```

| Flag | Commands | Meaning |
| --- | --- | --- |
| `--corpus PATH` | `build` | Corpus v1 directory (`manifest.json` and `entities.jsonl`). |
| `--out PATH` | `build` | Bundle directory to write. Ignored for writing when `--dry-run` is set. |
| `--bundle PATH` | `validate` | Bundle directory to read. |
| `--dry-run` | `build` | Parse, build, and print counts. Do not write `--out`. |
| `--output text\|json` | both | Default `text`. |

A missing flag or a bad `--output` exits 2 and prints one working command. There is no prompt.

## Exit codes

| Code | When |
| --- | --- |
| 0 | Success. |
| 2 | Missing flag or bad `--output`. |
| 3 | The corpus or bundle cannot be read, JSON is malformed, `schema_version` is not `1`, or `format_version` is not `2`. |
| 4 | Graph validation failed. Stderr names the first failing node id and the rule. |

## Tests

```bash
cargo test -p wh-graph
```

The tests build a synthetic corpus fixture. They do not embed published rules text and they do not use the network.

## Next step

Read the bundle from Rust with [wh-ask](wh-ask.md):

```rust
let bundle = wh_ask::Bundle::open(std::path::Path::new("./bundle"))?;
println!("{} nodes", bundle.manifest().node_count);
```
