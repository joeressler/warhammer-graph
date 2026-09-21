"""wh-corpus commands."""

from __future__ import annotations

from pathlib import Path

import typer

from wh_corpus.assemble import AssembleResult, assemble
from wh_corpus.catalog import (
    ASSEMBLE_EXAMPLE,
    EDITIONS,
    EXPORT_EXAMPLE,
    FETCH_EXAMPLE,
    IMPLEMENTED_EDITION,
    VALIDATE_EXAMPLE,
    require_edition,
)
from wh_corpus.errors import CorpusError, UsageError
from wh_corpus.fetch import FetchResult, fetch
from wh_corpus.models import dump_json
from wh_corpus.validate import ValidateResult, validate_corpus

app = typer.Typer(
    help=(
        "Download the Wahapedia 10th-edition export and write corpus v1.\n\n"
        "powered by Wahapedia"
    ),
    no_args_is_help=True,
    add_completion=False,
    rich_markup_mode=None,
    pretty_exceptions_enable=False,
)


def _require_output(value: str, example: str) -> str:
    if value not in {"text", "json"}:
        raise UsageError(f"bad flag: --output\n{example}")
    return value


def _require_path(value: Path | None, flag: str, example: str) -> Path:
    if value is None:
        raise UsageError(f"missing flag: {flag}\n{example}")
    return value


def _show(value: object) -> str:
    if isinstance(value, bool):
        return "true" if value else "false"
    return str(value)


def _emit(data: dict[str, object], output: str, urls: tuple[str, ...] = ()) -> None:
    if output == "json":
        payload = dict(data)
        if urls:
            payload["urls"] = list(urls)
        typer.echo(dump_json(payload))
        return
    for key, value in data.items():
        typer.echo(f"{key}: {_show(value)}")
    for url in urls:
        typer.echo(f"url: {url}")


def _guard(action) -> None:
    try:
        action()
    except CorpusError as exc:
        typer.echo(str(exc), err=True)
        raise typer.Exit(code=exc.exit_code) from exc


def _fetch_payload(result: FetchResult) -> dict[str, object]:
    payload: dict[str, object] = {
        "edition": result.edition,
        "cache": result.cache,
        "last_update": result.last_update,
    }
    if result.fetched_at is not None:
        payload["fetched_at"] = result.fetched_at
    payload["reused_cache"] = result.reused_cache
    return payload


def _assemble_payload(result: AssembleResult, *, include_valid: bool) -> dict[str, object]:
    payload: dict[str, object] = {
        "corpus": result.corpus,
        "schema_version": result.schema_version,
        "edition": result.edition,
        "entities": result.entities,
        "warnings": result.warnings,
        "last_update": result.last_update,
    }
    if include_valid:
        payload["valid"] = True
    return payload


def _validate_payload(result: ValidateResult) -> dict[str, object]:
    return {
        "corpus": result.corpus,
        "schema_version": result.schema_version,
        "valid": result.valid,
        "entities": result.entities,
        "warnings": result.warnings,
    }


@app.command("fetch")
def fetch_command(
    edition: str | None = typer.Option(None, "--edition"),
    cache_dir: Path | None = typer.Option(None, "--cache-dir"),
    output: str = typer.Option("text", "--output"),
    dry_run: bool = typer.Option(False, "--dry-run"),
    force: bool = typer.Option(False, "--force"),
) -> None:
    """Download the catalog into the cache, or reuse it when last_update matches.

    Examples:

    wh-corpus fetch --edition 10ed --cache-dir ./cache

    wh-corpus fetch --edition 10ed --cache-dir ./cache --force

    wh-corpus fetch --edition 10ed --cache-dir ./cache --dry-run --output json
    """

    def action() -> None:
        chosen = _require_output(output, FETCH_EXAMPLE)
        edition_key = require_edition(edition, FETCH_EXAMPLE)
        path = _require_path(cache_dir, "--cache-dir", FETCH_EXAMPLE)
        base_url = EDITIONS[IMPLEMENTED_EDITION].base_url
        assert base_url is not None
        result = fetch(
            edition=edition_key,
            cache_dir=path,
            base_url=base_url,
            force=force,
            dry_run=dry_run,
            fetch_command=FETCH_EXAMPLE,
        )
        _emit(_fetch_payload(result), chosen, result.urls if dry_run else ())

    _guard(action)


@app.command("assemble")
def assemble_command(
    edition: str | None = typer.Option(None, "--edition"),
    cache_dir: Path | None = typer.Option(None, "--cache-dir"),
    out: Path | None = typer.Option(None, "--out"),
    output: str = typer.Option("text", "--output"),
    dry_run: bool = typer.Option(False, "--dry-run"),
) -> None:
    """Read the cache and write corpus v1.

    Examples:

    wh-corpus assemble --edition 10ed --cache-dir ./cache --out ./corpus

    wh-corpus assemble --edition 10ed --cache-dir ./cache --out ./corpus --dry-run
    """

    def action() -> None:
        chosen = _require_output(output, ASSEMBLE_EXAMPLE)
        edition_key = require_edition(edition, ASSEMBLE_EXAMPLE)
        cache = _require_path(cache_dir, "--cache-dir", ASSEMBLE_EXAMPLE)
        destination = _require_path(out, "--out", ASSEMBLE_EXAMPLE)
        result = assemble(
            edition=edition_key,
            cache_dir=cache,
            out=destination,
            dry_run=dry_run,
            fetch_command=FETCH_EXAMPLE,
        )
        _emit(_assemble_payload(result, include_valid=False), chosen)

    _guard(action)


@app.command("validate")
def validate_command(
    corpus: Path | None = typer.Option(None, "--corpus"),
    output: str = typer.Option("text", "--output"),
) -> None:
    """Read an existing corpus. No network and no cache.

    Examples:

    wh-corpus validate --corpus ./corpus

    wh-corpus validate --corpus ./corpus --output json
    """

    def action() -> None:
        chosen = _require_output(output, VALIDATE_EXAMPLE)
        path = _require_path(corpus, "--corpus", VALIDATE_EXAMPLE)
        result = validate_corpus(path)
        _emit(_validate_payload(result), chosen)

    _guard(action)


@app.command("export")
def export_command(
    edition: str | None = typer.Option(None, "--edition"),
    cache_dir: Path | None = typer.Option(None, "--cache-dir"),
    out: Path | None = typer.Option(None, "--out"),
    output: str = typer.Option("text", "--output"),
    dry_run: bool = typer.Option(False, "--dry-run"),
    force: bool = typer.Option(False, "--force"),
) -> None:
    """Run fetch, then assemble, then validate.

    Examples:

    wh-corpus export --edition 10ed --cache-dir ./cache --out ./corpus

    wh-corpus export --edition 10ed --cache-dir ./cache --out ./corpus --force
    """

    def action() -> None:
        chosen = _require_output(output, EXPORT_EXAMPLE)
        edition_key = require_edition(edition, EXPORT_EXAMPLE)
        cache = _require_path(cache_dir, "--cache-dir", EXPORT_EXAMPLE)
        destination = _require_path(out, "--out", EXPORT_EXAMPLE)
        base_url = EDITIONS[IMPLEMENTED_EDITION].base_url
        assert base_url is not None
        fetch(
            edition=edition_key,
            cache_dir=cache,
            base_url=base_url,
            force=force,
            dry_run=dry_run,
            fetch_command=FETCH_EXAMPLE,
        )
        result = assemble(
            edition=edition_key,
            cache_dir=cache,
            out=destination,
            dry_run=dry_run,
            fetch_command=FETCH_EXAMPLE,
        )
        if not dry_run:
            validate_corpus(destination)
        _emit(_assemble_payload(result, include_valid=True), chosen)

    _guard(action)

