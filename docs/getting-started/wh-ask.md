# Getting started with wh-ask

`wh-ask` is a Rust library that reads a graph bundle built by [wh-graph](wh-graph.md). You open the bundle once, then call it to look up a unit, search passages, walk the graph, or list a roster, a unit's wargear, or a detachment's stratagems. Every call returns typed rows that serialize to JSON. The library generates no text, so it needs no language model, no model files, and no GPU. A caller such as an agent or a chatbot does the reasoning and asks the library for facts.

The library does not download Wahapedia, does not parse CSV, and does not use the network. After the bundle is on disk, it reads only the local filesystem.

The frozen contract is [03-rust-library.md](../specs/03-rust-library.md). This page is the usage guide.

## Install

You need Rust 1.83 or newer and nothing else. `wh-ask` is a library with no binary, and it is not published to crates.io. Depend on it by path from another crate in a workspace:

```toml
[dependencies]
wh-ask = { path = "../warhammer-graph/crates/wh-ask" }
```

Build and test it from the repository root:

```bash
cargo build -p wh-ask
cargo test -p wh-ask
```

## Open a bundle

Build a bundle first with `wh-graph build`. Then:

```rust
use std::path::Path;
use wh_ask::Bundle;

let bundle = Bundle::open(Path::new("./bundle"))?;
println!("{} nodes, {} edges", bundle.manifest().node_count, bundle.manifest().edge_count);
```

`open` reads `manifest.json` and `graph.db`, loads every node, passage, and edge into memory, and builds the lookup indexes. On the full 10th-edition bundle that takes about a second in a release build. After that no call touches the disk, and a call costs about as much as the number of edges on the nodes it uses. Open once and keep the `Bundle`.

A `Bundle` is read-only, and it is `Send` and `Sync`, so wrap it in an `Arc` to answer many requests at once.

`open` fails with `AskError::Bundle` when a file is missing, when `format_version` is not `2`, or when `graph.db` does not match `manifest.json`. The message says to rebuild.

## Ids and names

Every `unit`, `faction`, `detachment`, `keyword`, or `ability` argument takes a node id (`10ed:datasheet:000002621`) or an exact name (`Angron`). Case, `a`/`an`/`the`, and curly versus straight apostrophes do not matter. A shorter or misspelled name is not a match.

- A name nothing matches is `AskError::NotFound`, which suggests close names.
- A name several nodes share is `AskError::Ambiguous`, which lists each candidate's id and faction. Call again with the id.

```rust
match bundle.unit("Taxtical Squad") {
    Err(wh_ask::AskError::NotFound { suggestions, .. }) => println!("try: {suggestions:?}"),
    _ => {}
}
```

Use `find_units(name)` to turn a name into ids first when you want to choose.

## Find things

```rust
// Label search over titles and text. Empty `kinds` searches every kind.
let hits = bundle.search("lone operative", &["Ability"], 10);

// Datasheets with that exact name in any faction, else the closest names.
let units = bundle.find_units("Dreadnought");

let factions = bundle.factions();
let detachments = bundle.detachments("Adeptus Custodes")?;
```

`search` ranks an exact title first, then a title that contains your words, then passages that have all of them. It is label search, not semantic search, so use the words the rules use.

## Lists

| Call | Returns |
| --- | --- |
| `roster(faction \| chapter \| daemon god)` | Every unit it can field. A chapter or a daemon god also gets the generic parent units. |
| `units_with_keyword(keyword)` | Every unit with that keyword. |
| `units_with_ability(ability)` | Every unit with the ability, and each unit's value for it, such as `5+` for Feel No Pain. |
| `unit(unit)` | One `UnitCard` with everything below, in a single call. |
| `unit_text(unit)` | The datasheet's own text: loadout, transport capacity, leader rules, and damaged profile. |
| `unit_models(unit)` | Move, Toughness, Save, invulnerable save, Wounds, Leadership, and Objective Control per model. |
| `unit_composition(unit)` | How many models, such as `1 Boss Nob` and `9-19 Boyz`. |
| `unit_points(unit)` | The printed points lines, such as `5 models` for 90. |
| `unit_abilities(unit)` | The unit's own abilities, with scope, parameter, and rules text. |
| `unit_wargear(unit)` | One entry per weapon profile, with parsed stats and keywords. |
| `unit_options(unit)` | The wargear options, such as swapping one weapon for another. |
| `unit_keywords(unit)` | Keywords, marking the faction keywords. |
| `unit_leads(unit)` | Units this one can lead. |
| `unit_led_by(unit)` | Leaders that can join this unit. |
| `unit_enhancements(unit)` | Every enhancement the unit can be given, across all detachments. |
| `unit_detachment_rules(unit)` | The detachment rules that name this unit. |
| `enhancement_units(enhancement)` | The units an enhancement can be given to. |
| `detachment_rule_units(rule)` | The units a detachment rule names, such as who gets Idols of Khorne. |
| `detachment_stratagems`, `detachment_rules`, `detachment_enhancements` | A detachment's rules text. |
| `faction_abilities`, `faction_stratagems`, `faction_enhancements` | A faction's army-wide abilities, and every stratagem and enhancement it has. |

```rust
let card = bundle.unit("Angron")?;
let model = &card.models[0];
println!("Sv {} inv {:?}", model.save, model.invulnerable_save);   // Sv 2+ inv Some("4+")
for weapon in &card.wargear {
    if let Some(s) = &weapon.stats {
        println!("{}: A{} S{} AP{} D{}", weapon.name, s.attacks, s.strength, s.ap, s.damage);
    }
}
```

Things worth knowing:

- **Strike and sweep** are two `Weapon` entries whose names end in `- strike` and `- sweep`.
- **A unit's own abilities only.** Another unit's copy of a shared rule is that unit's. Shared rules such as Deadly Demise carry each unit's value in `parameter`.
- **Invulnerable saves** come back as `4+`, or `None` when the model has none. A condition such as `Against ranged attacks only` is in `invulnerable_save_note`.
- **Leaders** follow the export. A unit the export gives no leader edges, such as Angron, returns an empty list.
- **Rosters** match a name when the whole faction or chapter label is inside it, so `Ultramarines`, `the Ultramarines`, and `Ultramarines units` all work.
- **Transport and the damaged profile** are in the datasheet's text, not in separate fields. Read them from `UnitCard.text` or `unit_text`.
- **Same-named detachments.** A faction can have two detachments of one name, such as the standard `Daemonic Incursion` and its Boarding Actions variant. The ambiguity error tells them apart (`Chaos Daemons` versus `Chaos Daemons (Boarding Actions)`), and you call again with the id it offers.
- **Stratagem and enhancement text** includes their cost, turn, and phase lines ahead of the rules. The library does not split those into fields.

## Walk the graph

For anything the lists do not cover, use the raw graph:

```rust
let detail = bundle.node_detail("10ed:datasheet:000002621")?;       // text, attrs, link

// Nodes one edge away. Name the edge kinds for big nodes such as keywords.
let page = bundle.neighbors(&detail.id, &["DATASHEET_HAS_KEYWORD"], 50)?;
for n in page.items {
    println!("{:?} {}", n.direction, n.node.name);
}

// Everything within one edge, as nodes and the edges among them.
let graph = bundle.subgraph(&[detail.id.as_str()], &["DATASHEET_HAS_ABILITY"], 1)?;
```

`neighbors` caps a page at 500 and reports the full count in `total`. `subgraph` stops at 2000 edges and sets `truncated`. An unknown edge kind is an error that lists the known ones. `bundle.manifest().edge_counts_by_kind` shows what exists.

Mind the depth. Two edges out from a unit through `DATASHEET_HAS_ABILITY` reaches every unit that shares one of its core abilities, such as Deep Strike, which is about a thousand nodes. Keep depth at 1 unless you want that.

## Return it as JSON

Every result derives `Serialize`:

```rust
let json = serde_json::to_string(&bundle.unit_points("Warboss")?)?;
// [{"id":"10ed:datasheet_model_cost:000000001:1","description":"1 model","points":75}]
```

## Tests

```bash
cargo test -p wh-ask
```

The tests build a small bundle in a temporary directory. They need no real data and no network.
