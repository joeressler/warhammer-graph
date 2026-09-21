# Python CLI: `wh-corpus`

This document specifies the command that downloads the public Wahapedia export and writes corpus v1. The JSON contract is [corpus-v1.schema.json](../schemas/corpus-v1.schema.json). The Rust graph builder in [02-rust-graph.md](02-rust-graph.md) reads only that corpus. It does not know CSV filenames or Wahapedia URLs.

Corpus v1 is a directory of `manifest.json` and `entities.jsonl`. Field values stay strings, including booleans and HTML. Foreign keys are ids, not nested objects. Rows with a missing join stay in the file and produce a warning.

## Source

Wahapedia does not publish a request/response API. The publisher's data-export page states that the export is a set of CSV files linked by identifiers, refreshed when the site changes. For this spec the source is:

- Base URL: `https://wahapedia.ru/wh40k10ed/`
- Workbook: `https://wahapedia.ru/wh40k10ed/Export%20Data%20Specs.xlsx`
- Export page: `https://wahapedia.ru/wh40k10ed/the-rules/data-export/`

Files are UTF-8, pipe-delimited, and described by that workbook. The header catalog below was checked against the live 10th-edition files. Where the workbook and the live header disagree, the live header is normative. The known disagreement is `Datasheets_leader.csv`: the workbook calls the columns `datasheet_id` and `attached_datasheet_id`; the file on the server sends `leader_id` and `attached_id`. `leader_id` is the leader datasheet. `attached_id` is the datasheet it may attach to.

`Detachments_chapter_dp.csv` is not part of the 10th-edition export. The workbook does not list it, and the URL returns 404. Corpus v1 does not include it. A later edition that adds tables needs a new `schema_version`.

The CLI downloads data when the user runs it. The repository does not store the export or Games Workshop rules text. Tests use the synthetic rows in [Tests](#tests).

When the tool is published, its help text and any about screen include the phrase `powered by Wahapedia`.

## Edition registry

| Key | Game | Base URL | Status |
| --- | --- | --- | --- |
| `10ed` | Warhammer 40,000 | `https://wahapedia.ru/wh40k10ed/` | Implemented. This is the only edition corpus v1 accepts. |
| `11ed` | Warhammer 40,000 | `https://wahapedia.ru/wh40k11ed/` | Reserved. The command rejects it. |
| `aos` | Age of Sigmar | none | Reserved. The command rejects it. |

`--edition` is required on `fetch`, `assemble`, and `export`. The only accepted value is `10ed`. Any other value exits 2 and prints the implemented edition plus a copy-paste command. There is no interactive prompt.

## Language and package

- Python 3.12.
- Package name `wh-corpus`. Console script `wh-corpus`.
- Libraries: Typer for the CLI, httpx for HTTP, Pydantic v2 for the in-memory corpus, and the JSON Schema at `docs/schemas/corpus-v1.schema.json` checked with an implementation of draft 2020-12.

Suggested modules, so a later implementation has one place for each rule:

- `catalog.py` holds the edition registry and the header sets in this document.
- `fetch.py` downloads into the cache.
- `parse.py` turns CSV bytes into logical records.
- `assemble.py` assigns ids, refs, and warnings.
- `validate.py` applies the schema and the extra checks in [Validation](#validation).

## File catalog

Header comparison uses the set of names, not column order. Assemble maps fields by header name and writes `fields` keys in the order listed here.

Each live file ends with a trailing `|`. Splitting the header on `|` therefore yields one trailing empty name. Strip exactly one trailing empty name before comparing. Any other empty name, any missing name, or any extra name is header drift.

| File | Entity table | Columns, in `fields` order |
| --- | --- | --- |
| `Last_update.csv` | none | `last_update` |
| `Factions.csv` | `faction` | `id`, `name`, `link` |
| `Source.csv` | `source` | `id`, `name`, `type`, `edition`, `version`, `errata_date`, `errata_link` |
| `Datasheets.csv` | `datasheet` | `id`, `name`, `faction_id`, `source_id`, `legend`, `role`, `loadout`, `transport`, `virtual`, `leader_head`, `leader_footer`, `damaged_w`, `damaged_description`, `link` |
| `Datasheets_abilities.csv` | `datasheet_ability` | `datasheet_id`, `line`, `ability_id`, `model`, `name`, `description`, `type`, `parameter` |
| `Datasheets_keywords.csv` | `datasheet_keyword` | `datasheet_id`, `keyword`, `model`, `is_faction_keyword` |
| `Datasheets_models.csv` | `datasheet_model` | `datasheet_id`, `line`, `name`, `M`, `T`, `Sv`, `inv_sv`, `inv_sv_descr`, `W`, `Ld`, `OC`, `base_size`, `base_size_descr` |
| `Datasheets_options.csv` | `datasheet_option` | `datasheet_id`, `line`, `button`, `description` |
| `Datasheets_wargear.csv` | `datasheet_wargear` | `datasheet_id`, `line`, `line_in_wargear`, `dice`, `name`, `description`, `range`, `type`, `A`, `BS_WS`, `S`, `AP`, `D` |
| `Datasheets_unit_composition.csv` | `datasheet_unit_composition` | `datasheet_id`, `line`, `description` |
| `Datasheets_models_cost.csv` | `datasheet_model_cost` | `datasheet_id`, `line`, `description`, `cost` |
| `Datasheets_stratagems.csv` | `datasheet_stratagem` | `datasheet_id`, `stratagem_id` |
| `Datasheets_enhancements.csv` | `datasheet_enhancement` | `datasheet_id`, `enhancement_id` |
| `Datasheets_detachment_abilities.csv` | `datasheet_detachment_ability` | `datasheet_id`, `detachment_ability_id` |
| `Datasheets_leader.csv` | `datasheet_leader` | `leader_id`, `attached_id` |
| `Stratagems.csv` | `stratagem` | `faction_id`, `name`, `id`, `type`, `cp_cost`, `legend`, `turn`, `phase`, `detachment`, `detachment_id`, `description` |
| `Abilities.csv` | `ability` | `id`, `name`, `legend`, `faction_id`, `description` |
| `Enhancements.csv` | `enhancement` | `faction_id`, `id`, `name`, `cost`, `detachment`, `detachment_id`, `legend`, `description` |
| `Detachment_abilities.csv` | `detachment_ability` | `id`, `faction_id`, `name`, `legend`, `description`, `detachment`, `detachment_id` |
| `Detachments.csv` | `detachment` | `id`, `faction_id`, `name`, `legend`, `type` |

`Last_update.csv` is one logical row. Its `last_update` value is copied into the manifest. It does not become an entity. The publisher documents the timestamp as `yyyy-MM-dd HH:mm:ss` in GMT+3. Store that string unchanged.

Boolean columns are the strings `true` and `false`: `virtual` and `is_faction_keyword`. Description fields may contain HTML and newlines. Keep both. Wargear `type` is free text. In the current export the filled values are `Ranged` and `Melee`; empty is allowed. Do not coerce numbers.

## Parsing

1. Decode bytes as UTF-8. Reject the file if decoding fails. Strip one leading U+FEFF if present.
2. Normalize physical line breaks: `\r\n` and lone `\r` become `\n`. Newlines inside a field remain part of the field.
3. The first physical line is the header. It must not contain an embedded newline.
4. Fields are not quoted. A logical record may span physical lines. Append physical lines, joined by `\n`, until the buffer contains as many `|`-separated fields as the raw header (including the trailing empty field). Then drop that one trailing empty field.
5. Skip a physical line that is empty or only pipes when it is not continuing a buffer.
6. If the file ends with an unfinished buffer, the file is a parse error.
7. If a buffer would contain more fields than the header, the file is a parse error. Embedded `|` inside a field is not supported, because the export does not escape delimiters.
8. Do not drop data rows for any other reason.

Header drift and parse errors exit 4, before any corpus directory is replaced.

## Cache

`fetch` writes only under `--cache-dir`:

```text
{cache-dir}/10ed/cache-meta.json
{cache-dir}/10ed/Last_update.csv
{cache-dir}/10ed/Factions.csv
... remaining catalog files
```

`cache-meta.json` contains `edition`, `last_update`, `fetched_at` (UTC, RFC 3339), and a map of file name to lowercase hex SHA-256 of the raw response bytes.

Fetch procedure:

1. `GET {base}Last_update.csv` with the User-Agent `wh-corpus/1 (+https://wahapedia.ru; powered by Wahapedia)`.
2. If `cache-meta.json` exists, its `last_update` equals the remote value, and every catalog file exists, stop. Print that the cache was reused. This makes a second fetch a no-op.
3. Otherwise download the remaining catalog files one at a time, same User-Agent, timeout 60 seconds. On HTTP 429 or 5xx, retry that file up to 3 times with backoff of 1s, 2s, then 4s. Do not download files in parallel.
4. Parse each file far enough to check the header. On drift, delete the partial file, leave any previous complete cache in place, and exit 4.
5. Write `cache-meta.json` last, only after every file is stored and hashed.

`--force` ignores a matching `last_update` and downloads again. `--dry-run` prints each URL and whether the cache would be reused, and writes nothing.

## Identity and foreign keys

An entity id is `{edition}:{table}:{key}`. `edition` is `10ed`. Each key segment is UTF-8 percent-encoded with the unreserved set left as-is (`A-Z`, `a-z`, `0-9`, `-`, `.`, `_`, `~`). `:` and `%` inside a segment are encoded, so the first two colons always separate edition and table. Consumers treat the remainder as an opaque key.

Tables that have an `id` column use that value as the single key segment: `10ed:faction:EX1`.

Other tables use these segments, in order:

| Table | Segments |
| --- | --- |
| `datasheet_ability` | `datasheet_id`, `line` |
| `datasheet_keyword` | `datasheet_id`, `keyword`, `model` |
| `datasheet_model` | `datasheet_id`, `line` |
| `datasheet_option` | `datasheet_id`, `line` |
| `datasheet_wargear` | `datasheet_id`, `line`, `line_in_wargear` |
| `datasheet_unit_composition` | `datasheet_id`, `line` |
| `datasheet_model_cost` | `datasheet_id`, `line` |
| `datasheet_stratagem` | `datasheet_id`, `stratagem_id` |
| `datasheet_enhancement` | `datasheet_id`, `enhancement_id` |
| `datasheet_detachment_ability` | `datasheet_id`, `detachment_ability_id` |
| `datasheet_leader` | `leader_id`, `attached_id` |

If that natural key repeats inside one file, the first row keeps the key above and each later row appends a decimal occurrence segment starting at `2`. Occurrence is per exact natural key, in file order.

`refs` stores corpus ids for foreign-key columns. A blank foreign key is JSON `null` and is not a warning. A non-blank foreign key whose target id is absent is JSON `null` and appends a warning. The entity is still written.

Target ids are `{edition}:{target_table}:{percent-encoded raw id}`:

| Column | Target table | Blank allowed without a warning |
| --- | --- | --- |
| `faction_id` | `faction` | Yes, except on `datasheet` and `detachment`, where blank is a warning |
| `source_id` | `source` | Yes. Some datasheets have an empty source. |
| `datasheet_id` | `datasheet` | No |
| `ability_id` | `ability` | Yes. Blank means the ability text is inline on the datasheet row. |
| `stratagem_id` | `stratagem` | No |
| `enhancement_id` | `enhancement` | No |
| `detachment_ability_id` | `detachment_ability` | No |
| `detachment_id` | `detachment` | Yes |
| `leader_id` | `datasheet` | No |
| `attached_id` | `datasheet` | No |

Warning object, in this key order: `code` (`unresolved_foreign_key`), `entity_id`, `column`, `value`. `value` is the raw foreign key. Warnings follow entity file order, and within an entity they follow the `refs` key order required by the schema.

## Corpus directory

`assemble` writes a temporary directory beside `--out` and renames it into place only after the schema checks pass. A failed run leaves an existing `--out` untouched.

```text
{out}/manifest.json
{out}/entities.jsonl
```

`manifest.json` validates against the root of [corpus-v1.schema.json](../schemas/corpus-v1.schema.json). `files` lists all 20 catalog files in the table order above. `header` is the name list after stripping the trailing empty field, in the order the names appear in the catalog (not the order they appeared in the file). `sha256` is the hash of the cached bytes. `logical_rows` counts data records. `fetched_at` is copied from `cache-meta.json`, so assemble does not read the clock. `schema_version` is `1`, `edition` is `10ed`, `game` is `warhammer-40k`.

`entities.jsonl` is UTF-8. One JSON object per line, LF line endings, a trailing LF, no blank lines. Objects use compact separators (`,` and `:` with no extra space) and this key order: `id`, `edition`, `table`, `fields`, `refs`. `fields` uses catalog column order. Entities follow catalog table order, then logical file order. `Last_update.csv` contributes no entity.

A minimal faction line, using a synthetic id:

```json
{"id":"10ed:faction:EX1","edition":"10ed","table":"faction","fields":{"id":"EX1","name":"Example Faction","link":"https://example.invalid/factions/example"},"refs":{}}
```

A datasheet ability that points at a shared ability, and one that is inline (`ability_id` blank, `refs.ability_id` null, no warning):

```json
{"id":"10ed:datasheet_ability:EXDS:1","edition":"10ed","table":"datasheet_ability","fields":{"datasheet_id":"EXDS","line":"1","ability_id":"EXAB","model":"","name":"Example Ability","description":"<p>Example rule.</p>","type":"","parameter":""},"refs":{"datasheet_id":"10ed:datasheet:EXDS","ability_id":"10ed:ability:EXAB"}}
```

Running assemble twice on the same cache produces identical `manifest.json` and `entities.jsonl` bytes.

## Commands

The top-level `--help` lists the subcommands and the line `powered by Wahapedia`. It does not dump this document. Each subcommand has its own `--help` with an Examples section. Missing required flags exit 2 and print one valid invocation. No command asks a question.

Shared flags:

- `--edition 10ed` on `fetch`, `assemble`, and `export`.
- `--cache-dir PATH` on `fetch`, `assemble`, and `export`.
- `--out PATH` on `assemble` and `export`.
- `--output text|json`. Default `text`. `json` prints one JSON object on stdout and leaves errors on stderr.
- `--dry-run` on `fetch` and `assemble`. `export --dry-run` dry-runs fetch and assemble and does not write a corpus.
- `--force` on `fetch` and `export`.

### `wh-corpus fetch`

Download the catalog into the cache, or reuse it when `last_update` matches.

```text
wh-corpus fetch --edition 10ed --cache-dir ./cache
wh-corpus fetch --edition 10ed --cache-dir ./cache --force
wh-corpus fetch --edition 10ed --cache-dir ./cache --dry-run --output json
```

Text success:

```text
edition: 10ed
cache: ./cache/10ed
last_update: 2026-09-21 12:00:00
fetched_at: 2026-09-21T16:52:00Z
reused_cache: false
```

JSON success uses those same fields.

### `wh-corpus assemble`

Read the cache and write corpus v1. Exit 3 if `cache-meta.json` or any catalog file is missing, or if a file's SHA-256 does not match the meta file.

```text
wh-corpus assemble --edition 10ed --cache-dir ./cache --out ./corpus
wh-corpus assemble --edition 10ed --cache-dir ./cache --out ./corpus --dry-run
```

Text success:

```text
corpus: ./corpus
schema_version: 1
edition: 10ed
entities: 12345
warnings: 2
last_update: 2026-09-21 12:00:00
```

### `wh-corpus validate`

Read an existing corpus. No network and no cache.

```text
wh-corpus validate --corpus ./corpus
wh-corpus validate --corpus ./corpus --output json
```

Text success:

```text
corpus: ./corpus
schema_version: 1
valid: true
entities: 12345
warnings: 2
```

### `wh-corpus export`

Run fetch, then assemble, then validate. The corpus directory appears only if validate succeeds.

```text
wh-corpus export --edition 10ed --cache-dir ./cache --out ./corpus
wh-corpus export --edition 10ed --cache-dir ./cache --out ./corpus --force
```

Text success adds `valid: true` to the assemble fields.

## Exit codes

| Code | When | What stderr includes |
| --- | --- | --- |
| 0 | Success. Warnings do not change the code. | Nothing required. |
| 2 | Unknown edition, missing flag, or bad `--output`. | The flag that failed and one working command. |
| 3 | Network failure after retries, or cache incomplete or hashed bytes do not match `cache-meta.json`. | The URL or path, and a fetch command. |
| 4 | Header drift or a CSV parse error. | File name, expected names, actual names or the parse reason. |
| 5 | Schema or extra validation failed. | The first errors, and a validate command. |

## Validation

`validate` checks, in order:

1. `manifest.json` against the schema root.
2. `files` names equal the catalog, in catalog order, with no duplicates. Each `header` equals that file's catalog column list.
3. Each `entities.jsonl` line is one JSON value and matches `#/$defs/entity` resolved against the schema document root. Copying the `entity` object out of `$defs` and validating against that copy drops the document-root `$ref`s and is not a valid check.
4. `id` equals the identity rule applied to `edition`, `table`, and `fields`.
5. Every non-null `refs` value is the id of an entity in this file, and the target table matches the foreign-key table above.
6. Every warning's `entity_id` exists, and the set of warnings equals the set implied by non-blank foreign keys that did not resolve. Order matches the rule in [Identity and foreign keys](#identity-and-foreign-keys).
7. Entity order matches catalog order then file order. A re-assemble of the same cache matches these bytes.

`assemble` runs the same checks before it renames the output into place.

## Tests

Unit tests use synthetic CSV fixtures. They do not open a socket and they do not contain published rules text. Names such as `Example Faction` and `Example Unit` are enough.

Required cases:

- A fixture catalog with one faction, one source, one datasheet, one inline ability, one shared ability, one keyword, one model, one wargear profile, one leader link, one stratagem, and one detachment assembles to schema-valid JSONL. The second assemble is byte-identical.
- A description field that contains a newline and an HTML tag survives as one entity, with the newline and the tag still in `fields.description`.
- Renaming or adding a header column makes assemble exit 4 and leaves an existing `--out` directory unchanged.
- A datasheet whose `faction_id` is non-blank and unknown is kept. `refs.faction_id` is null. The manifest has one `unresolved_foreign_key` warning. Exit code is 0.
- A blank `ability_id` produces `refs.ability_id: null` and no warning.
- Two keyword rows with the same datasheet, keyword, and model get ids that differ by the occurrence segment on the second row.
- `fetch` against a local HTTP fixture reuses the cache when `Last_update.csv` is unchanged, and downloads again when it changes. The test server is in-process. The default test suite does not call `wahapedia.ru`.
- `validate` on the fixture corpus exits 0. A corpus with `schema_version` other than 1 exits 5.

## Later specs

[02-rust-graph.md](02-rust-graph.md) consumes this directory and rejects any `schema_version` other than `1`.
