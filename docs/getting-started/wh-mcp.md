# Getting started with wh-mcp

`wh-mcp` is a program that lets an AI agent, such as Claude, look things up in the graph bundle built by [wh-graph](wh-graph.md). The agent starts it, asks questions through 15 tools (a unit's stats, a faction's roster, a detachment's stratagems, and so on), and gets structured JSON back. It uses the [wh-ask](wh-ask.md) library for every answer and generates no text of its own.

The frozen contract is [04-mcp-server.md](../specs/04-mcp-server.md). This page is the install and usage guide.

```text
AI agent  <-- JSON over stdin/stdout -->  wh-mcp  -->  wh-ask library  -->  ./bundle
```

## Build

You need Rust 1.88 or newer, because the MCP toolkit requires it. The other crates still build on 1.83. From the repository root:

```bash
cargo build -p wh-mcp --release
```

The program is `target/release/wh-mcp` (`wh-mcp.exe` on Windows). It needs a bundle first. Build one with [wh-graph](wh-graph.md):

```bash
cargo run -p wh-graph -- build --corpus ./corpus --out ./bundle
```

Rebuild the bundle after pulling a change to `wh-graph`. `wh-mcp` rejects a bundle whose version or corpus fingerprint does not match, and exits with code 3 and a message that says to rebuild.

## Run it

```bash
wh-mcp --bundle ./bundle
```

You will normally not run it yourself. An AI client starts it. Run by hand it prints one line to stderr (`26406 nodes and 165697 edges loaded`) and then waits for JSON-RPC on stdin. `WH_BUNDLE=./bundle wh-mcp` works too, and `--bundle` wins when both are set. Stdout is reserved for the protocol, so everything meant for a person goes to stderr.

Loading takes about a second. After that each tool answers in milliseconds.

## Connect Claude Code

Use absolute paths, because the client starts the program from its own directory:

```bash
claude mcp add wh -- /full/path/to/target/release/wh-mcp --bundle /full/path/to/bundle
```

On Windows:

```powershell
claude mcp add wh -- C:\path\to\warhammer-graph\target\release\wh-mcp.exe --bundle C:\path\to\warhammer-graph\bundle
```

Then ask a rules question in Claude Code, such as "What invulnerable save does Angron have, and what are his weapon profiles?"

## Connect Claude Desktop

Add an entry to `claude_desktop_config.json` and restart the app:

```json
{
  "mcpServers": {
    "wh": {
      "command": "C:\\path\\to\\warhammer-graph\\target\\release\\wh-mcp.exe",
      "args": ["--bundle", "C:\\path\\to\\warhammer-graph\\bundle"]
    }
  }
}
```

## Connect a local model (Ollama)

Ollama runs models but is not an MCP client, so it needs a host in between. The [Ollama host guide](ollama-host.md) has a small Python host that does this, tests for it, and a comparison of five local models on eleven rules questions.

## The tools

| Tool | Use it for |
| --- | --- |
| `get_unit` | One unit's datasheet: stats, invulnerable save, points, abilities, wargear with weapon stats, options, leaders, transport. |
| `get_roster` | Every unit a faction, a chapter (`Ultramarines`), or a daemon god (`Khorne`) can field. |
| `get_detachment` | A detachment's stratagems, rules, and enhancements. |
| `get_faction_rules` | A faction's army-wide abilities, and optionally all its stratagems and enhancements. |
| `find_units`, `search` | Finding the right name or id. |
| `list_factions`, `list_detachments` | The names to use elsewhere. |
| `units_with_keyword`, `units_with_ability` | Every unit with a keyword or an ability, such as Feel No Pain 5+. |
| `units_for_rule` | The units an enhancement or a detachment rule applies to. |
| `get_node`, `get_neighbors`, `get_subgraph`, `bundle_info` | Raw graph access, for anything the others do not cover. |

Names must be exact, though case, `the`, and apostrophe style do not matter. `get_unit`, `get_detachment`, and `get_faction_rules` accept a `sections` list to return only some parts.

## What errors look like

A failed call is a normal reply flagged as an error, so the agent can read it and retry:

- A name that matches nothing lists close names, nearest in spelling first. `get_unit` for "Taxtical Squad" suggests `Tactical Squad` first.
- A name several things share lists their ids and what tells them apart. "Daemonic Incursion" is two detachments in one faction, offered as `Chaos Daemons` and `Chaos Daemons (Boarding Actions)`. The reply also carries an `instruction` telling the agent not to pick one silently: say that more than one exists, then ask which is meant or call again with each id.
- A bad argument, such as an unknown section, says which values are valid.

Replies that list things start with a `summary` that states the count (`Khorne can field 21 units.`), because models are poor at counting long lists. Stratagems carry their command point cost, turn, and phase as separate fields.

A list that was cut says so. `units_with_keyword` for `Infantry` has 774 units, returns 200 by default, and reports `total: 774` and `truncated: true`. Pass `limit` for more, up to 1000.

## Try it without an AI client

You can speak the protocol by hand. In Git Bash, from the repository root:

```bash
{
  echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"hand","version":"0"}}}'
  echo '{"jsonrpc":"2.0","method":"notifications/initialized"}'
  echo '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"get_unit","arguments":{"unit":"Angron","sections":["models"]}}}'
  sleep 2
} | target/release/wh-mcp --bundle bundle
```

Each reply is one line of JSON. The `result.content[0].text` field holds the tool's JSON.

## Troubleshooting

- **The client says the server failed to start.** Run the same command in a terminal. Exit code 3 means the bundle path is wrong or the bundle needs a rebuild, and exit code 2 means the arguments are wrong.
- **The client shows a JSON parse error.** Something printed to stdout. The test suite fails if that ever happens, so this should only occur with a modified program.
- **A unit comes back with no leaders.** Some units, such as Angron, have no leader entries in the Wahapedia export. An empty list means no data, not an error.
- **Invulnerable saves are missing.** The bundle was built before `inv_sv` was added. Rebuild it.

## Tests

```bash
cargo test -p wh-mcp
```

The tests start the real program and talk to it over stdio against a small bundle built in a temporary directory. They need no real data and no network.
