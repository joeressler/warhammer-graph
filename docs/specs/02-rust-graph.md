# Rust graph builder: `wh-graph`

This document specifies the crate that reads a corpus v1 directory and writes a petgraph bundle. The corpus contract is [01-python-cli.md](01-python-cli.md) and [corpus-v1.schema.json](../schemas/corpus-v1.schema.json). The frontend in [03-rust-frontend.md](03-rust-frontend.md) reads the bundle defined here.

`wh-graph` does not download Wahapedia and does not parse CSV. It rejects a manifest whose `schema_version` is not `1`.

## Crate

- Package name `wh-graph`, binary `wh-graph`.
- Rust edition 2021.
- Dependencies: `clap` (derive), `petgraph` with feature `serde-1`, `serde` with `derive`, `serde_json`, `postcard` 1.x, `sha2`.
- Graph type: `petgraph::stable_graph::StableGraph<GraphNode, GraphEdge>`.

`petgraph`'s own serde layout is not the contract. The JSONL files are the contract. `graph.postcard` is a snapshot that must describe the same nodes and edges. A later frontend may ignore the postcard file and rebuild the graph from JSONL.

## Commands

No command prompts. Each subcommand has `--help` with examples. Missing flags exit 2 and print one working invocation.

```text
wh-graph build --corpus ./corpus --out ./bundle
wh-graph build --corpus ./corpus --out ./bundle --dry-run --output json
wh-graph validate --bundle ./bundle
```

`--output text|json` defaults to `text`. `--dry-run` on `build` parses, builds, and prints counts, and does not write `--out`.

`build` writes a temporary directory and renames it onto `--out` only after validation passes. A failed build leaves an existing bundle in place. A second build of the same corpus overwrites the bundle with identical `nodes.jsonl`, `edges.jsonl`, `passages.jsonl`, and `graph.postcard` bytes.

Text success:

```text
bundle: ./bundle
format_version: 1
nodes: 100
edges: 250
passages: 100
corpus_fingerprint: <64 lowercase hex chars>
```

### Exit codes

| Code | When |
| --- | --- |
| 0 | Success. |
| 2 | Missing flag or bad `--output`. Stderr includes one working command. |
| 3 | Corpus or bundle cannot be read, JSON is malformed, or `schema_version` / `format_version` is not `1`. |
| 4 | Graph validation failed. Stderr names the first failing node id and the rule. |

## What becomes a node

Read `entities.jsonl` in file order. Build nodes from these tables. Other tables are edges only, or they fold into a node as described.

| Node kind | Source | Node id |
| --- | --- | --- |
| `Faction` | `faction` | Entity id. |
| `Source` | `source` | Entity id. |
| `Datasheet` | `datasheet` | Entity id. |
| `Model` | `datasheet_model` | Entity id. |
| `Ability` | `ability`, and each `datasheet_ability` whose `ability_id` is blank | Entity id of that row. |
| `Keyword` | Distinct normalized keywords from `datasheet_keyword` | `10ed:keyword:{percent-encoded normalized keyword}`. |
| `Wargear` | `datasheet_wargear` rows grouped by `(datasheet_id, line)` | `10ed:wargear:{percent-encoded datasheet_id}:{percent-encoded line}`. |
| `Option` | `datasheet_option` | Entity id. |
| `UnitComposition` | `datasheet_unit_composition` | Entity id. |
| `ModelCost` | `datasheet_model_cost` | Entity id. |
| `Stratagem` | `stratagem` | Entity id. |
| `Enhancement` | `enhancement` | Entity id. |
| `Detachment` | `detachment` | Entity id. |
| `DetachmentAbility` | `detachment_ability` | Entity id. |

`Option`, `UnitComposition`, and `ModelCost` exist so the option, composition, and cost edges have endpoints. Keyword rows and wargear profile rows are not one node each.

### Keywords

Normalize before deduplicating:

1. Replace U+00A0 with a normal space.
2. Trim both ends.
3. Collapse every run of Unicode whitespace to one space.
4. Casefold with Unicode default case folding.

The node label is the first row in file order that normalizes to that key, after steps 1–3 and before casefold, so `Example` and ` example ` are one node labeled `Example`. Two keyword nodes must not share a normalized label. The builder fails validation if they would.

### Wargear groups

Profiles that share `datasheet_id` and `line` are one `Wargear` node. Sort profiles by `line_in_wargear` as an integer when every value in the group is an integer; otherwise sort them as strings. `attrs.profile_entity_ids` lists those entity ids in that order. The label is the `name` of the first profile after sorting.

The same weapon name on two datasheets stays two nodes. Profiles are not merged across datasheets.

### Shared and inline abilities

A `datasheet_ability` row with a non-null `refs.ability_id` does not create an ability node. It becomes a `DATASHEET_HAS_ABILITY` edge to the existing `Ability` node. `type` and `parameter` live on that edge, because they can differ per datasheet.

A row with a null `ability_id` ref creates an `Ability` node at the `datasheet_ability` entity id. Its text includes `type` and `parameter`.

## Edges

Emit an edge only when every endpoint node exists. A null ref produces no edge and no build failure, except the datasheet faction rule below. Never emit an edge whose `from` or `to` is missing from `nodes.jsonl`.

Dedupe edges that share `kind`, `from`, `to`, and canonical attrs. The first one in processing order wins.

Non-wargear edges are emitted while scanning `entities.jsonl` from top to bottom. After that scan, append one `DATASHEET_HAS_WARGEAR` edge per group, in the order each `(datasheet_id, line)` pair was first seen. Dedupe runs after both steps, keeping the earlier edge.

| Kind | From | To | When | Attrs |
| --- | --- | --- | --- | --- |
| `FACTION_HAS_DATASHEET` | Faction | Datasheet | `refs.faction_id` is non-null | none |
| `FACTION_HAS_STRATAGEM` | Faction | Stratagem | `refs.faction_id` is non-null | none |
| `FACTION_HAS_ABILITY` | Faction | Ability | `refs.faction_id` on an `ability` entity is non-null | none |
| `FACTION_HAS_ENHANCEMENT` | Faction | Enhancement | `refs.faction_id` is non-null | none |
| `FACTION_HAS_DETACHMENT` | Faction | Detachment | `refs.faction_id` is non-null | none |
| `FACTION_HAS_DETACHMENT_ABILITY` | Faction | DetachmentAbility | `refs.faction_id` is non-null | none |
| `DATASHEET_FROM_SOURCE` | Datasheet | Source | `refs.source_id` is non-null | none |
| `DATASHEET_HAS_MODEL` | Datasheet | Model | `refs.datasheet_id` is non-null | `line` |
| `DATASHEET_HAS_ABILITY` | Datasheet | Ability | Shared: `refs.ability_id` non-null. Inline: the inline ability node. | `line`, `model`, `type`, `parameter` |
| `DATASHEET_HAS_KEYWORD` | Datasheet | Keyword | `refs.datasheet_id` is non-null | `model`, `is_faction_keyword` |
| `DATASHEET_HAS_WARGEAR` | Datasheet | Wargear | The group's datasheet node exists | `line` |
| `DATASHEET_HAS_OPTION` | Datasheet | Option | `refs.datasheet_id` is non-null | `line` |
| `DATASHEET_HAS_COMPOSITION` | Datasheet | UnitComposition | `refs.datasheet_id` is non-null | `line` |
| `DATASHEET_HAS_COST` | Datasheet | ModelCost | `refs.datasheet_id` is non-null | `line` |
| `DATASHEET_CAN_LEAD` | Leader datasheet (`refs.leader_id`) | Attached datasheet (`refs.attached_id`) | Both refs non-null | none |
| `DATASHEET_USES_STRATAGEM` | Datasheet | Stratagem | Both refs on `datasheet_stratagem` non-null | none |
| `DETACHMENT_HAS_ABILITY` | Detachment | DetachmentAbility | `refs.detachment_id` on the detachment ability is non-null | none |
| `DETACHMENT_HAS_STRATAGEM` | Detachment | Stratagem | `refs.detachment_id` on the stratagem is non-null | none |
| `DETACHMENT_HAS_ENHANCEMENT` | Detachment | Enhancement | `refs.detachment_id` on the enhancement is non-null | none |
| `ENHANCEMENT_APPLIES_TO_DATASHEET` | Enhancement | Datasheet | Both refs on `datasheet_enhancement` non-null | none |
| `DATASHEET_HAS_DETACHMENT_ABILITY` | Datasheet | DetachmentAbility | Both refs on `datasheet_detachment_ability` non-null | none |

Edge id: `10ed:edge:{KIND}:{16 hex chars}`. The hex is the first 16 lowercase hex digits of SHA-256 over the UTF-8 bytes of `from_id`, a newline, `to_id`, a newline, and canonical attrs JSON. Canonical attrs JSON has sorted keys, compact separators, and string values. An edge with no attrs hashes `{}`.

`DATASHEET_CAN_LEAD` points from the leader datasheet to the datasheet it may join. The frontend can walk that edge in either direction.

## Node and edge records

JSONL is UTF-8, one object per line, LF, trailing LF, compact separators.

Node key order: `id`, `kind`, `label`, `text`, `attrs`, `source_url`.

```json
{"id":"10ed:keyword:example","kind":"Keyword","label":"Example","text":"Example","attrs":{},"source_url":null}
```

`kind` is one of the node kinds in the table above. `attrs` is an object of strings, except `Wargear.attrs.profile_entity_ids`, which is an array of strings. `source_url` is a string or null.

Edge key order: `id`, `kind`, `from`, `to`, `attrs`.

```json
{"id":"10ed:edge:DATASHEET_HAS_KEYWORD:0123456789abcdef","kind":"DATASHEET_HAS_KEYWORD","from":"10ed:datasheet:EXDS","to":"10ed:keyword:example","attrs":{"is_faction_keyword":"false","model":""}}
```

`attrs` on edges is always a string map. Keys are sorted in the file.

Node order:

1. Nodes whose id is an entity id, in `entities.jsonl` order, skipping entity rows that do not create a node (keyword rows, wargear rows, and datasheet abilities that point at a shared ability).
2. `Wargear` nodes in first-seen group order.
3. `Keyword` nodes in first-seen order.

Edge order is the processing order above, after dedupe.

### `source_url`

Use the first of these that is a non-empty string:

1. The entity's own `fields.link`.
2. The `link` of the datasheet reached by a `datasheet_id`, `leader_id` (the leader's page), or, for a grouped wargear node, that datasheet.
3. The `link` of the faction reached by `faction_id` on the entity.

Otherwise `null`. Keyword nodes use `null`, because one keyword spans many pages. The datasheet passage carries the page link.

### `text`

`text` is plain text for retrieval. Apply `html_to_text` to any field that may contain HTML (`description`, `legend`, `loadout`, `transport`, `leader_head`, `leader_footer`, `damaged_description`, `inv_sv_descr`, `base_size_descr`).

`html_to_text`:

1. Remove `<script>...</script>` and `<style>...</style>`, case-insensitive, non-greedy.
2. Replace `<br>`, `<br/>`, `</p>`, `</div>`, `</li>`, `</tr>`, and `</h1>` through `</h6>` with a newline.
3. Remove remaining tags.
4. Decode `&amp;`, `&lt;`, `&gt;`, `&quot;`, `&apos;`, `&nbsp;`, `&#39;`, decimal `&#nnn;`, and hex `&#xhhh;`. Ignore a numeric reference that is not a Unicode scalar.
5. Collapse horizontal whitespace on each line to a single space, trim each line, and collapse three or more newlines to two.
6. Trim the result.

Join the parts below with a single newline, and omit a part whose value is empty after `html_to_text`.

| Kind | Label | Text parts, in order |
| --- | --- | --- |
| `Faction` | `name` | `name` |
| `Source` | `name` | `type`, `name`, `edition`, `version` |
| `Datasheet` | `name` | `name`, `role`, `loadout`, `transport`, `leader_head`, `leader_footer`, `damaged_w`, `damaged_description` |
| `Model` | `name` | `name`, `M`, `T`, `Sv`, `inv_sv`, `inv_sv_descr`, `W`, `Ld`, `OC`, `base_size`, `base_size_descr` |
| `Ability` (shared) | `name` | `name`, `legend`, `description` |
| `Ability` (inline) | `name`, or `Ability` if the name is empty | `name`, `type`, `parameter`, `description` |
| `Keyword` | Display label | The display label |
| `Wargear` | First profile `name` | Weapon name, then one line per profile: `range`, `type`, `A`, `BS_WS`, `S`, `AP`, `D`, then that profile's `description` |
| `Option` | `Option` plus the `line` | `button`, `description` |
| `UnitComposition` | `Unit composition` plus the `line` | `description` |
| `ModelCost` | `Cost` plus the `line` | `description`, `cost` |
| `Stratagem` | `name` | `name`, `type`, `cp_cost`, `turn`, `phase`, `detachment`, `description` |
| `Enhancement` | `name` | `name`, `cost`, `detachment`, `legend`, `description` |
| `Detachment` | `name` | `name`, `type`, `legend` |
| `DetachmentAbility` | `name` | `name`, `detachment`, `legend`, `description` |

Datasheet `text` does not repeat child models, weapons, or abilities. Those are their own passages. The frontend pulls them by walking edges.

`attrs` copies the string fields that are not already the whole text and that a filter might need:

- Datasheet: `role`, `virtual`, `faction_id` (raw field, possibly joined as `attrs.faction_node` holding the corpus id or `""` when the ref is null).
- Model: `line`, `M`, `T`, `Sv`, `W`, `Ld`, `OC`.
- Wargear: `line`, `profile_entity_ids`.
- Keyword: `normalized` (the casefolded key).
- Edges: only the columns listed in the edge table.

Keep `attrs` small. The full raw strings remain available by going back to the corpus; the bundle does not copy every CSV column onto every node.

## Passages

`passages.jsonl` has one object per node, in node order. Key order: `node_id`, `title`, `text`, `wahapedia_link`.

- `node_id` equals the node `id`.
- `title` equals the node `label`.
- `text` equals the node `text`.
- `wahapedia_link` equals the node `source_url`.

The graph builder does not compute embeddings. Vectors depend on the user's GGUF model. [03-rust-frontend.md](03-rust-frontend.md) embeds passages when it opens a bundle.

## Bundle manifest

`manifest.json` key order:

```json
{
  "format_version": 1,
  "corpus_schema_version": 1,
  "edition": "10ed",
  "corpus_fingerprint": "<sha256 of entities.jsonl bytes>",
  "last_update": "<copied from the corpus manifest>",
  "node_count": 0,
  "edge_count": 0,
  "passage_count": 0,
  "node_counts_by_kind": {},
  "edge_counts_by_kind": {}
}
```

`corpus_fingerprint` is the lowercase hex SHA-256 of the exact `entities.jsonl` bytes. Count maps use kind names as keys, sorted, values as integers. `passage_count` equals `node_count`.

## Postcard snapshot

`graph.postcard` is `postcard` 1 encoding of `StableGraph<GraphNode, GraphEdge>`.

```text
GraphNode { id, kind, label, text, attrs, source_url }
GraphEdge { id, kind, from_id, to_id, attrs }
```

`GraphNode.attrs` uses the same JSON types as the JSONL `attrs` object (`serde_json::Value` restricted to object, string, and array of string). `from_id` and `to_id` repeat the endpoint ids so the snapshot does not depend on petgraph index numbers.

`validate` loads the postcard graph and the JSONL files and checks set equality of:

- `(node.id, node.kind, node.label, node.text, node.source_url)`
- `(edge.id, edge.kind, edge.from_id, edge.to_id, canonical attrs)`

Index numbers may differ. Payload equality may not.

## Validation

`build` and `validate` both enforce:

- `format_version` is `1` and `corpus_schema_version` is `1`.
- Every edge `from` and `to` is a node id.
- Every `Datasheet` node is the target of exactly one `FACTION_HAS_DATASHEET` edge. A datasheet whose faction ref was null fails the build. Exit 4. Do not publish the bundle.
- No two `Keyword` nodes share `attrs.normalized`.
- `passage_count` equals the node count, and each passage's `node_id`, `title`, `text`, and `wahapedia_link` match that node.
- Postcard payloads match JSONL, as above.
- Node and edge counts on the manifest match the files.

A corpus warning for an unresolved non-faction link does not fail the build. The corresponding edge is absent.

## Tests

Use a synthetic corpus fixture that already validates as corpus v1. Do not embed published rules text.

- One faction, one datasheet, two keyword rows `Example` and ` example ` with different `model` values. The bundle has one `Keyword` node and two `DATASHEET_HAS_KEYWORD` edges.
- A datasheet with a null faction ref makes `build` exit 4 and leaves an existing `--out` unchanged.
- An inline ability and a shared ability produce one extra `Ability` node for the inline row and one edge onto the shared ability. The shared edge's `attrs` include `type` and `parameter`.
- Two wargear rows with the same `datasheet_id` and `line` and different `line_in_wargear` become one `Wargear` node. `profile_entity_ids` follows numeric `line_in_wargear` order.
- `DATASHEET_CAN_LEAD` runs from `leader_id` to `attached_id`.
- `build` twice yields identical JSONL and postcard bytes.
- `validate` accepts that bundle. Flipping one edge `to` id to a missing node makes `validate` exit 4.
- A manifest with `schema_version` `2` makes `build` exit 3.
