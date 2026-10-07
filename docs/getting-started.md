# Getting started

This repository is a local pipeline. It downloads the public Wahapedia Warhammer 40,000 10th-edition export, turns that export into a CozoDB knowledge graph, and gives code read access to that graph through a Rust library and an MCP server. The library returns structured rows. Nothing here generates text, so nothing needs a language model; an AI agent that connects to the MCP server does the reasoning. The repository does not store the export or rules text. `wh-corpus` downloads the files when you run it.

```text
Wahapedia CSV export
        │
        ▼
wh-corpus          ./cache and ./corpus
        │          manifest.json + entities.jsonl
        ▼
wh-graph           ./bundle
        │          nodes, edges, passages, Cozo graph.db
        ▼
wh-ask             Rust library: open the bundle, then look up, search,
                   walk the graph, and list rosters, wargear, and rules
        │
        ▼
wh-mcp             MCP server: the same calls as tools for an AI agent
```

Each application has its own guide:

- [wh-corpus](getting-started/wh-corpus.md) — Python CLI that writes corpus v1.
- [wh-graph](getting-started/wh-graph.md) — Rust builder that writes the CozoDB graph bundle.
- [wh-ask](getting-started/wh-ask.md) — Rust library that opens the bundle and returns typed results.
- [wh-mcp](getting-started/wh-mcp.md) — MCP server that exposes the library to an AI agent over stdio.
- [Ollama host](getting-started/ollama-host.md) — a Python example that connects a local Ollama model to `wh-mcp`, with a comparison of five local models.

The contracts those tools follow are the specifications. Read them in this order when you need the frozen field lists, not the install steps:

1. [Python CLI and corpus v1](specs/01-python-cli.md), with [corpus-v1.schema.json](schemas/corpus-v1.schema.json).
2. [Rust graph builder](specs/02-rust-graph.md).
3. [Rust library](specs/03-rust-library.md).
4. [MCP server](specs/04-mcp-server.md).

The `wh-corpus` help text includes the line `powered by Wahapedia`. Anything you build on the library that shows its results to people should credit Wahapedia too. Units, rules, and nodes in general carry a `wahapedia_link` you can show with them.

## Quick start

Once Python and Rust are installed (see below), one command does everything up to a working MCP server. From the repository root:

```bash
python scripts/setup.py
```

It installs `wh-corpus`, downloads the Wahapedia export, writes the corpus, compiles `wh-graph` and `wh-mcp`, builds and checks the graph bundle, and then prints the line that registers the server with Claude Code. The first run needs the network and takes several minutes, mostly compiling Rust. It stops at the first step that fails and says which one.

Options:

| Option | Effect |
| --- | --- |
| `--register` | Also register the server with Claude Code (`--scope user`, so every Claude Code session on this machine can use it). Needs the `claude` command. |
| `--skip-export` | Reuse the existing `./corpus` instead of downloading again. |
| `--no-install` | Do not pip-install `wh-corpus`. The script skips this on its own when the package is already importable. |
| `--edition 10ed` | The Wahapedia edition. |

Activate a virtual environment first if you want `wh-corpus` kept out of your main Python (see the steps below). Running the script again is safe: the download is cached, and the bundle is replaced only after the new one passes validation.

The steps below are what the script runs, one at a time, for when you want to run or change a single step.

## What you need

| Tool | Used by | Requirement |
| --- | --- | --- |
| Python | `wh-corpus` | 3.12 or newer |
| Rust | `wh-graph` and `wh-ask` | 1.83 or newer, via [rustup](https://rustup.rs/) |
| Rust | `wh-mcp` | 1.88 or newer, because the MCP toolkit requires it |

Nothing else. There is no native toolchain, no CMake, no model file, and no GPU. A normal Rust toolchain builds all three crates.

Check the compilers:

```bash
python3 --version
rustc --version
```

## Layout

```text
src/wh_corpus/          Python package and the wh-corpus console script
crates/wh-graph/        graph builder library and binary
crates/wh-ask/          bundle reader library (no binary)
crates/wh-mcp/          MCP server binary over the library
docs/specs/             frozen contracts
docs/schemas/           corpus v1 JSON Schema
tests/                  Python tests (synthetic CSV, no network)
```

Generated data stays outside the repo. A typical local run uses three directories you choose:

```text
./cache/10ed/           raw CSV files and cache-meta.json
./corpus/               manifest.json and entities.jsonl
./bundle/               manifest.json, JSONL files, and graph.db
```

## Run the steps by hand

Install `wh-corpus` from the repository root:

```bash
python3 -m venv .venv
source .venv/bin/activate
python -m pip install -e ".[dev]"
```

On Windows, activate with `.venv\Scripts\activate` instead of `source`.

Download the export and write corpus v1. This contacts `https://wahapedia.ru/wh40k10ed/`.

```bash
wh-corpus export --edition 10ed --cache-dir ./cache --out ./corpus
```

Build the bundle. From the repository root:

```bash
cargo run -p wh-graph -- build --corpus ./corpus --out ./bundle
```

Rebuild the bundle whenever you rebuild the corpus, or after pulling a change to `wh-graph`. The library rejects a bundle whose version or corpus fingerprint does not match, and the error says to rebuild.

Use the library. Add it to another crate's `Cargo.toml` by path, then open the bundle:

```rust
use std::path::Path;
use wh_ask::Bundle;

let bundle = Bundle::open(Path::new("./bundle"))?;
for unit in bundle.roster("Space Marines")?.units {
    println!("{} ({})", unit.name, unit.role.unwrap_or_default());
}
```

The [wh-ask guide](getting-started/wh-ask.md) lists every call.

Or let an AI agent use it through the MCP server. Build it and register it with a client such as Claude Code. The [wh-mcp guide](getting-started/wh-mcp.md) has the exact commands.

```bash
cargo build -p wh-mcp --release
claude mcp add wh -- /full/path/to/target/release/wh-mcp --bundle /full/path/to/bundle
```

## Tests

The default test suites use synthetic rows such as `Example Faction`. They do not call `wahapedia.ru`.

```bash
pytest
cargo test -p wh-graph
cargo test -p wh-ask
cargo test -p wh-mcp
```
