# Rust frontend: `wh-ask`

This document specifies a local desktop app that answers questions from a graph bundle built by [02-rust-graph.md](02-rust-graph.md). It is written against that bundle and against corpus v1 only through the fingerprint the bundle already stores. The app does not download Wahapedia, does not parse CSV, and does not interpret `entities.jsonl`.

The query path is a function shared by the window and by a headless `query` subcommand. Embeddings are computed in the app from the user's embedding GGUF. The graph builder does not bake vectors, because a vector is meaningless for a different model.

## Crate

- Package name `wh-ask`, binary `wh-ask`.
- Rust edition 2021.
- UI: `eframe` / `egui`.
- Inference: `llama-cpp-2`, linked to a local llama.cpp build. The app loads GGUF files from disk. It does not download models.
- Graph: `petgraph` `StableGraph` with the same node and edge payloads as the bundle spec. Rebuild the graph from `nodes.jsonl` and `edges.jsonl` on every open. That rebuild is the normative load path.

`graph.postcard` is an optional accelerator. Use it only when it decodes and `validate`'s payload check against the JSONL would succeed for this `format_version`. If decode fails, ignore the file and keep the JSONL graph. Do not refuse to open a bundle that has no postcard file.

Reject `format_version` other than `1`.

The app has no HTTP client. After the bundle directory and the GGUF files are on disk, opening and querying them uses only the local filesystem.

The window footer always shows `powered by Wahapedia`.

## Launch

No prompts on the terminal. Flags prefill the settings screen. Missing files are shown in the window; the process stays up. The headless subcommand exits instead.

```text
wh-ask
wh-ask --bundle ./bundle --chat-model ./models/chat.gguf --embed-model ./models/embed.gguf
wh-ask query --bundle ./bundle --chat-model ./models/chat.gguf --question "Which example units are in Example Faction?"
wh-ask query --bundle ./bundle --chat-model ./models/chat.gguf --embed-model ./models/embed.gguf --question "..." --top-k 8
```

`--help` on the binary and on `query` includes those examples. `query` requires `--bundle`, `--chat-model`, and `--question`. `--embed-model` is optional. `--top-k` is an integer from 1 to 32 and defaults to 8.

`--output text|json` on `query` defaults to `text`. JSON is one object: `answer`, `citations`, `retrieval` (`embedding` or `label`).

### Exit codes for `query`

| Code | When |
| --- | --- |
| 0 | An answer was produced. |
| 2 | A required flag is missing or `--top-k` is outside 1..32. Stderr includes one working command. |
| 3 | The bundle is unreadable or `format_version` is not `1`. |
| 4 | The chat GGUF failed to load, or an embedding GGUF was passed and failed to load. Label search is not a fallback when the user did pass `--embed-model` and that file failed. |

The window uses the same codes only when the process cannot create a window at all. A bad path typed into settings stays on screen as an error string.

## Settings

Persist settings in the platform config directory under `wh-ask/settings.json`: bundle directory, chat GGUF path, optional embedding GGUF path, `top_k` (default 8), and context reserve tokens (default 1024). Paths are absolute after the user picks them.

The settings screen has three path fields and the top-k control. Picking a file does not start a download.

## Embedding index

When an embedding model is configured, embed every passage once per pair of bundle fingerprint and model fingerprint.

- Bundle fingerprint: SHA-256 of `nodes.jsonl` bytes, plus SHA-256 of `passages.jsonl` bytes, written in the index header. Also store `manifest.corpus_fingerprint` so a replaced corpus is obvious.
- Model fingerprint: SHA-256 of the GGUF file bytes. Hashing the whole file is acceptable at index time; cache the hash in the index header and recompute only when the file length or mtime changes, then confirm with a full hash before reuse.

Store the index outside the bundle, under the config directory:

```text
wh-ask/indexes/{bundle_passages_sha256}/{model_sha256}/index.bin
```

`index.bin` layout, little-endian:

| Field | Type |
| --- | --- |
| Magic | 6 bytes `WHASK1` |
| Dimension | u32, must match the model |
| Count | u32, must equal the passage count |
| Passage file SHA-256 | 32 bytes |
| Model file SHA-256 | 32 bytes |
| Records | `count` times: u32 byte length, UTF-8 `node_id`, then `dimension` f32 values |

Record order follows `passages.jsonl`. Vectors are L2-normalized. If the header does not match the open bundle and model, rebuild the index before searching.

The window may show an index-building state. `query` builds the index on first use and then searches. A second `query` with the same bundle and model reuses `index.bin`.

If no embedding model is configured, do not create an index. Retrieval is the label fallback below.

## Retrieval

### Embedding rank

1. Embed the question with the same model and L2-normalize it.
2. Score every passage by dot product.
3. Take the top `top_k` passages. Ties break by the order of `passages.jsonl`.

### Label fallback

Used only when no embedding model is configured.

1. Casefold the question and split it on whitespace into tokens. Drop tokens shorter than 2 characters.
2. Score each passage by the number of tokens that occur in the casefolded `title`, then in the casefolded `text`.
3. A passage whose title contains the entire casefolded question scores above token matches.
4. Take the top `top_k` with a score above 0. If none score above 0, the seed set is empty and the prompt says the bundle had no matching passage.

### Neighborhood

Seeds are the retrieved passages' nodes, best rank first.

Expand only along these kinds, in either direction: `FACTION_HAS_DATASHEET`, `FACTION_HAS_DETACHMENT`, `FACTION_HAS_ABILITY`, `FACTION_HAS_STRATAGEM`, `FACTION_HAS_ENHANCEMENT`, `FACTION_HAS_DETACHMENT_ABILITY`, `DATASHEET_HAS_KEYWORD`, `DATASHEET_HAS_ABILITY`, `DETACHMENT_HAS_ABILITY`, `DETACHMENT_HAS_STRATAGEM`, `DETACHMENT_HAS_ENHANCEMENT`, `DATASHEET_HAS_DETACHMENT_ABILITY`.

Steps:

1. Let `F` be every `Faction` node adjacent to a seed by one of those kinds.
2. Let `D` be every `Datasheet` node that is a seed, plus datasheet nodes adjacent to a seed that is a `Keyword`, `Ability`, `Stratagem`, `Enhancement`, `Detachment`, or `DetachmentAbility`. Add at most 8 of those extra datasheets, in the order of the seed rank that found them.
3. From each datasheet in `D`, add adjacent `Keyword` and `Ability` nodes, and detachment nodes reached through `DATASHEET_HAS_DETACHMENT_ABILITY` then `DETACHMENT_HAS_ABILITY`, or through a stratagem or enhancement seed's `DETACHMENT_HAS_*` edge.
4. Depth from a seed is at most 2.
5. The context list keeps every seed first, then faction nodes, then keywords, abilities, and detachments, then any other nodes added above. Cap the list at 32 nodes. Truncate from the end of that tail. Do not drop a seed to satisfy the cap unless the token budget step says to truncate text.

For each kept node, append the passage with that `node_id`.

Also collect edges whose `from` and `to` are both in the context. Render each as one line: `{from label} -{kind}-> {to label}` plus attrs in sorted-key form when attrs is non-empty.

## Prompt and answer

Send the chat model only this prompt. Do not add passages that were not selected above.

```text
You answer questions about Warhammer 40,000 using only the passages below.
When you use a passage, cite it as [node_id]. If that passage has a link, write the link on the same line as the citation.
If the passages do not contain the answer, say that they do not.

Edges:
{edge lines}

Passages:
---
title: {title}
node_id: {node_id}
link: {wahapedia_link or none}
{text}
---

Question: {question}
```

Token budget: reserve `context_reserve` tokens (default 1024) for the answer. If the prompt is larger than the model's context minus that reserve, drop expanded nodes from the end of the context list and rebuild the prompt. If the seeds alone still do not fit, truncate `text` from the end of the lowest-ranked seed, then the next, until the prompt fits. Never drop the question or the instruction paragraph.

Generation uses the loaded chat GGUF with the library defaults for temperature unless the settings screen later gains a control. The spec requires a deterministic test double, not a particular temperature.

### Citations

After generation:

1. Parse `[node_id]` occurrences from the answer. A node id matches the bundle's id grammar: `10ed:` plus a kind or table segment plus the rest of the id, or `10ed:edge:` is not a citation. Accept only ids that are node ids in the prompt.
2. Drop citations whose id was not in the prompt. They do not appear in the citation list.
3. The citation list is the prompt nodes that were either cited or, if the model cited nothing, the seed nodes. Each item shows `node_id`, title, `wahapedia_link` when present, and up to 8 neighbor labels from edges that touch that node inside the full graph (not only the prompt). Neighbor labels are the other endpoint's `label`, in edge-file order.
4. The on-screen answer is the model text unchanged, including any citation brackets that survived. The citation list is separate and always has links when the passage has one.

`query` text output prints the answer, then a `Citations:` block with `node_id`, title, and link on each line. JSON uses:

```json
{
  "answer": "...",
  "retrieval": "embedding",
  "citations": [
    {
      "node_id": "10ed:datasheet:EXDS",
      "title": "Example Unit",
      "wahapedia_link": "https://example.invalid/example-unit",
      "neighbors": ["Example", "Example Faction"]
    }
  ]
}
```

## Window

Regions, top to bottom:

1. Settings strip: bundle path, chat model path, embedding model path, top-k. A missing embedding path is a valid state and shows `Label search`.
2. Question box and a submit control. Submit runs the shared query function. While a query runs, submit is disabled.
3. Answer text.
4. Citation list. Selecting a citation shows that node's neighbor labels.
5. Footer: `powered by Wahapedia`.

Empty bundle, bad GGUF, and index-building errors replace the answer area with the error string. They do not clear the question.

## Out of scope

This spec does not cover army roster building, points math, detachment validation, or hosting a server. Those can be later documents. They are not implied by the query path.

## Tests

Unit tests use a synthetic bundle with a handful of nodes and a fake embedder and a fake chat model. They do not load a GGUF and they do not open a network socket.

- With no embedding model, the question `example unit` ranks the passage titled `Example Unit` first.
- With a fake embedder that returns a fixed vector per node id, the highest dot product is the seed, and expansion of a datasheet seed includes its keyword node and its faction node.
- Expansion stops at 32 nodes when the fixture is padded with extra keyword links.
- A chat double that cites a node id from the prompt and a node id that was not in the prompt yields a citation list that contains only the in-prompt id, and that item includes a neighbor label.
- A chat double that cites nothing still lists the seed passages.
- `format_version` `2` makes `query` exit 3.
- Reusing an `index.bin` whose passage hash matches does not call the embedder again. A changed passage hash does.
