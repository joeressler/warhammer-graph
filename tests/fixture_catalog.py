"""Synthetic catalog rows. Names are examples, not published rules text."""

from __future__ import annotations

import hashlib
from pathlib import Path

from wh_corpus.catalog import CATALOG, FILES_BY_NAME
from wh_corpus.models import CacheMeta, dump_json

LAST_UPDATE = "2026-09-21 12:00:00"
FETCHED_AT = "2026-09-21T16:52:00Z"
INLINE_DESCRIPTION = "<p>Example rule.</p>\ncontinued"


def fill(file_name: str, **values: str) -> dict[str, str]:
    columns = FILES_BY_NAME[file_name].columns
    unknown = set(values) - set(columns)
    if unknown:
        raise AssertionError(f"unknown columns for {file_name}: {sorted(unknown)}")
    return {column: values.get(column, "") for column in columns}


def render(rows_by_file: dict[str, list[dict[str, str]]]) -> dict[str, bytes]:
    files: dict[str, bytes] = {}
    for spec in CATALOG:
        lines = ["|".join(spec.columns) + "|"]
        for row in rows_by_file.get(spec.file_name, []):
            lines.append("|".join(row[column] for column in spec.columns) + "|")
        files[spec.file_name] = ("\n".join(lines) + "\n").encode("utf-8")
    return files


def sample_rows(
    *,
    faction_id: str = "EX1",
    keywords: list[dict[str, str]] | None = None,
    last_update: str = LAST_UPDATE,
) -> dict[str, list[dict[str, str]]]:
    if keywords is None:
        keywords = [
            fill(
                "Datasheets_keywords.csv",
                datasheet_id="EXDS",
                keyword="Infantry",
                model="Example",
                is_faction_keyword="false",
            )
        ]
    return {
        "Last_update.csv": [fill("Last_update.csv", last_update=last_update)],
        "Factions.csv": [
            fill(
                "Factions.csv",
                id="EX1",
                name="Example Faction",
                link="https://example.invalid/factions/example",
            )
        ],
        "Source.csv": [
            fill(
                "Source.csv",
                id="EXSRC",
                name="Example Source",
                type="codex",
                edition="10",
                version="1",
            )
        ],
        "Datasheets.csv": [
            fill(
                "Datasheets.csv",
                id="EXDS",
                name="Example Unit",
                faction_id=faction_id,
                source_id="EXSRC",
                role="Battleline",
                virtual="false",
                link="https://example.invalid/datasheets/example",
            )
        ],
        "Datasheets_abilities.csv": [
            fill(
                "Datasheets_abilities.csv",
                datasheet_id="EXDS",
                line="1",
                ability_id="",
                name="Inline Ability",
                description=INLINE_DESCRIPTION,
            ),
            fill(
                "Datasheets_abilities.csv",
                datasheet_id="EXDS",
                line="2",
                ability_id="EXAB",
                name="Example Ability",
                description="<p>Shared.</p>",
            ),
        ],
        "Datasheets_keywords.csv": keywords,
        "Datasheets_models.csv": [
            fill(
                "Datasheets_models.csv",
                datasheet_id="EXDS",
                line="1",
                name="Example Model",
                M="6",
                T="4",
                Sv="3+",
                W="2",
                Ld="6+",
                OC="2",
            )
        ],
        "Datasheets_wargear.csv": [
            fill(
                "Datasheets_wargear.csv",
                datasheet_id="EXDS",
                line="1",
                line_in_wargear="1",
                name="Example Rifle",
                type="Ranged",
                range="24",
                A="1",
                BS_WS="3+",
                S="4",
                AP="0",
                D="1",
            )
        ],
        "Datasheets_leader.csv": [
            fill("Datasheets_leader.csv", leader_id="EXDS", attached_id="EXDS")
        ],
        "Stratagems.csv": [
            fill(
                "Stratagems.csv",
                faction_id="EX1",
                name="Example Stratagem",
                id="EXST",
                type="Battle Tactic",
                cp_cost="1",
                detachment="Example Detachment",
                detachment_id="EXDET",
                description="Example stratagem.",
            )
        ],
        "Abilities.csv": [
            fill(
                "Abilities.csv",
                id="EXAB",
                name="Example Ability",
                faction_id="EX1",
                description="<p>Example rule.</p>",
            )
        ],
        "Detachments.csv": [
            fill(
                "Detachments.csv",
                id="EXDET",
                faction_id="EX1",
                name="Example Detachment",
                type="Example",
            )
        ],
    }


def write_cache(
    cache_dir: Path,
    files: dict[str, bytes],
    *,
    last_update: str = LAST_UPDATE,
    fetched_at: str = FETCHED_AT,
) -> Path:
    directory = cache_dir / "10ed"
    directory.mkdir(parents=True, exist_ok=True)
    hashes: dict[str, str] = {}
    for spec in CATALOG:
        blob = files[spec.file_name]
        (directory / spec.file_name).write_bytes(blob)
        hashes[spec.file_name] = hashlib.sha256(blob).hexdigest()
    meta = CacheMeta(
        edition="10ed",
        last_update=last_update,
        fetched_at=fetched_at,
        sha256=hashes,
    )
    (directory / "cache-meta.json").write_text(dump_json(meta.model_dump()) + "\n", encoding="utf-8")
    return directory
