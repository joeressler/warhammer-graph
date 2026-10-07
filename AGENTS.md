# Workspace overview

A local pipeline from the public Wahapedia 10th-edition export to a read-only Rust library over a CozoDB graph. The library returns structured rows. It generates no text and uses no language model. The MCP server `wh-mcp` (Phase 4 of the data-first plan) wraps it for external agents. Phases 1 to 4 are done: delete the LLM layer, simplify, design the clean API, and build the MCP server.

```text
wh-corpus (Python)  ->  ./corpus   corpus v1: manifest.json + entities.jsonl
wh-graph  (Rust)    ->  ./bundle   nodes/edges/passages JSONL + graph.db (CozoDB SQLite)
wh-ask    (Rust lib) <-  ./bundle   Bundle::open, then lookups, search, graph walks, lists
wh-mcp    (Rust bin) <-  wh-ask     15 read-only MCP tools over stdio, for an AI agent
```

| Path | What it is |
| --- | --- |
| `src/wh_corpus/` | Python package and the `wh-corpus` console script (httpx, jsonschema, pydantic, typer). |
| `tests/` | Python tests with synthetic CSV and no network. |
| `crates/wh-graph/` | Graph builder: library plus the `wh-graph` binary. Fixtures in `tests/fixtures/{rich,null-faction}`. |
| `crates/wh-ask/` | Bundle reader library, no binary. `bundle.rs` (open, indexes, id/name resolution), `names.rs`, `search.rs`, `graph.rs`, `lists.rs`, `roster.rs`, `types.rs`, `error.rs`. Unit tests are in `names.rs` and `lists.rs`, integration tests in `tests/api.rs`. |
| `crates/wh-mcp/` | MCP server binary over `wh-ask`. `main.rs` (arguments, open the bundle, serve on stdio), `server.rs` (the 15 tools), `params.rs` (tool inputs), `reply.rs` (results and errors). `tests/stdio.rs` starts the real binary and speaks JSON-RPC to it. |
| `examples/ollama-host/` | A Python host that connects a local model to `wh-mcp`, through Ollama or llama.cpp's `llama-server`: `host.py` (the loop and CLI, `--backend`), `llamacpp.py` (the llama-server chat function), `serve_llamacpp.py` (starts llama-server on an Ollama model file), `questions.py` (graded questions), `evaluate.py` (model comparison), and `tests/`. Its dependencies are in its own `requirements.txt`, not `pyproject.toml`. |
| `scripts/setup.py` | One-step setup: runs `wh_corpus export`, `cargo build --release`, `wh-graph build` and `validate` as subprocesses, stopping at the first failure. Standard library only. Tested with faked commands in `tests/test_setup_script.py`. |
| `evals/` | Eval harness for the agent: `golden.jsonl` (60 questions, static), `author.py` (computes answers from the bundle), `run.py` (runs them through `examples/ollama-host/host.py`), `grade.py` (deterministic grader), `README.md` (scoring contract and the checked-in baseline in `baseline/`). Offline tests in `test_eval_harness.py`; the live run is `pytest evals -m eval --eval-model <model>`. |
| `docs/specs/` | Frozen contracts: `01-python-cli.md`, `02-rust-graph.md`, `03-rust-library.md`, `04-mcp-server.md`. |
| `docs/getting-started*` | Install and usage guides. `docs/schemas/` holds the corpus v1 JSON Schema. |

Generated data stays out of git: `cache/` (raw CSV), `corpus/`, `bundle/` (about 105 MB), and `target/` are all in `.gitignore`.

## Commands

```bash
python scripts/setup.py                 # one step: install wh-corpus, export, build the bundle, build wh-mcp (--register adds it to Claude Code)
# Python (use `python`, not `python3`, in Git Bash on this Windows machine)
python -m venv .venv                    # activate: .venv\Scripts\activate (PowerShell) or source .venv/Scripts/activate (Git Bash)
python -m pip install -e ".[dev]"
wh-corpus export --edition 10ed --cache-dir ./cache --out ./corpus
wh-corpus assemble --edition 10ed --cache-dir ./cache --out ./corpus   # rebuild corpus from the cache, no network
pytest                                  # without installing: PYTHONPATH=src python -m pytest

# Rust
cargo run -p wh-graph -- build --corpus ./corpus --out ./bundle     # rebuild the bundle
cargo run -p wh-graph -- validate --bundle ./bundle
RUSTDOCFLAGS="-D warnings" cargo doc -p wh-ask --no-deps            # rustdoc must stay warning-free
cargo build -p wh-mcp --release                                      # the MCP server (wh-mcp.exe on Windows)
cargo run -p wh-mcp -- --bundle ./bundle                             # waits for JSON-RPC on stdin; normally a client starts it
python -m pip install -r examples/ollama-host/requirements.txt       # for the Ollama host
python examples/ollama-host/host.py --trace --ask "What invulnerable save does Angron have?"
python -m pytest examples/ollama-host/tests                          # offline; needs cargo build -p wh-mcp -p wh-graph
OLLAMA_LIVE=1 python -m pytest examples/ollama-host/tests/test_live.py   # opt-in, needs Ollama and ./bundle
python examples/ollama-host/evaluate.py --models granite4.1:8b --runs 2   # model comparison table
python examples/ollama-host/serve_llamacpp.py --ollama-model granite4.1:8b   # llama.cpp server on :8080 (leave running)
python examples/ollama-host/host.py --backend llamacpp --trace --ask "..."   # the same host on llama.cpp
python -m evals.run --backend llamacpp --model granite4.1:8b          # the golden set on llama.cpp
cargo test --workspace
pytest evals                                                         # eval harness tests, offline
pytest evals -m eval --eval-model granite4.1:8b -s                   # live baseline: golden set through the real agent (~7 min)
python -m evals.author --write --check                               # regenerate golden.jsonl from ./bundle (a new golden set needs a new baseline)
```

Toolchain here is Rust 1.96.1 and Python 3.13. The docs state Rust 1.83 or newer (1.88 or newer for `wh-mcp`, which the MCP SDK requires) and Python 3.12 or newer as minimums. Only `wh-corpus fetch` and `export` use the network (they check the remote `Last_update.csv` even when `./cache` is warm). `assemble`, `validate`, `wh-graph`, `wh-ask`, `wh-mcp`, and every test run offline. No tool needs a native toolchain, a model file, or a GPU.

## Conventions

- Rust crates are edition 2021, `publish = false`, version 0.1.0, with every dependency pinned exactly (`=x.y.z`). Keep that.
- `wh-graph` exits 2 for a usage error, 3 for an unreadable corpus or bundle, and 4 for a failed validation. `wh-ask` returns `AskError` (`Bundle`, `NotFound`, `Ambiguous`, `Invalid`) and has no exit codes. `wh-mcp` exits 0 for help, version, or a closed stdin, 1 when the session fails, 2 for a usage error, and 3 for an unreadable bundle.
- Tests use small synthetic data and never the network. Do not commit published rules text. A throwaway program that runs against `./bundle` is the way to check real-data behavior, and it gets deleted afterward.
- The repository stores LF line endings, and `core.autocrlf` is `true`, so git converts on add and checkout. The working tree is a mix, with older files CRLF from the Windows checkout and newer files LF, and none are mixed within one file. Either is fine for commits. Do not convert a file wholesale. PowerShell 5.1 `Set-Content` and `Out-File -Encoding utf8` also add a BOM that these files do not have, so edit with the editor tools instead. To check endings, count bytes in Python (`b.count(b"\r\n")`); `grep -c $'\r'` in Git Bash matches every line and proves nothing.
- Keep docs in step with code. A change to a bundle attr, a public call, or a limit updates `docs/specs/` and the matching guide in the same change.
- History is merge-commit PRs from `agent/<topic>-<id>` branches into `main`.

## Learned User Preferences

- Prefer adapting corpus validation and schema to the live Wahapedia export rather than editing or cleaning the cached dataset.
- `wh-ask` is a data library, not a question-answering app: no LLM, no llama.cpp, no embeddings, no GUI, no CLI. An external agent or chatbot, through the `wh-mcp` server, does the reasoning and calls the library for facts. Do not reintroduce text generation or question parsing into either crate.
- Retrieval is label search for 0.1.0. Semantic search is deferred, not planned.
- Lists come from graph structure, not from text heuristics. Prefer a typed endpoint over teaching a caller to parse passages.
- When a name is a Space Marine chapter, a Chaos Space Marine chapter, or a Legiones Daemonica god, the roster also includes the generic parent units (Adeptus Astartes, Heretic Astartes, or godless Legiones Daemonica) that can be fielded with it. Another chapter's or god's units stay out.
- Faction rosters are lists of one unit per entry. A unit's abilities are that unit's own, with rules text, never another unit's, and never a Chaos God keyword roster just because an ability name contains a god word like Khorne.
- Wargear is one entry per weapon profile with its stats, so strike and sweep are separate entries. An ability such as Deadly Demise is never wargear.
- Lead lists come from `DATASHEET_CAN_LEAD` in the edge's direction. Do not answer with keywords, abilities, or the faction roster.
- Points are the printed cost lines. Characteristics are the model profile values (Move, Toughness, Save, invulnerable save, Wounds, Leadership, Objective Control), never wound brackets or wargear.
- Results carry names alongside ids. Ids are for follow-up calls. Anything shown to a person uses datasheet and faction names, not raw node ids.

## Learned Workspace Facts

- Live Wahapedia cache rows can have empty `datasheet_keyword.keyword` and `datasheet_wargear.line`. The corpus schema allows those empty strings and keeps the rows.
- `wh-graph build` writes the bundle to a temporary directory and replaces `--out` only after validation. The full bundle is 26,406 nodes and 165,697 edges, and `graph.db` is about 60 MB. Rebuild after changing `wh-graph` or the corpus. `wh-ask` rejects a `format_version` or `corpus_fingerprint` mismatch and says to rebuild.
- `Bundle::open` reads `manifest.json` and `graph.db` only, loads every node, passage, and edge into memory, and indexes edges by node. It takes about a second in a release build, and every call after that is a hash lookup. Per-call Cozo queries over the edge table took seconds each and were removed. Do not bring them back.
- `Bundle` is `Send` and `Sync` (a test guards it), so a server can share one open bundle behind an `Arc`.
- Caps: `search` at 100 results, `neighbors` at 500 per page, `subgraph` at depth 3 and 2000 edges, `find_units` at 10 fuzzy matches. Every list call returns all of its rows.
- Names resolve by name key: casefolded, `a`/`an`/`the` dropped, curly and straight apostrophes unified. So `Imotekh Stormlord` finds `Imotekh The Stormlord`, while `Stormlord` stays its own unit. A shared name is `Ambiguous`, listing each id and faction. A miss is `NotFound` with suggestions.
- Model `attrs` hold `M`, `T`, `Sv`, `inv_sv`, `inv_sv_descr`, `W`, `Ld`, and `OC` as the export prints them. `inv_sv` is `-` for none, a bare number like `4`, or `4*`. The library turns those into `4+` or `None` and strips the leading `*` from the note. A bundle built before `inv_sv` was added lacks it, so rebuild.
- Each weapon profile is its own `Wargear` node (`Gork’s Klaw - strike`, `Gork’s Klaw - sweep`). The stat line is `Range Type A Skill S AP D`, where Type is Ranged or Melee, followed by a line of keywords.
- A shared core ability (Deadly Demise, Feel No Pain) is one `Ability` node. The per-unit value is the `parameter` on the `DATASHEET_HAS_ABILITY` edge, so read it from the edge.
- Ability edge `type` values come straight from the export and include non-English labels, such as `Special (правая колонка)`. They pass through unchanged.
- `datasheet_leader` rows become `DATASHEET_CAN_LEAD` edges (not nodes), from the leader to the unit it may join. Transport text lives on the datasheet passage (no transport join table). Some leaders (for example Angron) are absent from the Wahapedia leader export, so the library returns an empty list and cannot invent lead targets.
- Chaos God names like Khorne can resolve as keywords or datasheets. "Units Khorne can take" is the Khorne daemon keyword roster plus the Legiones Daemonica units with no god keyword, not the Khorne keyword treated as a unit.
- Some datasheet names are longer than the common name: Marneus Calgar is `Marneus Calgar in Armour of Antilochus`, and the daemon princes are `Daemon Prince of Khorne` and so on. Look names up with `find_units` before asserting a unit is missing.
- `DATASHEET_USES_STRATAGEM` has about 91k edges. Walk stratagems from a detachment, not from a unit.
- The `wh-ask` public surface is `Bundle`, the result types, `AskError`, `Candidate`, and re-exports of `Attrs`, `EdgeRecord`, and `BundleManifest`. Everything else (the graph node type, error constructors, name resolution) is `pub(crate)`. `#![warn(missing_docs)]` is on, and the crate docs hold a compiled example. Lists of kinds and seeds are `&[&str]`.
- A datasheet's passage text is its name, role, loadout, transport capacity, leader rules, and damaged profile. Transport and the damaged profile have no separate fields, so `unit_text` returns that text without the name and role lines. Composition (`DATASHEET_HAS_COMPOSITION`) and wargear options (`DATASHEET_HAS_OPTION`) are their own nodes, and an option's text starts with a `•` line that the library strips.
- Detachment `attrs` hold `type`: empty for a standard detachment, `Boarding Actions` for a variant. A faction can have two detachments of one name (`Daemonic Incursion`, `Infestation Swarm`), so `Ambiguous.detail` shows the faction and the type, such as `Chaos Daemons (Boarding Actions)`. For an enhancement, stratagem, or detachment rule the detail is the owning detachment.
- `ENHANCEMENT_APPLIES_TO_DATASHEET` runs from the enhancement to the datasheet, and `DATASHEET_HAS_DETACHMENT_ABILITY` from the datasheet to the detachment rule. "Which units get Idols of Khorne" is `detachment_rule_units`, and it returns 11 units.
- A faction ability's text is its name, a flavor legend, then the rules, with no type line. Only a datasheet ability repeats its type line, and the library strips that using the edge's `type`.
- A `subgraph` of depth 2 through `DATASHEET_HAS_ABILITY` reaches every unit that shares a core ability such as Deep Strike, about a thousand nodes. Keep depth at 1 for a unit.
- Stratagem `attrs` hold `type`, `cp_cost`, `turn`, `phase`, and `detachment`, and enhancement `attrs` hold `cost` and `detachment`, as the export prints them. `wh-ask` returns typed `Stratagem` and `Enhancement` rows and drops the matching header lines from `text`. A bundle built before these attrs lacks them, so rebuild. Some stratagems (Boarding Actions) have an empty `detachment`.
- `NotFound` suggestions (and the `find_units` fallback) come first from edit distance on name keys (`names::edit_distance`, a swap of neighbours counts as one edit), then from label search. `Taxtical Squad` suggests `Tactical Squad`.
- `wh-mcp` list replies lead with a `summary` sentence and `count`/`total`, so models read the number instead of counting. This needs `serde_json`'s `preserve_order` feature, which keeps `summary` the first key. An `Ambiguous` error also carries an `instruction` telling the agent not to pick one silently.
- Tests build their own small bundle with `wh_graph::write_graph_db`, so they need no real data.
- `wh-mcp` uses `rmcp` 3.5.0 (the official Rust MCP SDK) over stdio. Its macros (`#[tool_router]`, `#[tool]`, `#[tool_handler]`) turn each method into a tool, and `#[tool_handler(instructions = ...)]` takes a string literal, not a constant. Tool inputs derive `Deserialize` and `rmcp::schemars::JsonSchema`, and their doc comments become the descriptions an agent reads.
- **Stdout is the MCP protocol.** `wh-mcp` must never print to stdout while serving; status goes to stderr with `eprintln!`. A test fails if any stdout line is not JSON-RPC.
- MCP errors are tool-level: `CallToolResult::error(..)` is flagged `isError` and the agent can read it, while a protocol error is rendered opaquely by clients. Library errors map to a JSON body with `error` plus `suggestions` or `candidates`. A malformed argument, such as an unknown section, also comes back as a tool-level error whose text lists the valid values.
- The MCP layer applies agent-sized caps below the library's: `search` default 10 and cap 50, `get_neighbors` 50 and 200, `get_subgraph` depth 2 and 100 edges and 150 nodes, and `units_with_keyword` and `units_with_ability` 200 and 1000 with `total` and `truncated`. `Infantry` alone is 774 units and about 150 KB, which is why. A full roster (Space Marines is 298 units, about 59 KB) is deliberately not cut.
- `get_unit` defaults to the full datasheet. `enhancements` and `detachment_rules` are opt-in sections, and `get_faction_rules` defaults to abilities only.
- On Windows, Python's `subprocess` cannot start a relative path with forward slashes, such as `target/release/wh-mcp.exe`. Use an absolute path. The binary is `wh-mcp.exe`, and Git Bash's `/tmp` is not the same directory Windows Python sees.
- `wh-mcp` has been tested with the official Python `mcp` client (2.3.0) and with local Ollama models through `examples/ollama-host/`. It has not been tried from Claude Code or Claude Desktop.
- The Python `mcp` 2.x client: `Client(StdioServerParameters(command=..., args=[...]), mode="legacy")` as an async context manager, with `list_tools()`, `call_tool(name, args)`, and `client.instructions`. Result fields are snake_case (`input_schema`, `is_error`). It negotiates protocol `2025-11-25` with `wh-mcp`. Use `mode="legacy"`, the plain `initialize` handshake.
- Ollama (0.35.1) accepts `wh-mcp`'s tool schemas unchanged, including the `$defs` enums on `sections`. A model must list `tools` in `ollama show`. Installed chat models that do: `granite4.1:8b` (the default), `granite4.1:3b`, `lfm2.5:8b-a1b-q8_0`, `qwen3:0.6b`, and `gpt-oss:20b` (13 GB, pulled 2026-10-06). The machine has an RTX 5000 Ada laptop GPU with 16 GB of VRAM and 64 GB of RAM, so about 13 GB of weights is the practical ceiling. Set `num_ctx` (the host uses 16384); Ollama's default is small.
- On Windows a piped Python stdout is cp1252 and raises on characters models write, such as the non-breaking hyphen U+2011. The host switches output to UTF-8. Grading in `questions.py` maps typographic dashes, quotes, and spaces to plain forms before comparing.
- The host's offline tests drive the real loop with a scripted fake model against the real `wh-mcp` and a synthetic bundle built by `wh-graph` from `crates/wh-graph/tests/fixtures/rich`. Do not use `./bundle` in tests that must run anywhere.
- Model comparison (2026-10-06, 11 questions, 2 runs, temperature 0, passes out of 22), before then after the count, spelling, and ambiguity changes: `gpt-oss:20b` 16 to 21, `granite4.1:8b` 12 to 19, `lfm2.5` 16 to 16, `granite4.1:3b` 10 to 15, `qwen3:0.6b` 4 to 12. The count in a `summary` sentence fixed `fnp_count` for every model. Still failing: free-text transport capacity (no typed field), Imotekh's leaders for small models, and `granite4.1:3b` skipping an ambiguity. The host's 12,000-character tool-result cut broke `gpt-oss:20b` once (nonsense replies), so try `--max-tool-chars 0` on large contexts. A 5-run, temperature 0.17 repeat gave 53, 49, 38, 33 and 26 of 55 in the same order (`gpt-oss:20b`, `granite4.1:8b`, `lfm2.5`, `granite4.1:3b`, `qwen3:0.6b`). See `docs/getting-started/ollama-host.md`.

- Eval baseline (2026-10-07, `granite4.1:8b`, temperature 0, `evals/golden.jsonl`): answer exact match 98.0%, tool correct 98.2%, abstain correct 33.3%, hallucination 1.7%. The weakness is declining: it answers refusal questions with a nearby real unit or a different edition's number. Later prompt, verifier, or training work must beat these on the same golden set. Keep `golden.jsonl` out of any training data.
- llama.cpp (build b11476, CUDA 13.4) is installed at `C:\Users\joear\llama.cpp\b11476`, outside OneDrive and the repo. `llama-server` needs `--jinja` for tool calling, fixes its context at start (`-c`, so the host's `--num-ctx` is ignored), speaks the OpenAI chat format, and rejects an assistant tool call without `"type": "function"` (HTTP 500). `llamacpp.py` converts the host's Ollama-shaped messages (made-up stable `call_<i>_<j>` ids). Ollama's model files are plain GGUF, so `serve_llamacpp.py --ollama-model X` serves the same weights without copying them.
- Same model on both servers (`granite4.1:8b`, 60 golden questions, temperature 0): Ollama 98.0% answers / 33.3% abstain / 1.7% hallucination, llama.cpp 96.1% / 33.3% / 3.3%, which is one question of noise. Samples are in `evals/baseline/` and `evals/baseline-llamacpp/`, made with `python -m evals.grade <run dir> --export-sample <dir>`.
- The grader strips node ids (`10ed:datasheet:000000614`) from answers before looking for numbers, because models cite ids.

## Leftovers from the removed LLM app

- The `models/` directory and the llama.cpp and eframe build output are gone. No model files are needed, and `models/` is no longer in `.gitignore`. The unused embedding code in `wh-graph`'s store is deleted too.
