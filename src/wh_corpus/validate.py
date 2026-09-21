"""Schema and extra checks for a corpus v1 directory."""

from __future__ import annotations

import json
from dataclasses import dataclass
from pathlib import Path

from wh_corpus.catalog import (
    CATALOG,
    TABLES,
    TABLE_ORDER,
    format_entity_id,
    natural_key,
    quote_segment,
)
from wh_corpus.errors import ValidationFailed
from wh_corpus.schema_loader import entity_validator, manifest_validator


@dataclass(frozen=True)
class ValidateResult:
    corpus: str
    schema_version: int
    valid: bool
    entities: int
    warnings: int


def _fail(corpus: str, messages: list[str]) -> None:
    preview = "\n".join(messages[:10])
    raise ValidationFailed(preview, corpus)


def _schema_messages(errors: list[object]) -> list[str]:
    messages: list[str] = []
    for error in errors:
        path = ".".join(str(part) for part in getattr(error, "path", []))
        prefix = f"{path}: " if path else ""
        messages.append(f"{prefix}{getattr(error, 'message', error)}")
    return messages


def _load_entities(text: str, corpus: str) -> list[dict[str, object]]:
    if text == "":
        return []
    if not text.endswith("\n"):
        _fail(corpus, ["entities.jsonl must end with a trailing newline"])
    lines = text.split("\n")[:-1]
    entities: list[dict[str, object]] = []
    for index, line in enumerate(lines, start=1):
        if line == "":
            _fail(corpus, [f"entities.jsonl line {index} is blank"])
        try:
            value = json.loads(line)
        except json.JSONDecodeError as exc:
            _fail(corpus, [f"entities.jsonl line {index} is not JSON: {exc.msg}"])
        if not isinstance(value, dict):
            _fail(corpus, [f"entities.jsonl line {index} is not an object"])
        entities.append(value)
    return entities


def check_corpus(manifest: dict[str, object], entities: list[dict[str, object]], *, corpus: str) -> None:
    """Run the spec checks in order. The entity schema is resolved from the document root."""
    manifest_errors = list(manifest_validator().iter_errors(manifest))
    if manifest_errors:
        _fail(corpus, _schema_messages(manifest_errors))

    files = manifest["files"]
    assert isinstance(files, list)
    names = [item["name"] for item in files if isinstance(item, dict)]
    expected_names = [item.file_name for item in CATALOG]
    if names != expected_names or len(names) != len(set(names)):
        _fail(corpus, [f"files must be the catalog in order: {', '.join(expected_names)}"])
    for item, spec in zip(files, CATALOG, strict=True):
        if not isinstance(item, dict) or item.get("header") != list(spec.columns):
            _fail(corpus, [f"{spec.file_name} header does not match the catalog"])

    validator = entity_validator()
    schema_errors: list[str] = []
    for index, entity in enumerate(entities, start=1):
        found = list(validator.iter_errors(entity))
        if found:
            schema_errors.extend(
                f"entity {index}: {message}" for message in _schema_messages(found)
            )
            if len(schema_errors) >= 10:
                break
    if schema_errors:
        _fail(corpus, schema_errors)

    counts: dict[tuple[str, tuple[str, ...]], int] = {}
    id_errors: list[str] = []
    for entity in entities:
        table = str(entity["table"])
        fields = entity["fields"]
        assert isinstance(fields, dict)
        field_map = {str(key): str(value) for key, value in fields.items()}
        key = natural_key(table, field_map)
        seen = counts.get((table, key), 0) + 1
        counts[(table, key)] = seen
        expected = format_entity_id(str(entity["edition"]), table, key, seen)
        if entity["id"] != expected:
            id_errors.append(f"id {entity['id']} does not match {expected}")
    if id_errors:
        _fail(corpus, id_errors)

    ids_by_table: dict[str, set[str]] = {}
    for entity in entities:
        ids_by_table.setdefault(str(entity["table"]), set()).add(str(entity["id"]))

    ref_errors: list[str] = []
    implied: list[dict[str, str]] = []
    for entity in entities:
        table = str(entity["table"])
        spec = TABLES[table]
        fields = entity["fields"]
        refs = entity["refs"]
        assert isinstance(fields, dict)
        assert isinstance(refs, dict)
        expected_refs: dict[str, str | None] = {}
        for foreign_key in spec.foreign_keys:
            raw = str(fields[foreign_key.column])
            if raw == "":
                expected_refs[foreign_key.column] = None
                if foreign_key.blank_warns:
                    implied.append(
                        {
                            "code": "unresolved_foreign_key",
                            "entity_id": str(entity["id"]),
                            "column": foreign_key.column,
                            "value": raw,
                        }
                    )
                continue
            target = f"{entity['edition']}:{foreign_key.target_table}:{quote_segment(raw)}"
            present = target in ids_by_table.get(foreign_key.target_table, set())
            expected_refs[foreign_key.column] = target if present else None
            if not present:
                implied.append(
                    {
                        "code": "unresolved_foreign_key",
                        "entity_id": str(entity["id"]),
                        "column": foreign_key.column,
                        "value": raw,
                    }
                )
            elif refs.get(foreign_key.column) is not None:
                ref_value = refs[foreign_key.column]
                if not isinstance(ref_value, str) or ref_value not in ids_by_table.get(
                    foreign_key.target_table, set()
                ):
                    ref_errors.append(f"{entity['id']} refs.{foreign_key.column} is not an entity id")
                else:
                    _edition, target_table, _rest = ref_value.split(":", 2)
                    if target_table != foreign_key.target_table:
                        ref_errors.append(
                            f"{entity['id']} refs.{foreign_key.column} targets {target_table}"
                        )
        if refs != expected_refs:
            ref_errors.append(f"{entity['id']} refs do not match resolved foreign keys")
    if ref_errors:
        _fail(corpus, ref_errors)

    warnings = manifest["warnings"]
    assert isinstance(warnings, list)
    known_ids = {str(entity["id"]) for entity in entities}
    for warning in warnings:
        if isinstance(warning, dict) and warning.get("entity_id") not in known_ids:
            _fail(corpus, [f"warning entity_id {warning.get('entity_id')} does not exist"])
    if warnings != implied:
        _fail(corpus, ["warnings do not match unresolved foreign keys"])

    tables = [str(entity["table"]) for entity in entities]
    order_index = {name: index for index, name in enumerate(TABLE_ORDER)}
    last = -1
    for table in tables:
        current = order_index[table]
        if current < last:
            _fail(corpus, ["entities are not in catalog table order"])
        last = current


def validate_corpus(corpus: Path) -> ValidateResult:
    """Read an existing corpus. This does not use the network or the cache."""
    display = str(corpus)
    manifest_path = corpus / "manifest.json"
    entities_path = corpus / "entities.jsonl"
    if not manifest_path.is_file() or not entities_path.is_file():
        _fail(display, ["corpus is missing manifest.json or entities.jsonl"])
    try:
        manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        _fail(display, [f"manifest.json is not JSON: {exc}"])
    if not isinstance(manifest, dict):
        _fail(display, ["manifest.json is not an object"])
    entities = _load_entities(entities_path.read_text(encoding="utf-8"), display)
    check_corpus(manifest, entities, corpus=display)
    warnings = manifest["warnings"]
    schema_version = manifest["schema_version"]
    assert isinstance(warnings, list)
    assert isinstance(schema_version, int)
    return ValidateResult(
        corpus=display,
        schema_version=schema_version,
        valid=True,
        entities=len(entities),
        warnings=len(warnings),
    )
