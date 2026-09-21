# Getting started with wh-corpus

`wh-corpus` downloads the public Wahapedia Warhammer 40,000 10th-edition export and writes corpus v1: a directory containing `manifest.json` and `entities.jsonl`. It is the first step in the pipeline. [wh-graph](wh-graph.md) reads that directory and does not know the CSV filenames or the Wahapedia URLs.

The frozen contract is [01-python-cli.md](../specs/01-python-cli.md) and [corpus-v1.schema.json](../schemas/corpus-v1.schema.json). This page is the install and command guide.

## Install

You need Python 3.12 or newer. From the repository root:

```bash
python3 -m venv .venv
source .venv/bin/activate
python -m pip install -e ".[dev]"
```

On Windows, activate the virtual environment with `.venv\Scripts\activate`.

That install exposes the `wh-corpus` console script and the development extra (`pytest`). Confirm it:

```bash
wh-corpus --help
```

The help text lists `fetch`, `assemble`, `validate`, and `export`, and it includes `powered by Wahapedia`. You can also run `python -m wh_corpus --help` from an environment where the package is installed.

The only implemented edition is `10ed`. `11ed` and `aos` are reserved names. Passing any other `--edition` exits 2 and prints a command you can copy.

## Write a corpus

`export` runs fetch, then assemble, then validate. The corpus directory is replaced only after validation succeeds.

```bash
wh-corpus export --edition 10ed --cache-dir ./cache --out ./corpus
```

Fetch talks to `https://wahapedia.ru/wh40k10ed/`. It downloads one CSV at a time with a 60 second timeout. HTTP 429 and 5xx responses are retried up to three times. A second export reuses the cache when the remote `Last_update.csv` timestamp matches the cache and every catalog file is already present.

Text output looks like this:

```text
corpus: ./corpus
schema_version: 1
edition: 10ed
entities: 12345
warnings: 2
last_update: 2026-09-21 12:00:00
valid: true
```

`--output json` prints one JSON object on stdout. Errors stay on stderr.

On disk:

```text
cache/10ed/cache-meta.json
cache/10ed/Last_update.csv
cache/10ed/Factions.csv
... remaining catalog files

corpus/manifest.json
corpus/entities.jsonl
```

`manifest.json` records `schema_version` 1, edition `10ed`, game `warhammer-40k`, the catalog file hashes, and `last_update`. `entities.jsonl` is one JSON object per line. An entity id looks like `10ed:faction:EX1`. Description fields keep HTML and embedded newlines.

Warnings do not fail the run. A non-blank foreign key that does not resolve becomes JSON `null` on that ref, and the manifest records `unresolved_foreign_key`. The entity is still written.

## Commands

| Command | What it does |
| --- | --- |
| `fetch` | Download into `--cache-dir`, or reuse the cache when `last_update` matches. |
| `assemble` | Read the cache and write corpus v1 to `--out`. No network. |
| `validate` | Check an existing corpus. No network and no cache. |
| `export` | `fetch`, then `assemble`, then `validate`. |

```bash
wh-corpus fetch --edition 10ed --cache-dir ./cache
wh-corpus fetch --edition 10ed --cache-dir ./cache --force
wh-corpus fetch --edition 10ed --cache-dir ./cache --dry-run --output json

wh-corpus assemble --edition 10ed --cache-dir ./cache --out ./corpus
wh-corpus assemble --edition 10ed --cache-dir ./cache --out ./corpus --dry-run

wh-corpus validate --corpus ./corpus
wh-corpus validate --corpus ./corpus --output json

wh-corpus export --edition 10ed --cache-dir ./cache --out ./corpus --force
```

Shared flags:

- `--edition 10ed` on `fetch`, `assemble`, and `export`.
- `--cache-dir PATH` on `fetch`, `assemble`, and `export`.
- `--out PATH` on `assemble` and `export`.
- `--output text|json`. Default `text`.
- `--dry-run` on `fetch`, `assemble`, and `export`. Fetch prints each URL and whether the cache would be reused. Nothing is written. `export --dry-run` dry-runs fetch and assemble and does not write a corpus.
- `--force` on `fetch` and `export`. Ignores a matching `last_update` and downloads again.

No command prompts. A missing flag exits 2 and prints one working invocation. Each subcommand's `--help` includes examples.

## Exit codes

| Code | When |
| --- | --- |
| 0 | Success. Warnings do not change the code. |
| 2 | Unknown edition, missing flag, or bad `--output`. |
| 3 | Network failure after retries, or a cache that is incomplete or whose hashes do not match `cache-meta.json`. |
| 4 | Header drift or a CSV parse error. An existing `--out` directory is left in place. |
| 5 | Schema or extra validation failed. Stderr includes a `validate` command. |

## Tests

From the repository root, with the `dev` extra installed:

```bash
pytest
```

Tests assemble synthetic CSV in memory and, for fetch, an in-process HTTP server. The suite does not open a socket to `wahapedia.ru`.

## Next step

Point [wh-graph](wh-graph.md) at the corpus directory:

```bash
cargo run -p wh-graph -- build --corpus ./corpus --out ./bundle
```
