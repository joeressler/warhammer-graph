# Using wh-mcp with a local Ollama model

`examples/ollama-host/` is a small Python program that connects a local [Ollama](https://ollama.com) model to the [wh-mcp](wh-mcp.md) server. Ollama runs models but is not an MCP client, so the host sits between the two: it starts `wh-mcp`, shows its tools to the model, runs the tool calls the model asks for, and feeds the results back until the model answers in plain text.

```text
you  ->  host.py  ->  Ollama model  <->  wh-mcp (started by host.py)  ->  bundle
```

It is an example and a test of the server with a model that is not Claude. It is not part of the Rust workspace or of the `wh-corpus` package, and its dependencies are listed separately.

## Install

You need Ollama running with a model that supports tool calling, a built `wh-mcp`, and a bundle:

```bash
cargo build -p wh-mcp --release
cargo run -p wh-graph -- build --corpus ./corpus --out ./bundle
python -m pip install -r examples/ollama-host/requirements.txt
ollama pull granite4.1:8b
```

`ollama show <model>` lists a model's capabilities. It must include `tools`.

## Ask a question

```bash
python examples/ollama-host/host.py --ask "What invulnerable save does Angron have?"
python examples/ollama-host/host.py --trace      # interactive, and show each tool call
```

`--trace` prints each tool call to stderr, for example:

```text
  [ok] find_units({"name": "Angron"}) -> 175 chars in 3 ms
  [ok] get_unit({"unit": "10ed:datasheet:000002621", "sections": ["models", "wargear"]}) -> 1051 chars in 2 ms
```

| Flag | Meaning |
| --- | --- |
| `--model` | The Ollama model. Default `granite4.1:8b`. |
| `--bundle`, `--exe` | The bundle directory and the `wh-mcp` binary. Defaults are `./bundle` and `target/release` or `target/debug`. |
| `--num-ctx` | The model's context window in tokens. Default 16384. |
| `--max-steps` | Most tool-calling rounds per question. Default 8. |
| `--max-tool-chars` | Cut a tool result to this length, with a note. Default 12000. 0 means no cut. |
| `--tools` | Offer only these comma-separated tools, such as `get_unit,get_roster,find_units`. |
| `--ollama-host` | A different Ollama server URL. |

## How it works

1. **Connect.** The host launches `wh-mcp` over stdio with the official `mcp` Python client and reads the server's tools and its usage instructions.
2. **Convert.** Each MCP tool becomes an Ollama tool, with its description and input schema passed through unchanged. Ollama accepts the schemas as they are, including the `sections` enums.
3. **Prompt.** The system prompt is the server's own instructions, then a short host note: answer only from the tools, and read an error and call again with a corrected name or an id it offers.
4. **Loop.** Ask the model. If it calls tools, run them through MCP and send the results back. Repeat until it answers in text or reaches `--max-steps`.
5. **Errors.** A failed tool call, an unknown tool, or a tool outside `--tools` becomes text the model can read, so it can recover. It never ends the conversation.

Three details matter for local models:

- **Context.** Ollama's default context window is small, and a unit card is about 9 KB of JSON. The host sets `num_ctx` to 16384 and cuts a tool result at 12,000 characters. A full Space Marines roster is about 59 KB, so a small model asked for one gets a cut roster with a note that says so.
- **Output encoding.** Models write typographic characters such as a non-breaking hyphen, which a Windows pipe (cp1252) cannot encode. The host switches stdout to UTF-8 so a finished answer is never lost.
- **One conversation.** In interactive mode, follow-up questions keep the earlier ones.

## Tests

The offline tests need no Ollama. They drive the host loop with a scripted fake model against the real `wh-mcp` binary and a small synthetic bundle built from the repository's example corpus:

```bash
cargo build -p wh-mcp -p wh-graph
python -m pytest examples/ollama-host/tests
```

They cover tool conversion and allowlists, a tool call running through the real server, errors going back to the model, an unknown tool, the step limit, result cutting, follow-up questions, the grading rules, and UTF-8 output.

The live tests talk to a real model and the real bundle, so they are skipped unless asked for:

```bash
OLLAMA_LIVE=1 python -m pytest examples/ollama-host/tests/test_live.py -v
OLLAMA_LIVE=1 OLLAMA_MODEL=granite4.1:3b python -m pytest examples/ollama-host/tests/test_live.py -v
```

## Comparing models

`evaluate.py` runs the question set against one or more models and prints a table of passes out of runs. A run passes when the model called an expected tool and its answer states every expected fact. Every expected fact comes from the real bundle.

```bash
python examples/ollama-host/evaluate.py --models granite4.1:8b,granite4.1:3b --runs 2 --json results.json
```

It needs the real bundle. Local models are not deterministic, so run each question more than once.

## Results

Run on 2026-10-06 against the real 26,406-node bundle, with Ollama 0.35.1, `mcp` 2.3.0, and `wh-mcp` 0.1.0. Each question ran twice per model at temperature 0, with the context window at 16,384 tokens and the host's defaults. Cells are passes out of runs.

| Question | granite4.1:8b | granite4.1:3b | lfm2.5:8b-a1b | qwen3:0.6b |
|---|---|---|---|---|
| `angron_invuln` | 2/2 | 2/2 | 2/2 | 0/2 |
| `angron_weapons` | 2/2 | 0/2 | 2/2 | 0/2 |
| `warboss_points` | 2/2 | 2/2 | 2/2 | 0/2 |
| `dropship_transport` | 0/2 | 1/2 | 2/2 | 2/2 |
| `imotekh_leads` | 2/2 | 0/2 | 2/2 | 0/2 |
| `khorne_units` | 0/2 | 0/2 | 0/2 | 0/2 |
| `idols_of_khorne` | 2/2 | 2/2 | 2/2 | 0/2 |
| `fnp_count` | 0/2 | 0/2 | 0/2 | 0/2 |
| `auric_stratagems` | 2/2 | 1/2 | 0/2 | 1/2 |
| `typo_recovery` | 0/2 | 2/2 | 2/2 | 0/2 |
| `daemonic_incursion` | 0/2 | 0/2 | 2/2 | 1/2 |
| **Total** | **12/22** | **10/22** | **16/22** | **4/22** |
| Median time per question | 8 s | 3 s | 7 s | 3 s |

Read this table with care. At temperature 0 the two runs of a cell mostly repeat each other, so each cell is closer to one sample than two, and four models and eleven questions are a small test. It shows what is hard, not a ranking to rely on.

What the table shows:

- **Models find the right tool far more often than they get the right fact.** The model called an expected tool in 22 of 22 runs for `granite4.1:8b`, 20 of 22 for `granite4.1:3b` and `lfm2.5`, and 16 of 22 for `qwen3:0.6b`. Most failures are in reading the reply, not in choosing the call.
- **Single lookups of a typed fact work well.** Angron's invulnerable save and weapon profiles, a Warboss's points, who gets Idols of Khorne, and Imotekh's leaders pass for both 8B models.
- **Counting fails everywhere.** No model passed `khorne_units` (the true count is 21) or `fnp_count` (116). `granite4.1:8b` said 19 units for Khorne and 48 for Feel No Pain. The reply to `fnp_count` has `total: 116` at the top, and the model ignored it and counted rows. I checked whether the host's 12,000-character cut caused that. With no cut at all the model still answered 57, so the cut is not to blame.
- **A wrong unit can look confident.** For the misspelled "Taxtical Squad", `granite4.1:8b` followed the server's suggestions to "Bike Squad" and described that unit, instead of the Tactical Squad the user meant. The smaller `granite4.1:3b` and `lfm2.5` recovered.
- **Ambiguity is easy to skip.** For "Daemonic Incursion", which is two detachments in one faction, both granite models picked one id and answered without mentioning the other. Only `lfm2.5` reported both every time, and `qwen3:0.6b` once.
- **Free text is read less reliably than fields.** The Orion Assault Dropship's transport capacity is in the datasheet text. `granite4.1:8b` said it carries "1 model", and `granite4.1:3b` said 30 once.
- **`lfm2.5:8b-a1b-q8_0` scored best here (16/22)** but was the least tidy. It hit the step limit in 4 runs, looping on `search`, sometimes emitted a malformed tool call as plain text, and wrote its reasoning into the answer in every run. The host strips `<think>` text, and re-grading with it stripped changed none of the verdicts.
- **`qwen3:0.6b` is too small** for 15 tools. It often said the tools did not contain the data when they did.

What this suggests for the server, which are ideas and not yet changes:

- **Add counts where a model has to count.** A `count` on rosters, and a more prominent `total` on the long lists, would remove the two failures every model shares.
- **Suggest better names for a misspelled one.** The suggestions come from word overlap, so "Taxtical Squad" suggests `Bike Squad`. An edit-distance match would suggest `Tactical Squad`.
- **Say more plainly that a name is shared.** The ambiguity error lists the ids, but models can still pick one and carry on.

To reproduce or extend the table, run `evaluate.py` with the models you have.

