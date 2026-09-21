"""Edition registry and the 10th-edition file catalog."""

from __future__ import annotations

from dataclasses import dataclass
from urllib.parse import quote

from wh_corpus.errors import HeaderDriftError, UsageError

SOURCE_URL = "https://wahapedia.ru/wh40k10ed/"
SPEC_URL = "https://wahapedia.ru/wh40k10ed/Export%20Data%20Specs.xlsx"
GAME = "warhammer-40k"
USER_AGENT = "wh-corpus/1 (+https://wahapedia.ru; powered by Wahapedia)"
IMPLEMENTED_EDITION = "10ed"

FETCH_EXAMPLE = "wh-corpus fetch --edition 10ed --cache-dir ./cache"
ASSEMBLE_EXAMPLE = "wh-corpus assemble --edition 10ed --cache-dir ./cache --out ./corpus"
VALIDATE_EXAMPLE = "wh-corpus validate --corpus ./corpus"
EXPORT_EXAMPLE = "wh-corpus export --edition 10ed --cache-dir ./cache --out ./corpus"


@dataclass(frozen=True)
class Edition:
    key: str
    game: str
    base_url: str | None
    implemented: bool


EDITIONS: dict[str, Edition] = {
    "10ed": Edition(
        key="10ed",
        game="Warhammer 40,000",
        base_url=SOURCE_URL,
        implemented=True,
    ),
    "11ed": Edition(
        key="11ed",
        game="Warhammer 40,000",
        base_url="https://wahapedia.ru/wh40k11ed/",
        implemented=False,
    ),
    "aos": Edition(
        key="aos",
        game="Age of Sigmar",
        base_url=None,
        implemented=False,
    ),
}


@dataclass(frozen=True)
class ForeignKey:
    column: str
    target_table: str
    blank_warns: bool


@dataclass(frozen=True)
class CatalogFile:
    file_name: str
    table: str | None
    columns: tuple[str, ...]
    key_columns: tuple[str, ...]
    foreign_keys: tuple[ForeignKey, ...]


def _fk(column: str, target: str, *, blank_warns: bool) -> ForeignKey:
    return ForeignKey(column=column, target_table=target, blank_warns=blank_warns)


def _file(
    file_name: str,
    table: str | None,
    columns: tuple[str, ...],
    key_columns: tuple[str, ...] = (),
    foreign_keys: tuple[ForeignKey, ...] = (),
) -> CatalogFile:
    if table is not None and not key_columns:
        key_columns = ("id",)
    return CatalogFile(
        file_name=file_name,
        table=table,
        columns=columns,
        key_columns=key_columns,
        foreign_keys=foreign_keys,
    )


_DATASHEET = _fk("datasheet_id", "datasheet", blank_warns=True)
_FACTION_OPTIONAL = _fk("faction_id", "faction", blank_warns=False)
_FACTION_REQUIRED = _fk("faction_id", "faction", blank_warns=True)
_DETACHMENT = _fk("detachment_id", "detachment", blank_warns=False)

# Live 10th-edition headers. Datasheets_leader.csv uses leader_id and attached_id.
CATALOG: tuple[CatalogFile, ...] = (
    _file("Last_update.csv", None, ("last_update",), key_columns=()),
    _file("Factions.csv", "faction", ("id", "name", "link")),
    _file(
        "Source.csv",
        "source",
        ("id", "name", "type", "edition", "version", "errata_date", "errata_link"),
    ),
    _file(
        "Datasheets.csv",
        "datasheet",
        (
            "id",
            "name",
            "faction_id",
            "source_id",
            "legend",
            "role",
            "loadout",
            "transport",
            "virtual",
            "leader_head",
            "leader_footer",
            "damaged_w",
            "damaged_description",
            "link",
        ),
        foreign_keys=(
            _FACTION_REQUIRED,
            _fk("source_id", "source", blank_warns=False),
        ),
    ),
    _file(
        "Datasheets_abilities.csv",
        "datasheet_ability",
        (
            "datasheet_id",
            "line",
            "ability_id",
            "model",
            "name",
            "description",
            "type",
            "parameter",
        ),
        key_columns=("datasheet_id", "line"),
        foreign_keys=(
            _DATASHEET,
            _fk("ability_id", "ability", blank_warns=False),
        ),
    ),
    _file(
        "Datasheets_keywords.csv",
        "datasheet_keyword",
        ("datasheet_id", "keyword", "model", "is_faction_keyword"),
        key_columns=("datasheet_id", "keyword", "model"),
        foreign_keys=(_DATASHEET,),
    ),
    _file(
        "Datasheets_models.csv",
        "datasheet_model",
        (
            "datasheet_id",
            "line",
            "name",
            "M",
            "T",
            "Sv",
            "inv_sv",
            "inv_sv_descr",
            "W",
            "Ld",
            "OC",
            "base_size",
            "base_size_descr",
        ),
        key_columns=("datasheet_id", "line"),
        foreign_keys=(_DATASHEET,),
    ),
    _file(
        "Datasheets_options.csv",
        "datasheet_option",
        ("datasheet_id", "line", "button", "description"),
        key_columns=("datasheet_id", "line"),
        foreign_keys=(_DATASHEET,),
    ),
    _file(
        "Datasheets_wargear.csv",
        "datasheet_wargear",
        (
            "datasheet_id",
            "line",
            "line_in_wargear",
            "dice",
            "name",
            "description",
            "range",
            "type",
            "A",
            "BS_WS",
            "S",
            "AP",
            "D",
        ),
        key_columns=("datasheet_id", "line", "line_in_wargear"),
        foreign_keys=(_DATASHEET,),
    ),
    _file(
        "Datasheets_unit_composition.csv",
        "datasheet_unit_composition",
        ("datasheet_id", "line", "description"),
        key_columns=("datasheet_id", "line"),
        foreign_keys=(_DATASHEET,),
    ),
    _file(
        "Datasheets_models_cost.csv",
        "datasheet_model_cost",
        ("datasheet_id", "line", "description", "cost"),
        key_columns=("datasheet_id", "line"),
        foreign_keys=(_DATASHEET,),
    ),
    _file(
        "Datasheets_stratagems.csv",
        "datasheet_stratagem",
        ("datasheet_id", "stratagem_id"),
        key_columns=("datasheet_id", "stratagem_id"),
        foreign_keys=(
            _DATASHEET,
            _fk("stratagem_id", "stratagem", blank_warns=True),
        ),
    ),
    _file(
        "Datasheets_enhancements.csv",
        "datasheet_enhancement",
        ("datasheet_id", "enhancement_id"),
        key_columns=("datasheet_id", "enhancement_id"),
        foreign_keys=(
            _DATASHEET,
            _fk("enhancement_id", "enhancement", blank_warns=True),
        ),
    ),
    _file(
        "Datasheets_detachment_abilities.csv",
        "datasheet_detachment_ability",
        ("datasheet_id", "detachment_ability_id"),
        key_columns=("datasheet_id", "detachment_ability_id"),
        foreign_keys=(
            _DATASHEET,
            _fk("detachment_ability_id", "detachment_ability", blank_warns=True),
        ),
    ),
    _file(
        "Datasheets_leader.csv",
        "datasheet_leader",
        ("leader_id", "attached_id"),
        key_columns=("leader_id", "attached_id"),
        foreign_keys=(
            _fk("leader_id", "datasheet", blank_warns=True),
            _fk("attached_id", "datasheet", blank_warns=True),
        ),
    ),
    _file(
        "Stratagems.csv",
        "stratagem",
        (
            "faction_id",
            "name",
            "id",
            "type",
            "cp_cost",
            "legend",
            "turn",
            "phase",
            "detachment",
            "detachment_id",
            "description",
        ),
        foreign_keys=(_FACTION_OPTIONAL, _DETACHMENT),
    ),
    _file(
        "Abilities.csv",
        "ability",
        ("id", "name", "legend", "faction_id", "description"),
        foreign_keys=(_FACTION_OPTIONAL,),
    ),
    _file(
        "Enhancements.csv",
        "enhancement",
        (
            "faction_id",
            "id",
            "name",
            "cost",
            "detachment",
            "detachment_id",
            "legend",
            "description",
        ),
        foreign_keys=(_FACTION_OPTIONAL, _DETACHMENT),
    ),
    _file(
        "Detachment_abilities.csv",
        "detachment_ability",
        (
            "id",
            "faction_id",
            "name",
            "legend",
            "description",
            "detachment",
            "detachment_id",
        ),
        foreign_keys=(_FACTION_OPTIONAL, _DETACHMENT),
    ),
    _file(
        "Detachments.csv",
        "detachment",
        ("id", "faction_id", "name", "legend", "type"),
        foreign_keys=(_FACTION_REQUIRED,),
    ),
)

FILES_BY_NAME: dict[str, CatalogFile] = {item.file_name: item for item in CATALOG}
TABLES: dict[str, CatalogFile] = {
    item.table: item for item in CATALOG if item.table is not None
}
TABLE_ORDER: tuple[str, ...] = tuple(item.table for item in CATALOG if item.table is not None)


def require_edition(value: str | None, example: str) -> str:
    """Accept only the implemented edition and name the flag when it is missing."""
    if value is None or value == "":
        raise UsageError(f"missing flag: --edition\n{example}")
    edition = EDITIONS.get(value)
    if edition is None or not edition.implemented:
        raise UsageError(
            f"unknown edition: {value}\nimplemented edition: {IMPLEMENTED_EDITION}\n{example}"
        )
    return value


def quote_segment(value: str) -> str:
    """Percent-encode one id segment, leaving the unreserved set as-is."""
    return quote(value, safe="-._~")


def format_entity_id(edition: str, table: str, key: tuple[str, ...], occurrence: int) -> str:
    """Build edition:table:key, appending an occurrence segment from the second duplicate."""
    parts = [edition, table, *(quote_segment(segment) for segment in key)]
    if occurrence > 1:
        parts.append(str(occurrence))
    return ":".join(parts)


def natural_key(table: str, fields: dict[str, str]) -> tuple[str, ...]:
    spec = TABLES[table]
    return tuple(fields[column] for column in spec.key_columns)


def assert_header(filename: str, actual: list[str], expected: tuple[str, ...] | list[str]) -> None:
    """Header drift is a name-set mismatch, including empty or duplicate names."""
    expected_list = list(expected)
    if any(name == "" for name in actual) or len(actual) != len(expected_list) or set(actual) != set(expected_list):
        raise HeaderDriftError(filename, expected_list, actual)
