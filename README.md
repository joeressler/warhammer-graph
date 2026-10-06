# warhammer-graph

A local pipeline that turns the public Wahapedia Warhammer 40,000 export into a CozoDB knowledge graph, a Rust library that reads the graph and returns structured rows (rosters, abilities, wargear, points, model profiles, leaders, stratagems, and enhancements), and an MCP server that lets an AI agent call that library. Nothing here generates text. The agent does the reasoning and asks for the facts it needs.

Start with the [getting started guide](docs/getting-started.md). Each application has its own guide:

- [wh-corpus](docs/getting-started/wh-corpus.md) downloads the 10th-edition export and writes corpus v1.
- [wh-graph](docs/getting-started/wh-graph.md) reads that corpus and writes a CozoDB graph bundle.
- [wh-ask](docs/getting-started/wh-ask.md) is the Rust library that opens the bundle and answers lookups, searches, and list requests.
- [wh-mcp](docs/getting-started/wh-mcp.md) is the MCP server that exposes the library to an AI agent over stdio. A [Python example](docs/getting-started/ollama-host.md) connects it to a local Ollama model.

Once the tools are installed, the pipeline is:

```bash
wh-corpus export --edition 10ed --cache-dir ./cache --out ./corpus
cargo run -p wh-graph -- build --corpus ./corpus --out ./bundle
```

Then read the bundle from Rust:

```rust
use std::path::Path;
use wh_ask::Bundle;

let bundle = Bundle::open(Path::new("./bundle"))?;
let roster = bundle.roster("Ultramarines")?;        // chapter units plus generic Space Marine units
let angron = bundle.unit("Angron")?;                // stats, points, abilities, wargear, leaders
let holders = bundle.units_with_ability("Feel No Pain")?;
```

Or let an AI agent call it. Build the server, then register it with an MCP client such as Claude Code:

```bash
cargo build -p wh-mcp --release
claude mcp add wh -- /full/path/to/target/release/wh-mcp --bundle /full/path/to/bundle
```

The specifications freeze the contracts those guides follow. Read them in this order. Each later document uses the contract the earlier one freezes.

1. [Python CLI and corpus v1](docs/specs/01-python-cli.md), with the JSON Schema in [corpus-v1.schema.json](docs/schemas/corpus-v1.schema.json). The CLI downloads the 10th-edition pipe-delimited export and writes `manifest.json` plus `entities.jsonl`.
2. [Rust graph builder](docs/specs/02-rust-graph.md). `wh-graph` reads corpus v1 and writes a CozoDB graph bundle (`nodes.jsonl`, `edges.jsonl`, `passages.jsonl`, `graph.db`).
3. [Rust library](docs/specs/03-rust-library.md). `wh-ask` opens that bundle and returns typed rows for name lookup, label search, graph navigation, and unit, roster, and rules lists.
4. [MCP server](docs/specs/04-mcp-server.md). `wh-mcp` exposes that library to an agent as 15 read-only tools over stdio.

These documents are the specifications for those tools. Wahapedia publishes CSV exports, not a live web service, and the corpus is downloaded when the CLI runs. The three Rust crates and the Python package in this repository implement those specifications.
