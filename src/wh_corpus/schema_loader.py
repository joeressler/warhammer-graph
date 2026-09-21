"""Load corpus v1 JSON Schema from the repository or the installed package."""

from __future__ import annotations

import json
from functools import lru_cache
from importlib.resources import files
from pathlib import Path

from jsonschema import Draft202012Validator
from referencing import Registry, Resource


def schema_text() -> str:
    """The single schema document, preferring the repo copy during development."""
    repo = Path(__file__).resolve().parents[2] / "docs" / "schemas" / "corpus-v1.schema.json"
    if repo.is_file():
        return repo.read_text(encoding="utf-8")
    return files("wh_corpus").joinpath("corpus-v1.schema.json").read_text(encoding="utf-8")


@lru_cache(maxsize=1)
def schema_document() -> dict[str, object]:
    return json.loads(schema_text())


@lru_cache(maxsize=1)
def schema_registry() -> Registry:
    document = schema_document()
    resource = Resource.from_contents(document)
    return Registry().with_resource(str(document["$id"]), resource)


def manifest_validator() -> Draft202012Validator:
    document = schema_document()
    return Draft202012Validator(document, registry=schema_registry())


def entity_validator() -> Draft202012Validator:
    """Validate an entity via a document-root $ref so nested $refs keep resolving."""
    document = schema_document()
    return Draft202012Validator(
        {"$ref": f"{document['$id']}#/$defs/entity"},
        registry=schema_registry(),
    )
