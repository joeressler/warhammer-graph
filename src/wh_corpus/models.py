"""Pydantic models for the in-memory cache and corpus."""

from __future__ import annotations

import json
from typing import Literal

from pydantic import BaseModel, ConfigDict


def dump_json(value: object) -> str:
    """Compact UTF-8 JSON with stable key order from the input object."""
    return json.dumps(value, ensure_ascii=False, separators=(",", ":"))


class CacheMeta(BaseModel):
    model_config = ConfigDict(extra="forbid")

    edition: str
    last_update: str
    fetched_at: str
    sha256: dict[str, str]


class EntityRecord(BaseModel):
    model_config = ConfigDict(extra="forbid")

    id: str
    edition: Literal["10ed"]
    table: str
    fields: dict[str, str]
    refs: dict[str, str | None]

    def json_obj(self) -> dict[str, object]:
        return {
            "id": self.id,
            "edition": self.edition,
            "table": self.table,
            "fields": dict(self.fields),
            "refs": dict(self.refs),
        }


class WarningRecord(BaseModel):
    model_config = ConfigDict(extra="forbid")

    code: Literal["unresolved_foreign_key"]
    entity_id: str
    column: str
    value: str

    def json_obj(self) -> dict[str, str]:
        return {
            "code": self.code,
            "entity_id": self.entity_id,
            "column": self.column,
            "value": self.value,
        }


class SourceFileRecord(BaseModel):
    model_config = ConfigDict(extra="forbid")

    name: str
    sha256: str
    logical_rows: int
    header: list[str]

    def json_obj(self) -> dict[str, object]:
        return {
            "name": self.name,
            "sha256": self.sha256,
            "logical_rows": self.logical_rows,
            "header": list(self.header),
        }


class Manifest(BaseModel):
    model_config = ConfigDict(extra="forbid")

    schema_version: Literal[1] = 1
    edition: Literal["10ed"] = "10ed"
    game: Literal["warhammer-40k"] = "warhammer-40k"
    source_url: str
    spec_url: str
    last_update: str
    fetched_at: str
    files: list[SourceFileRecord]
    warnings: list[WarningRecord]

    def json_obj(self) -> dict[str, object]:
        return {
            "schema_version": self.schema_version,
            "edition": self.edition,
            "game": self.game,
            "source_url": self.source_url,
            "spec_url": self.spec_url,
            "last_update": self.last_update,
            "fetched_at": self.fetched_at,
            "files": [item.json_obj() for item in self.files],
            "warnings": [item.json_obj() for item in self.warnings],
        }
