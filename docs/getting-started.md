# Getting started

This repository is a local pipeline. It downloads the public Wahapedia Warhammer 40,000 10th-edition export, turns that export into a petgraph knowledge graph, and answers questions from the graph with llama.cpp. The repository does not store the export or rules text. `wh-corpus` downloads the files when you run it.

```text
Wahapedia CSV export
        │
        ▼
wh-corpus          ./cache and ./corpus
        │          manifest.json + entities.jsonl
        ▼
wh-graph           ./bundle
        │          nodes, edges, passages, postcard snapshot
        ▼
wh-ask             your chat GGUF, and an optional embedding GGUF
```

Each application has its own guide:

- [wh-corpus](getting-started/wh-corpus.md) — Python CLI that writes corpus v1.
- [wh-graph](getting-started/wh-graph.md) — Rust builder that writes the petgraph bundle.
- [wh-ask](getting-started/wh-ask.md) — local window and `query` command that cite node ids and Wahapedia links.

The contracts those tools follow are the specifications. Read them in this order when you need the frozen field lists, not the install steps:

1. [Python CLI and corpus v1](specs/01-python-cli.md), with [corpus-v1.schema.json](schemas/corpus-v1.schema.json).
2. [Rust graph builder](specs/02-rust-graph.md).
3. [Rust frontend](specs/03-rust-frontend.md).

Help text and the `wh-ask` window include the line `powered by Wahapedia`.

## What you need

| Tool | Used by | Requirement |
| --- | --- | --- |
| Python | `wh-corpus` | 3.12 or newer |
| Rust | `wh-graph` and `wh-ask` | 1.83 or newer, via [rustup](https://rustup.rs/) |
| A C++ toolchain, CMake, and libclang | `wh-ask` only | Compiles the bundled llama.cpp when you build the desktop app |
| Linux window libraries | `wh-ask` window on Linux | X11 and Wayland packages listed in the [wh-ask guide](getting-started/wh-ask.md) |
| Chat GGUF | `wh-ask` | A model file you already have on disk. The app does not download models. |

`wh-graph` does not need llama.cpp. You can build the corpus and the bundle before you install the desktop libraries.

Check the compilers:

```bash
python3 --version
rustc --version
```

## Layout

```text
src/wh_corpus/          Python package and the wh-corpus console script
crates/wh-graph/        graph builder library and binary
crates/wh-ask/          query library, desktop window, and wh-ask binary
docs/specs/             frozen contracts
docs/schemas/           corpus v1 JSON Schema
tests/                  Python tests (synthetic CSV, no network)
```

Generated data stays outside the repo. A typical local run uses three directories you choose:

```text
./cache/10ed/           raw CSV files and cache-meta.json
./corpus/               manifest.json and entities.jsonl
./bundle/               manifest.json, JSONL files, and graph.postcard
```

## Run the pipeline

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

Ask a question. Place a chat GGUF on disk first. The first build of `wh-ask` compiles llama.cpp, so it takes longer than the graph build.

```bash
cargo run -p wh-ask -- query \
  --bundle ./bundle \
  --chat-model ./models/chat.gguf \
  --question "Which units are in this faction?"
```

Omit `--embed-model` to search passage titles and text by label. Pass an embedding GGUF when you want vector retrieval. Open the window with `cargo run -p wh-ask` after the same build. The application guides cover flags, outputs, and exit codes.

## Tests

The default test suites use synthetic rows such as `Example Faction`. They do not call `wahapedia.ru` and they do not load a GGUF.

```bash
pytest
cargo test -p wh-graph
cargo test -p wh-ask --no-default-features
```

`cargo test -p wh-ask` without `--no-default-features` also compiles the desktop feature, including llama.cpp and the window.
