# Rust graph builder: `wh-graph`

This document specifies the crate that reads a corpus v1 directory and writes a CozoDB graph bundle. The corpus contract is [01-python-cli.md](01-python-cli.md) and [corpus-v1.schema.json](../schemas/corpus-v1.schema.json). The library in [03-rust-library.md](03-rust-library.md) reads the bundle defined here.

`wh-graph` does not download Wahapedia and does not parse CSV. It rejects a manifest whose `schema_version` is not `1`.

## Crate

- Package name `wh-graph`, binary `wh-graph`.
- Rust edition 2021.
- Dependencies: `clap` (derive), `cozo` 0.7.x with `storage-sqlite`, `storage-sqlite-src`, and `graph-algo` (no RocksDB), `serde` with `derive`, `serde_json`, `sha2`.
- Graph store: CozoDB SQLite file `graph.db`.

The JSONL files are the byte-stable contract. `graph.db` is a derived store of the same nodes, edges, and passages. A second build of the same corpus yields identical JSONL bytes. SQLite page layout is not required to be byte-identical.

## Commands

No command prompts. Each subcommand has `--help` with examples. Missing flags exit 2 and print one working invocation.

```text
wh-graph build --corpus ./corpus --out ./bundle
wh-graph build --corpus ./corpus --out ./bundle --dry-run --output json
wh-graph validate --bundle ./bundle
```

`--output text|json` defaults to `text`. `--dry-run` on `build` parses, builds, and prints counts, and does not write `--out`.

`build` writes a temporary directory and renames it onto `--out` only after validation passes. A failed build leaves an existing bundle in place. A second build of the same corpus overwrites the bundle with identical `nodes.jsonl`, `edges.jsonl`, and `passages.jsonl` bytes.

Text success:

```text
bundle: ./bundle
format_version: 2
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
| 3 | Corpus or bundle cannot be read, JSON is malformed, `schema_version` is not `1`, or `format_version` is not `2`. |
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

`DATASHEET_CAN_LEAD` points from the leader datasheet to the datasheet it may join. The library walks it forward for what a leader can lead and backward for who can lead a unit.

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

Datasheet `text` does not repeat child models, weapons, or abilities. Those are their own passages. The library pulls them by walking edges.

`attrs` copies the string fields that are not already the whole text and that a filter might need:

- Datasheet: `role`, `virtual`, `faction_id` (raw field, possibly joined as `attrs.faction_node` holding the corpus id or `""` when the ref is null).
- Model: `line`, `M`, `T`, `Sv`, `inv_sv`, `inv_sv_descr`, `W`, `Ld`, `OC`. Values are the export's printed strings, with HTML stripped from `inv_sv_descr`. `inv_sv` is `-` when the model has no invulnerable save, and is otherwise a bare number such as `4`, sometimes followed by `*`. `inv_sv_descr` is a free-text condition and is often empty. The bundle does not normalize these; [03-rust-library.md](03-rust-library.md) does.
- Wargear: `line`, `profile_entity_ids`.
- Keyword: `normalized` (the casefolded key).
- Detachment: `type`, as printed. It is empty for a standard detachment and `Boarding Actions` for a Boarding Actions variant. A faction can have two detachments of the same name that differ only by this.
- Edges: only the columns listed in the edge table.

Keep `attrs` small. The full raw strings remain available by going back to the corpus; the bundle does not copy every CSV column onto every node.

## Passages

`passages.jsonl` has one object per node, in node order. Key order: `node_id`, `title`, `text`, `wahapedia_link`.

- `node_id` equals the node `id`.
- `title` equals the node `label`.
- `text` equals the node `text`.
- `wahapedia_link` equals the node `source_url`.

The graph builder does not compute embeddings, and neither does the library in [03-rust-library.md](03-rust-library.md). Passage search is label search over `title` and `text`.

## Bundle manifest

`manifest.json` key order:

```json
{
  "format_version": 2,
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

## Cozo store

`graph.db` is a CozoDB SQLite file opened with engine `sqlite`. Relations:

```text
node {id => kind, label, text, source_url, attrs_json}
graph_edge {seq => id, kind, from_id, to_id, attrs_json}
passage {seq => node_id, title, text, link}
meta {key => value}
```

`seq` is the JSONL line order, starting at 0. `attrs_json` is the canonical attrs JSON string. `meta` holds `format_version`, `corpus_fingerprint`, and the SHA-256 hex of `nodes.jsonl` and `passages.jsonl`.

`validate` loads `graph.db` and the JSONL files and checks:

- set equality of `(node.id, node.kind, node.label, node.text, node.source_url)`
- `seq` order equality of `(edge.id, edge.kind, edge.from, edge.to, canonical attrs)`
- `seq` order equality of passage rows
- `meta.format_version` and `meta.corpus_fingerprint` match `manifest.json`

A missing or unreadable `graph.db` fails validate. JSONL-only bundles are not accepted.

## Validation

`build` and `validate` both enforce:

- `format_version` is `2` and `corpus_schema_version` is `1`.
- Every edge `from` and `to` is a node id.
- Every `Datasheet` node is the target of exactly one `FACTION_HAS_DATASHEET` edge. A datasheet whose faction ref was null fails the build. Exit 4. Do not publish the bundle.
- No two `Keyword` nodes share `attrs.normalized`.
- `passage_count` equals the node count, and each passage's `node_id`, `title`, `text`, and `wahapedia_link` match that node.
- Cozo payloads match JSONL, as above.
- Node and edge counts on the manifest match the files.

A corpus warning for an unresolved non-faction link does not fail the build. The corresponding edge is absent.

## Tests

Use a synthetic corpus fixture that already validates as corpus v1. Do not embed published rules text.

- One faction, one datasheet, two keyword rows `Example` and ` example ` with different `model` values. The bundle has one `Keyword` node and two `DATASHEET_HAS_KEYWORD` edges.
- A datasheet with a null faction ref makes `build` exit 4 and leaves an existing `--out` unchanged.
- An inline ability and a shared ability produce one extra `Ability` node for the inline row and one edge onto the shared ability. The shared edge's `attrs` include `type` and `parameter`.
- Two wargear rows with the same `datasheet_id` and `line` and different `line_in_wargear` become one `Wargear` node. `profile_entity_ids` follows numeric `line_in_wargear` order.
- `DATASHEET_CAN_LEAD` runs from `leader_id` to `attached_id`.
- `build` twice yields identical JSONL bytes.
- `validate` accepts that bundle. Flipping one edge `to` id to a missing node makes `validate` exit 4.
- A manifest with `schema_version` `2` makes `build` exit 3.
