# warhammer-graph

A local pipeline that turns the public Wahapedia Warhammer 40,000 export into a CozoDB knowledge graph, a Rust library that reads the graph and returns structured rows (rosters, abilities, wargear, points, model profiles, leaders, stratagems, and enhancements), and an MCP server that lets an AI agent call that library. Nothing here generates text. The agent does the reasoning and asks for the facts it needs.

## Quickstart: get the model and the tools, then connect your own client

There is a model fine-tuned for exactly this server (Granite 4.1 3B, about 2 GB). It answers by calling the 15
`wh-mcp` tools and says so when the data does not have the answer. It needs these tools to work. One command gets
both ready and then prints everything you need to connect whatever client you like.

**You need** (install once):

| What | Why | Get it |
| --- | --- | --- |
| Python 3.12 or newer | the setup scripts and the host | <https://www.python.org/downloads/> |
| Rust 1.88 or newer | builds the MCP server (`wh-mcp`) | <https://rustup.rs/> |
| `llama-server` (llama.cpp) | runs the model; tested with build b11476 | <https://github.com/ggml-org/llama.cpp/releases>. Unzip it and put it on your `PATH` (or pass `--llama-server <path>`). Take a CUDA build for an NVIDIA GPU, otherwise a CPU build. |

About 3 GB of disk, an internet connection, and a few minutes for the first build. A GPU is optional.

**Then run:**

```bash
git clone https://github.com/joeressler/warhammer-graph
cd warhammer-graph
python scripts/quickstart.py
```

It skips whatever is already done:

1. **Builds the tools.** Runs `scripts/setup.py`: downloads the public Wahapedia 10th-edition export (this repository
   does not contain it), builds the graph bundle in `./bundle`, and compiles `wh-mcp`.
2. **Downloads the model** from [Hugging Face](https://huggingface.co/jressler/granite-4.1-3b-wh40k-tools-GGUF) into
   `./models`. It resumes if interrupted and checks the SHA-256.
3. **Saves the system prompt** the model was trained with to `./models/system-prompt.txt`.
4. **Prints a connection sheet** and exits. Nothing is left running.

The sheet gives you, with your real absolute paths, ready to copy and paste:

- **Where everything is:** the model file, the bundle, the `wh-mcp` binary, the system prompt, and `llama-server`.
- **1. Start the model:** the exact `llama-server` command (with `--jinja`, which tool calling needs), plus the
  endpoint (`http://localhost:8080/v1`) and model name to give your client.
- **2. Add the tools:** the `wh-mcp` command and arguments, the same as an `mcpServers` JSON block, and the
  `claude mcp add` line.
- **3. Settings the model expects:** the system prompt file, temperature 0, a 16384-token context, the tool-call
  loop (about 8 rounds), cutting tool results at 12,000 characters, and returning tool errors as text.
- **A command to try it** without your own client, using the bundled host.

Print the sheet again any time with `python scripts/quickstart.py --info`.

| Option | Effect |
| --- | --- |
| `--serve` | After preparing, start the model server and keep it running (Ctrl+C stops it). |
| `--ask [QUESTION]` / `--chat` | Try the model first through the bundled host, one question or interactive. `--trace` shows each tool call. |
| `--info` | Only print the connection sheet. |
| `--ngl 0` | Run the model on the CPU only (slower). |
| `--llama-server PATH` | Use this `llama-server` (also: the `LLAMA_SERVER` variable, or `PATH`). |
| `--gguf FILE` | Use a model file you already have instead of downloading one. |
| `--port N` | Serve on this port (default 8080). |
| `--dry-run` | Say what is built, downloaded and found, and exit. |

The model is trained for these tools, their schemas and this server's instructions, so connect it through a client
that passes the system prompt and tools as the sheet says; asking it cold gives worse answers.

Models on Hugging Face: the [GGUF](https://huggingface.co/jressler/granite-4.1-3b-wh40k-tools-GGUF) used here, and
the [LoRA adapter](https://huggingface.co/jressler/granite-4.1-3b-wh40k-tools-lora) it was made from. It is an
unofficial fan project, **powered by Wahapedia** (<https://wahapedia.ru>); Warhammer 40,000 and its rules belong to
Games Workshop. To use the server with Claude Code or another model instead, see the sections below.

Start with the [getting started guide](docs/getting-started.md). Each application has its own guide:

- [wh-corpus](docs/getting-started/wh-corpus.md) downloads the 10th-edition export and writes corpus v1.
- [wh-graph](docs/getting-started/wh-graph.md) reads that corpus and writes a CozoDB graph bundle.
- [wh-ask](docs/getting-started/wh-ask.md) is the Rust library that opens the bundle and answers lookups, searches, and list requests.
- [wh-mcp](docs/getting-started/wh-mcp.md) is the MCP server that exposes the library to an AI agent over stdio. A [Python example](docs/getting-started/ollama-host.md) connects it to a local Ollama model.

With Python and Rust installed, one command downloads the data, builds the bundle, and builds the MCP server:

```bash
python scripts/setup.py
```

Or run the steps yourself:

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
