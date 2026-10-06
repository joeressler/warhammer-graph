# MCP server: `wh-mcp`

This document specifies a program that serves a graph bundle to an AI agent over the Model Context Protocol (MCP). It is a thin layer over the library in [03-rust-library.md](03-rust-library.md): each tool calls library functions, and the library does the work. The server adds names, defaults, and size caps that suit a model's context window, and it generates no text.

## Crate

- Package name `wh-mcp`, binary `wh-mcp`, Rust edition 2021, minimum Rust 1.88 (the MCP SDK requires it; the other crates keep 1.83).
- Dependencies: `rmcp` 3.5.0 (the official Rust MCP SDK) with `server`, `macros`, and `transport-io`, `tokio`, `serde`, `serde_json`, and `wh-ask`. Every version is pinned exactly.
- Tools are declared with the SDK's `#[tool]` macros. Each tool's input type derives `Deserialize` and `JsonSchema`, so the input schema an agent sees is generated from the same type that reads the arguments, and the doc comments on those types are the descriptions the agent reads.

## Running

```text
wh-mcp --bundle ./bundle
WH_BUNDLE=./bundle wh-mcp
wh-mcp --help
wh-mcp --version
```

`--bundle` wins over `WH_BUNDLE`. The server opens the bundle once, before the handshake, and shares that one `Bundle` between all requests.

The transport is stdio: one JSON-RPC message per line on stdin and stdout. **Stdout carries only protocol.** The server never prints anything else to stdout while serving, and it writes its one status line (`N nodes and M edges loaded`) to stderr. A client that starts the server owns both pipes, and the server exits with code 0 when the client closes stdin.

| Code | When |
| --- | --- |
| 0 | `--help`, `--version`, or the client closed stdin. |
| 1 | The MCP session could not start or ended with an error. |
| 2 | No bundle was given, or an argument is unknown. Stderr shows the usage. |
| 3 | The bundle cannot be opened, as in [03-rust-library.md](03-rust-library.md#opening-a-bundle). |

## Protocol

The server answers `initialize` with its name `wh-mcp`, its version, the `tools` capability, and a short `instructions` text, and it accepts the protocol version the client proposes when it supports it. Only tools are offered. There are no resources and no prompts.

The instructions are delivered once at connect time. They tell the agent to start with `get_unit` for a unit and `get_roster` for a faction, that names must be exact, that a failed name lookup lists close names, that an ambiguous name lists ids to retry with, that an empty list means the source data has none and not that the call failed (some units, such as Angron, have no leader entries in the export), and to credit Wahapedia.

All 15 tools are annotated read-only. None changes anything.

## Replies

A successful call returns one text block holding compact JSON. The shapes are the library's result types from [03-rust-library.md](03-rust-library.md), serialized unchanged, except where the table below adds a wrapper.

A call that fails returns a **tool-level error**: a normal reply flagged `isError`, with a JSON text block that always has an `error` message. The agent reads it and can retry. The server never returns a protocol-level error for a bad request, because clients render those without the message.

| Failure | The `error` text block also has |
| --- | --- |
| A name that matches nothing (`NotFound`) | `suggestions`: close names, or the faction names for a roster, or the known edge kinds for an edge-kind typo. |
| A name that matches several nodes (`Ambiguous`) | `candidates`: each with `id`, `name`, and a `detail` that tells it apart. Call again with one id. |
| An id of the wrong kind, or an unusable argument (`Invalid`) | Only `error`. |
| A malformed or unknown argument | The SDK's own message in plain text (not JSON), such as ``unknown variant `bogus`, expected one of `text`, `keywords`, ...``. It lists every valid value. |

A bundle that fails to open is a startup failure and exits with code 3. It is not a reply.

## Tools

| Tool | Inputs | Returns | Library calls |
| --- | --- | --- | --- |
| `search` | `query`, `kinds?`, `limit?` | Array of `SearchHit` | `search` |
| `find_units` | `name` | Array of `UnitRef` | `find_units` |
| `list_factions` | none | Array of `NodeRef` | `factions` |
| `list_detachments` | `faction` | Array of `NodeRef` | `detachments` |
| `get_roster` | `subject` | `{subject, units}` | `roster` |
| `get_unit` | `unit`, `sections?` | `{unit, <sections>}` | `unit` and `unit_enhancements`, `unit_detachment_rules` |
| `get_detachment` | `detachment`, `sections?` | `{<sections>}` | `detachment_stratagems`, `detachment_rules`, `detachment_enhancements` |
| `get_faction_rules` | `faction`, `sections?` | `{<sections>}` | `faction_abilities`, `faction_stratagems`, `faction_enhancements` |
| `units_with_keyword` | `keyword`, `limit?` | `{total, truncated, items}` of `UnitRef` | `units_with_keyword` |
| `units_with_ability` | `ability`, `limit?` | `{total, truncated, items}` of `AbilityHolder` | `units_with_ability` |
| `units_for_rule` | `kind`, `name` | Array of `UnitRef` | `enhancement_units` or `detachment_rule_units` |
| `get_node` | `id` | `NodeDetail` | `node_detail` |
| `get_neighbors` | `id`, `edge_kinds?`, `limit?` | `{total, items}` of `Neighbor` | `neighbors` |
| `get_subgraph` | `seeds`, `edge_kinds?`, `depth?` | `{nodes, edges, truncated}` | `subgraph` |
| `bundle_info` | none | The bundle manifest | `manifest` |

Unit, faction, detachment, keyword, and ability arguments take an id or an exact name, resolved as in [03-rust-library.md](03-rust-library.md#resolving-an-id-or-a-name). `units_for_rule` takes `kind` `enhancement` or `detachment_rule`.

### Sections

`get_unit`, `get_detachment`, and `get_faction_rules` take an optional `sections` list so an agent can ask for only what it needs. A value outside the list is an error that names the valid values. An omitted or empty list means the default.

| Tool | Sections | Default |
| --- | --- | --- |
| `get_unit` | `text`, `keywords`, `models`, `composition`, `points`, `abilities`, `wargear`, `options`, `leads`, `led_by`, `enhancements`, `detachment_rules` | The full datasheet: everything except `enhancements` and `detachment_rules`, which are long and are returned only when asked for. |
| `get_detachment` | `stratagems`, `rules`, `enhancements` | All three. |
| `get_faction_rules` | `abilities`, `stratagems`, `enhancements` | `abilities` only, because a faction's stratagems and enhancements are long lists. |

`get_unit` always returns `unit` (the `UnitRef`) and then one key per section, named as above. The name is resolved once, so an unknown or ambiguous unit is a single error.

## Size caps

A model pays for every byte it reads, so the server uses smaller defaults and caps than the library's. A list that was cut says so.

| Tool | Default | Cap |
| --- | --- | --- |
| `search` | 10 results | 50 |
| `get_neighbors` | 50 neighbors | 200, and `total` is always the full count |
| `get_subgraph` | depth 1 | depth 2, 100 edges, 150 nodes, and `truncated` is set when cut |
| `units_with_keyword`, `units_with_ability` | 200 rows | 1000, and `total` is always the full count, with `truncated` set when cut |

A `limit` outside its range is raised to 1 or lowered to the cap, never rejected. Rosters, unit datasheets, and detachment lists are never cut, because they are bounded by the data (a roster is at most a few hundred units).

## Out of scope

HTTP transport, authentication, hosting, resources, prompts, and semantic search are not part of this server. A change to the tool list, a default, or a cap updates this document in the same change.

## Tests

The tests start the real binary and speak JSON-RPC to it over stdio, against a small bundle built in a temporary directory. They need no real data and no network.

- The handshake names the server and carries the instructions. All 15 tools are listed, each read-only, each with a description and an object input schema.
- `get_unit` returns the full datasheet by default, picks and unlocks sections, treats an empty list as the default, and rejects an unknown section with a message that lists the valid ones.
- A library error is a tool-level error: a typo suggests names, a shared name lists candidates with their details and the offered id works, and an id of the wrong kind is rejected.
- Rosters, factions, detachments, faction rules, keyword, ability, and rule lookups, search, and the graph tools all return the shapes above.
- Limits are clamped, long lists report `total` and `truncated`, and a huge subgraph is cut.
- Every line the server writes to stdout is a JSON-RPC message. A stray print makes the suite fail. Closing stdin ends the server with code 0.
- The command line reports a missing or unreadable bundle on stderr with codes 2 and 3 and keeps stdout empty.
