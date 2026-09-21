"""Assign corpus ids and foreign keys, then write manifest.json and entities.jsonl."""

from __future__ import annotations

import hashlib
import shutil
from dataclasses import dataclass
from pathlib import Path

from wh_corpus.catalog import (
    CATALOG,
    GAME,
    SOURCE_URL,
    SPEC_URL,
    CatalogFile,
    format_entity_id,
    natural_key,
    quote_segment,
)
from wh_corpus.errors import CacheError, ParseError
from wh_corpus.fetch import edition_dir
from wh_corpus.models import (
    CacheMeta,
    EntityRecord,
    Manifest,
    SourceFileRecord,
    WarningRecord,
    dump_json,
)
from wh_corpus.parse import parse_csv
from wh_corpus.validate import check_corpus


@dataclass(frozen=True)
class AssembleResult:
    corpus: str
    schema_version: int
    edition: str
    entities: int
    warnings: int
    last_update: str
    valid: bool


def _load_meta(directory: Path, fetch_command: str) -> CacheMeta:
    path = directory / "cache-meta.json"
    if not path.is_file():
        raise CacheError(f"cache incomplete: {path}\n{fetch_command}")
    try:
        return CacheMeta.model_validate_json(path.read_text(encoding="utf-8"))
    except (OSError, ValueError) as exc:
        raise CacheError(f"cache incomplete: {path}\n{fetch_command}") from exc


def _load_rows(
    directory: Path,
    meta: CacheMeta,
    fetch_command: str,
) -> dict[str, tuple[list[dict[str, str]], str]]:
    loaded: dict[str, tuple[list[dict[str, str]], str]] = {}
    for spec in CATALOG:
        path = directory / spec.file_name
        if not path.is_file():
            raise CacheError(f"cache incomplete: {path}\n{fetch_command}")
        raw = path.read_bytes()
        digest = hashlib.sha256(raw).hexdigest()
        recorded = meta.sha256.get(spec.file_name)
        if recorded != digest:
            raise CacheError(f"hash mismatch: {path}\n{fetch_command}")
        rows = parse_csv(raw, spec.file_name, spec.columns)
        loaded[spec.file_name] = (rows, digest)
    return loaded


def _target_id(edition: str, target_table: str, raw: str) -> str:
    return f"{edition}:{target_table}:{quote_segment(raw)}"


def build_corpus(
    *,
    edition: str,
    meta: CacheMeta,
    loaded: dict[str, tuple[list[dict[str, str]], str]],
) -> tuple[Manifest, list[EntityRecord]]:
    """Map cached rows onto entities. Last_update.csv contributes no entity."""
    counts: dict[tuple[str, tuple[str, ...]], int] = {}
    entities: list[EntityRecord] = []
    ids_by_table: dict[str, set[str]] = {}
    pending: list[tuple[EntityRecord, CatalogFile]] = []
    for spec in CATALOG:
        rows, _digest = loaded[spec.file_name]
        if spec.table is None:
            continue
        for row in rows:
            fields = {column: row[column] for column in spec.columns}
            key = natural_key(spec.table, fields)
            seen = counts.get((spec.table, key), 0) + 1
            counts[(spec.table, key)] = seen
            entity = EntityRecord(
                id=format_entity_id(edition, spec.table, key, seen),
                edition="10ed",
                table=spec.table,
                fields=fields,
                refs={},
            )
            entities.append(entity)
            pending.append((entity, spec))
            ids_by_table.setdefault(spec.table, set()).add(entity.id)

    warnings: list[WarningRecord] = []
    for entity, spec in pending:
        refs: dict[str, str | None] = {}
        for foreign_key in spec.foreign_keys:
            raw = entity.fields[foreign_key.column]
            if raw == "":
                refs[foreign_key.column] = None
                if foreign_key.blank_warns:
                    warnings.append(
                        WarningRecord(
                            code="unresolved_foreign_key",
                            entity_id=entity.id,
                            column=foreign_key.column,
                            value=raw,
                        )
                    )
                continue
            target = _target_id(edition, foreign_key.target_table, raw)
            if target in ids_by_table.get(foreign_key.target_table, set()):
                refs[foreign_key.column] = target
            else:
                refs[foreign_key.column] = None
                warnings.append(
                    WarningRecord(
                        code="unresolved_foreign_key",
                        entity_id=entity.id,
                        column=foreign_key.column,
                        value=raw,
                    )
                )
        entity.refs = refs

    last_rows, _last_digest = loaded["Last_update.csv"]
    if len(last_rows) != 1:
        raise ParseError("Last_update.csv", "expected one logical row")
    files = [
        SourceFileRecord(
            name=spec.file_name,
            sha256=loaded[spec.file_name][1],
            logical_rows=len(loaded[spec.file_name][0]),
            header=list(spec.columns),
        )
        for spec in CATALOG
    ]
    manifest = Manifest(
        source_url=SOURCE_URL,
        spec_url=SPEC_URL,
        last_update=last_rows[0]["last_update"],
        fetched_at=meta.fetched_at,
        files=files,
        warnings=warnings,
        game=GAME,
    )
    return manifest, entities


def _publish(out: Path, manifest: Manifest, entities: list[EntityRecord]) -> None:
    """Write beside --out and rename into place only after the caller has validated."""
    if out.parent != Path(""):
        out.parent.mkdir(parents=True, exist_ok=True)
    temporary = out.with_name(out.name + ".tmp-wh-corpus")
    if temporary.exists():
        shutil.rmtree(temporary)
    temporary.mkdir(parents=True)
    try:
        (temporary / "manifest.json").write_bytes(
            (dump_json(manifest.json_obj()) + "\n").encode("utf-8")
        )
        body = "".join(dump_json(entity.json_obj()) + "\n" for entity in entities)
        (temporary / "entities.jsonl").write_bytes(body.encode("utf-8"))
        if out.exists():
            backup = out.with_name(out.name + ".bak-wh-corpus")
            if backup.exists():
                shutil.rmtree(backup)
            out.rename(backup)
            try:
                temporary.rename(out)
            except Exception:
                backup.rename(out)
                raise
            shutil.rmtree(backup)
        else:
            temporary.rename(out)
    finally:
        if temporary.exists():
            shutil.rmtree(temporary, ignore_errors=True)


def assemble(
    *,
    edition: str,
    cache_dir: Path,
    out: Path,
    dry_run: bool = False,
    fetch_command: str,
) -> AssembleResult:
    """Read the cache and write corpus v1. A failed run leaves --out untouched."""
    directory = edition_dir(cache_dir, edition)
    meta = _load_meta(directory, fetch_command)
    loaded = _load_rows(directory, meta, fetch_command)
    manifest, entities = build_corpus(edition=edition, meta=meta, loaded=loaded)
    check_corpus(manifest.json_obj(), [entity.json_obj() for entity in entities], corpus=str(out))
    if not dry_run:
        _publish(out, manifest, entities)
    return AssembleResult(
        corpus=str(out),
        schema_version=manifest.schema_version,
        edition=edition,
        entities=len(entities),
        warnings=len(manifest.warnings),
        last_update=manifest.last_update,
        valid=True,
    )
