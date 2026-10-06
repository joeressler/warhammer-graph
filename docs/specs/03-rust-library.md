# Rust library: `wh-ask`

This document specifies a read-only Rust library over a graph bundle built by [02-rust-graph.md](02-rust-graph.md). A caller opens the bundle, asks for what it needs, and gets structured rows back. The library does not generate text, does not embed passages, and does not interpret questions. A caller that wants natural language, such as an agent or a chatbot, sits on top of it. A server that exposes these calls to such a caller is a separate crate and is not specified here.

The library is written against the bundle only. It does not download Wahapedia, does not parse CSV, and does not read `entities.jsonl`.

## Crate

- Package name `wh-ask`, library only. There is no binary.
- Rust edition 2021.
- Dependencies: `caseless`, `serde` with `derive`, `serde_json`, and `wh-graph` for the bundle types and the CozoDB reader.
- Every result type derives `Serialize`, so a caller can return it as JSON unchanged.

### Public surface

The public surface is [`Bundle`](#opening-a-bundle), `AskError` and `Candidate`, the result types, and three re-exports from `wh-graph` that appear in results: `Attrs`, `EdgeRecord`, and `BundleManifest`. Nothing else is public. In particular the graph node type, the error constructors, and the id and name resolution helpers are internal, so a caller never depends on `wh-graph`'s node representation.

Every public item has rustdoc, and the crate enforces it with `#![warn(missing_docs)]`. The crate docs include an example that is compiled by the doc test. A change to the public surface keeps `cargo doc` free of warnings.

Calls take borrowed strings. Lists of kinds and seeds are `&[&str]`, so a caller holding `Vec<String>` converts with one `iter().map(String::as_str)`.

## Opening a bundle

`Bundle::open(dir)` reads `manifest.json` and `graph.db` from the bundle directory. It does not read `nodes.jsonl`, `edges.jsonl`, or `passages.jsonl`.

It fails with `AskError::Bundle` when `manifest.json` or `graph.db` is missing or unreadable, when `format_version` is not `2`, when the `graph.db` `meta` `format_version` is not `2`, or when `meta` `corpus_fingerprint` does not equal the manifest's. The message tells the caller to rebuild.

Open reads every node, passage, and edge into memory once and builds three indexes: node kind to passages, `(kind, name key)` to node ids, and node id to the edges that touch it. After open, no call queries CozoDB. A call costs the degree of the nodes it touches, not a scan of the graph. A `Bundle` is read-only.

## Name keys

Name lookup compares name keys, not raw labels. The key of a string is:

1. Casefold it (Unicode default casefold), then replace `’`, `‘`, and `` ` `` with `'`.
2. Drop the words `a`, `an`, and `the`.
3. Trim leading and trailing characters that are not letters, digits, or `'` from each word.
4. Drop words shorter than 2 characters, so a lone `-` vanishes.
5. Join the words with one space.

So `Imotekh Stormlord` and `Imotekh The Stormlord` share a key, `Emperor’s Children` and `emperor's children` share a key, and `Gork’s Klaw - strike` has the key `gork's klaw strike`. A shorter label such as `Stormlord` keeps its own key and stays a different unit.

## Resolving an id or a name

Every call that takes a unit, faction, detachment, keyword, or ability accepts a node id or a name. Resolution is:

1. If the argument is a node id, it is used when the node has the expected kind. A node of another kind is `AskError::Invalid`, naming both kinds.
2. Otherwise the argument's name key is looked up among nodes of the expected kind.
   - No match is `AskError::NotFound` with up to 5 distinct names from [label search](#label-search) of that kind as suggestions.
   - One match is the node.
   - Several matches are `AskError::Ambiguous`. Each candidate carries its `id`, `name`, and a `detail` that tells it apart from the others: a datasheet's faction, a detachment's faction followed by its type in parentheses when it has one (`Chaos Daemons (Boarding Actions)`), and for an enhancement, stratagem, or detachment rule the detachment or faction that owns it. The detail is empty for other kinds. The caller picks an id and calls again.

`units_with_ability` is the one call that accepts several matches: an ability can be a shared core rule and a datasheet's own copy under one name, and all of them are included.

## Label search

`search(query, kinds, limit)` ranks passages by their title and text. `kinds` limits the node kinds searched, and an empty list means all. `limit` is clamped to 1 through 100. An empty or all-punctuation query returns nothing.

1. The query's name key words are the search words, without repeats. The query's phrase is those words joined in order.
2. A passage scores 1000 when its title's name key equals the phrase, 500 when that key contains the phrase, plus 10 for each search word in its casefolded title and 1 for each in its casefolded text.
3. A passage is a strong match when its title key equals or contains the phrase, or when every search word is in its title or text. When any passage is a strong match, the rest are dropped. Otherwise every passage with at least one search word is kept.
4. Results sort by score, highest first, then by shorter title, then by bundle order. Each result is a `NodeRef` and its `score`.

Search does not stem or correct spelling. A misspelled name returns the passages that share words with it, which is what `NotFound` suggestions use.

## Graph navigation

| Call | Returns |
| --- | --- |
| `node_detail(id)` | One node: `id`, `kind`, `name`, `text`, `attrs`, `wahapedia_link`. An unknown id is `NotFound`. |
| `neighbors(id, edge_kinds, limit)` | A `Page`: `total` and up to `limit` `Neighbor` rows. |
| `subgraph(seeds, edge_kinds, depth)` | The reached `nodes`, the `edges` among them, and `truncated`. Seeds are ids, passed as `&[&str]`. |
| `manifest()` | The bundle manifest, including node and edge counts by kind. |

A `Neighbor` is the edge kind, its `attrs`, a `direction` (`outgoing` when the queried node is the edge's `from`, `incoming` when it is the `to`), and the other end as a `NodeRef`.

For both calls an empty `edge_kinds` means every edge kind in the manifest. A kind that is not in the manifest is `NotFound`, and the error lists the known kinds, so a typo is never read as no edges. `neighbors` clamps `limit` to 1 through 500 and reports the uncapped count in `total`. `subgraph` needs at least one seed that exists, clamps `depth` to 1 through 3, and includes every edge of the chosen kinds that joins two reached nodes, not only the edges walked. It returns at most 2000 edges, in bundle order, and sets `truncated` when it cut the list. Its `nodes` are the seeds, then each edge endpoint in order.

Keywords and factions touch thousands of nodes, so callers should pass `edge_kinds` for those.

## Lists

Each list call reads edges from the bundle and returns every matching row in bundle order, never a ranked subset, unless the table says otherwise. A unit argument is a datasheet id or name, resolved as above.

| Call | Source | Returns |
| --- | --- | --- |
| `factions()` | `Faction` nodes | `NodeRef`s by name |
| `detachments(faction)` | `FACTION_HAS_DETACHMENT` | `NodeRef`s by name |
| `find_units(name)` | `Datasheet` nodes | `UnitRef`s: the unit for an id, else every datasheet with that name key, else the 10 closest by label search |
| `roster(subject)` | [Rosters](#rosters) | `Roster`: the `subject` node and its `UnitRef`s |
| `units_with_keyword(keyword)` | `DATASHEET_HAS_KEYWORD` into the keyword | `UnitRef`s |
| `units_with_ability(ability)` | `DATASHEET_HAS_ABILITY` into every ability of that name | `AbilityHolder`s: the unit, its `parameter`, and its `model`; repeats of the same unit, parameter, and model are dropped |
| `unit(unit)` | all of the below | `UnitCard`: the unit, its `text`, `keywords`, `models`, `composition`, `points`, `abilities`, `wargear`, `options`, `leads`, and `led_by` |
| `unit_keywords(unit)` | `DATASHEET_HAS_KEYWORD` | `Keyword`s with `is_faction_keyword` from the edge |
| `unit_text(unit)` | the datasheet node | The datasheet's own text as a string |
| `unit_models(unit)` | `DATASHEET_HAS_MODEL` | `ModelProfile`s |
| `unit_composition(unit)` | `DATASHEET_HAS_COMPOSITION` | `UnitComposition`s, such as `5-10 Boyz` |
| `unit_points(unit)` | `DATASHEET_HAS_COST` | `PointsCost`s |
| `unit_abilities(unit)` | `DATASHEET_HAS_ABILITY` | `Ability`s |
| `unit_wargear(unit)` | `DATASHEET_HAS_WARGEAR` | `Weapon`s, one per profile |
| `unit_options(unit)` | `DATASHEET_HAS_OPTION` | `WargearOption`s |
| `unit_leads(unit)` | `DATASHEET_CAN_LEAD` from the unit | `UnitRef`s the unit can lead, by name |
| `unit_led_by(unit)` | `DATASHEET_CAN_LEAD` to the unit | `UnitRef`s that can lead the unit, by name |
| `unit_enhancements(unit)` | `ENHANCEMENT_APPLIES_TO_DATASHEET` to the unit | `RuleText`s, across all detachments |
| `unit_detachment_rules(unit)` | `DATASHEET_HAS_DETACHMENT_ABILITY` from the unit | `RuleText`s of the detachment rules that name the unit |
| `enhancement_units(enhancement)` | `ENHANCEMENT_APPLIES_TO_DATASHEET` from the enhancement | `UnitRef`s it can be given to |
| `detachment_rule_units(rule)` | `DATASHEET_HAS_DETACHMENT_ABILITY` to the rule | `UnitRef`s the rule names |
| `detachment_stratagems(detachment)` | `DETACHMENT_HAS_STRATAGEM` | `RuleText`s |
| `detachment_rules(detachment)` | `DETACHMENT_HAS_ABILITY` | `RuleText`s |
| `detachment_enhancements(detachment)` | `DETACHMENT_HAS_ENHANCEMENT` | `RuleText`s |
| `faction_abilities(faction)` | `FACTION_HAS_ABILITY` | `RuleText`s, such as an army-wide rule |
| `faction_stratagems(faction)` | `FACTION_HAS_STRATAGEM` | `RuleText`s across all the faction's detachments |
| `faction_enhancements(faction)` | `FACTION_HAS_ENHANCEMENT` | `RuleText`s across all the faction's detachments |

A `UnitRef` is `id`, `name`, `faction` (from the datasheet's `faction_node`), `role`, and `wahapedia_link`. A `NodeRef` is `id`, `kind`, `name`, and `wahapedia_link`.

A leader's direction is the edge's direction. `DATASHEET_CAN_LEAD` runs from the leader to the unit it may join, so `unit_leads` follows it forward and `unit_led_by` follows it backward. A unit the export lists no leader edges for returns an empty list, because the bundle cannot invent edges the export lacks.

### Model profiles

A `ModelProfile` copies the Model node's `attrs`: `movement` (`M`), `toughness` (`T`), `save` (`Sv`), `wounds` (`W`), `leadership` (`Ld`), and `objective_control` (`OC`), each as printed. It adds two normalized fields from `inv_sv` and `inv_sv_descr`:

- `invulnerable_save` is `None` when `inv_sv` is empty or `-`. Otherwise a trailing `*` is dropped, and a bare number gets a `+`, so `4`, `4*`, and `4+` all give `4+`.
- `invulnerable_save_note` is `inv_sv_descr` with a leading `*` and surrounding space removed, such as `Against ranged attacks only`. It is `None` when the description is empty or when there is no invulnerable save.

The note is free text from the export and can restate the condition in several ways. The library does not interpret it.

### Points

A `PointsCost` comes from a `ModelCost` node's text. The last non-empty line is the cost and the lines before it, joined with `, `, are the `description`. `points` is the cost as a whole number, or `None` when it is not one.

### Unit text, composition, and options

A datasheet's own text is the part of its passage that is not a separate node: its loadout, its transport capacity, its leader rules, and its damaged profile, in the order the export prints them. `unit_text` and `UnitCard.text` return it without the leading name line and role line (the role is `UnitRef.role`). A datasheet with none of those has an empty text. The library does not split the text into fields, so transport capacity and the damaged profile are read from it.

A `UnitComposition` is one `DATASHEET_HAS_COMPOSITION` node's text, trimmed, such as `1 Boss Nob` or `9-19 Boyz`. A `WargearOption` is one `DATASHEET_HAS_OPTION` node's text with the export's leading `•` removed, such as `This model's big choppa can be replaced with 1 power klaw.` Both keep the edge order.

### Abilities

An `Ability` is one `DATASHEET_HAS_ABILITY` edge and the node it reaches:

- `scope` is the edge's `type` (`Core`, `Faction`, `Datasheet`, `Wargear`, and so on) and `parameter` and `model` are the edge's values. Empty values are `None`. The export's own labels pass through unchanged, including a few non-English ones.
- `text` is the node's text without its name line, and without the next line when that line equals `scope`.

A shared core ability, such as Feel No Pain, is one node whose per-unit value lives on the edge, so two units show different `parameter`s for the same `id`. An ability belongs only to the datasheet whose edge reaches it.

### Wargear

Each weapon profile is its own node, so a weapon with strike and sweep profiles is two `Weapon`s whose names say which. A `Weapon` has `id`, `name`, `keywords`, the raw `text`, and `stats`.

`stats` is parsed from the first line of the text that has at least 7 words whose second word is `Ranged` or `Melee`, in the order range, type, attacks, skill, strength, AP, damage. Skill is Ballistic Skill for a ranged weapon and Weapon Skill for a melee one. Lines after the stat line, split on commas, are the `keywords`. When no line fits, `stats` is `None` and `text` still holds everything.

### Stratagems, enhancements, and rules

A `RuleText` is `id`, `name`, and `text`, which is the node's text without its name line. A stratagem's text carries its type, cost, turn, and phase lines ahead of its `WHEN`, `TARGET`, and `EFFECT` rules. An enhancement's text carries its cost and detachment lines ahead of its rules. A faction ability's text keeps its flavor legend before the rules, because the legend is part of the node. The library does not split those fields.

An enhancement can be given to several units, and a name can be shared by enhancements in different detachments, so `unit_enhancements` lists every applicable enhancement across all detachments and `enhancement_units` is `Ambiguous` for a shared name until an id is given. The same holds for `detachment_rule_units`.

## Rosters

`roster(subject)` takes a faction or keyword id, or a name. A name matches when a faction's label, or the label of a roster keyword below, occurs whole inside it, so `the Space Marines faction` finds `Space Marines`. An exact label wins, then the longest label, then the earlier node. No match is `NotFound` listing the faction names. An id of another kind is `Invalid`. The result's units are datasheets only.

A plain faction is that faction's datasheets on `FACTION_HAS_DATASHEET`, in edge order.

A Space Marine chapter is a faction keyword that shares a datasheet with the faction keyword `Adeptus Astartes` and is not that keyword. Its roster is every datasheet that has it as a faction keyword, then every datasheet whose only faction keyword is `Adeptus Astartes`. Another chapter's datasheets stay out.

A Chaos Space Marine chapter is a faction whose label is `Death Guard`, `World Eaters`, `Thousand Sons`, or `Emperor's Children`. Its roster is its `FACTION_HAS_DATASHEET` datasheets, then every datasheet whose only faction keyword is `Heretic Astartes`.

A Legiones Daemonica god is the keyword `Khorne`, `Tzeentch`, `Nurgle`, or `Slaanesh`. Its roster is every datasheet whose only faction keyword is `Legiones Daemonica` and that either has that god keyword or has none of the four. The keyword `Legiones Daemonica` itself lists every datasheet whose only faction keyword is `Legiones Daemonica`.

A datasheet's faction keywords are its `DATASHEET_HAS_KEYWORD` edges whose `is_faction_keyword` is `true`, ignoring keywords with an empty label.

## Errors

`AskError` has four variants.

| Variant | When |
| --- | --- |
| `Bundle` | The bundle cannot be opened or its versions do not match. |
| `NotFound` | A name, id, or edge kind matches nothing. Carries up to 5 suggestions, or the faction names, or the known edge kinds. |
| `Ambiguous` | A name matches several nodes. Carries each candidate's id, name, and a detail that tells it apart. |
| `Invalid` | An id has the wrong kind, or an argument is unusable, such as an empty seed list. |

`AskError` implements `Display` and `std::error::Error`, and its messages say what to do next.

## Limits

| Call | Limit |
| --- | --- |
| `search` | `limit` 1 through 100 |
| `neighbors` | `limit` 1 through 500 |
| `subgraph` | depth 1 through 3, at most 2000 edges |
| `find_units` | 10 fuzzy matches |
| lists | none; every matching row is returned |

## Out of scope

This spec does not cover generating answers, parsing questions, embeddings, army building, army points totals, detachment validation, or serving these calls over a network. Those belong to a caller.

## Tests

Integration tests build a small bundle in a temporary directory with `write_graph_db`, so they need no real data and no network.

- A faction roster is every datasheet it has. A chapter roster adds the generic parent datasheets and omits another chapter's. A daemon god roster keeps godless daemons and drops another god's.
- A roster takes an id or a name with extra words, rejects an id of another kind, and lists the factions when nothing matches.
- A name shared by two datasheets is `Ambiguous` with each faction until an id is given. A misspelled name suggests close names. Detachments of one name in one faction are told apart by type (`Space Marines (Boarding Actions)`), and shared enhancement names name their detachment.
- A unit card gathers its text, keywords, models, composition, points, abilities, wargear, options, and both lead directions.
- A datasheet's own text keeps its transport capacity and damaged profile without the name and role lines. Composition and options are typed, and an option loses its bullet.
- Enhancements are found from the unit and from the enhancement. A detachment rule and the units it names are linked both ways. A faction has its own abilities and stratagems.
- A model's invulnerable save is `5+` with its note for the printed `5`, and `None` for `-`.
- An ability belongs to the unit whose edge reaches it, with that edge's `parameter`, and a shared ability shows each unit's own value.
- Strike and sweep are separate weapons with parsed stats and keywords. A passage with no stat line keeps its text and has no `stats`.
- Lead direction follows the edge in both calls.
- Units are found by ability and by keyword.
- A detachment lists its stratagems, enhancements, and rules.
- Search ranks an exact title first and filters by kind. `neighbors` reports direction, caps its page, and rejects an unknown edge kind. `subgraph` includes edges between reached nodes.
- Results serialize to JSON, and opening a missing bundle is a `Bundle` error.
