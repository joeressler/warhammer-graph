# warhammer-graph

A local pipeline that turns the public Wahapedia Warhammer 40,000 export into a petgraph knowledge graph a local Rust app can query with llama.cpp.

Start with the [getting started guide](docs/getting-started.md). Each application has its own guide:

- [wh-corpus](docs/getting-started/wh-corpus.md) downloads the 10th-edition export and writes corpus v1.
- [wh-graph](docs/getting-started/wh-graph.md) reads that corpus and writes a petgraph bundle.
- [wh-ask](docs/getting-started/wh-ask.md) loads the bundle and answers questions with llama.cpp, citing node ids and Wahapedia links.

Once the tools are installed, the pipeline is:

```bash
wh-corpus export --edition 10ed --cache-dir ./cache --out ./corpus
cargo run -p wh-graph -- build --corpus ./corpus --out ./bundle
cargo run -p wh-ask -- query --bundle ./bundle --chat-model ./models/chat.gguf --question "Which units are in this faction?"
```

The specifications freeze the contracts those guides follow. Read them in this order. Each later document uses the contract the earlier one freezes.

1. [Python CLI and corpus v1](docs/specs/01-python-cli.md), with the JSON Schema in [corpus-v1.schema.json](docs/schemas/corpus-v1.schema.json). The CLI downloads the 10th-edition pipe-delimited export and writes `manifest.json` plus `entities.jsonl`.
2. [Rust graph builder](docs/specs/02-rust-graph.md). `wh-graph` reads corpus v1 and writes a petgraph bundle (`nodes.jsonl`, `edges.jsonl`, `passages.jsonl`, `graph.postcard`).
3. [Rust frontend](docs/specs/03-rust-frontend.md). `wh-ask` loads that bundle and answers questions with llama.cpp, citing node ids and Wahapedia links.

These documents are the specifications for those tools. Wahapedia publishes CSV exports, not a live web service, and the corpus is downloaded when the CLI runs. The three applications in this repository implement those specifications.
