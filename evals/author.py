"""Author `golden.jsonl` from the live bundle, and check that its answers are reachable.

    python -m evals.author --write     # compute every expected answer from ./bundle and write golden.jsonl
    python -m evals.author --check     # ask the real wh-mcp tools directly (no model) and confirm each
                                       # expected number appears in what the tools return

Expected answers are computed from the bundle's `nodes.jsonl` and `edges.jsonl`
by plain counting, not through `wh-ask`, so the answer key does not depend on the
code being tested. The committed `golden.jsonl` is a static fixture: running this
only matters when the bundle or the question templates change. If the game data
is updated, rebuild the bundle, run `--write`, read the diff, then run `--check`.
"""

from __future__ import annotations

import argparse
import asyncio
import collections
import json
import sys
from pathlib import Path
from typing import Any

from evals.agent import ROOT, host
from evals.grade import GOLDEN, answer_numbers, payload_numbers

STATS = {
    "T": "Toughness",
    "W": "Wounds",
    "OC": "Objective Control",
    "M": "Move",
    "Sv": "Save",
    "Ld": "Leadership",
}


class Graph:
    """The bundle's nodes and edges, indexed for the few lookups the questions need."""

    def __init__(self, bundle: Path) -> None:
        read = lambda name: [json.loads(line) for line in (bundle / name).read_text(encoding="utf-8").splitlines()]
        self.nodes = read("nodes.jsonl")
        self.by_id = {n["id"]: n for n in self.nodes}
        self.out: dict[str, dict[str, list[str]]] = collections.defaultdict(lambda: collections.defaultdict(list))
        self.into: dict[str, dict[str, list[str]]] = collections.defaultdict(lambda: collections.defaultdict(list))
        for edge in read("edges.jsonl"):
            self.out[edge["from"]][edge["kind"]].append(edge["to"])
            self.into[edge["to"]][edge["kind"]].append(edge["from"])

    def one(self, kind: str, label: str) -> dict[str, Any]:
        """The only node of `kind` with this label. A shared name is an error: the question would be ambiguous."""
        found = [n for n in self.nodes if n["kind"] == kind and n["label"].casefold() == label.casefold()]
        if len(found) != 1:
            raise SystemExit(f"{kind} {label!r} matches {len(found)} nodes; pick an unambiguous one")
        return found[0]

    def children(self, node: dict[str, Any], edge: str) -> list[dict[str, Any]]:
        return [self.by_id[i] for i in self.out[node["id"]][edge]]

    # -- facts
    def stat(self, unit: str, key: str) -> str:
        values = {m["attrs"][key] for m in self.children(self.one("Datasheet", unit), "DATASHEET_HAS_MODEL")}
        if len(values) != 1:
            raise SystemExit(f"{unit} has {sorted(values)} for {key}; the question would be ambiguous")
        return values.pop()

    def points(self, unit: str) -> int:
        costs = self.children(self.one("Datasheet", unit), "DATASHEET_HAS_COST")
        if len(costs) != 1:
            raise SystemExit(f"{unit} has {len(costs)} cost lines; the question would be ambiguous")
        return int(costs[0]["text"].splitlines()[-1])

    def faction_units(self, faction: str) -> int:
        return len(self.out[self.one("Faction", faction)["id"]]["FACTION_HAS_DATASHEET"])

    def keyword_units(self, keyword: str) -> int:
        return len(set(self.into[self.one("Keyword", keyword)["id"]]["DATASHEET_HAS_KEYWORD"]))

    def ability_units(self, ability: str) -> int:
        holders: set[str] = set()
        for node in self.nodes:
            if node["kind"] == "Ability" and node["label"].casefold() == ability.casefold():
                holders.update(self.into[node["id"]]["DATASHEET_HAS_ABILITY"])
        return len(holders)

    def detachment_count(self, detachment: str, edge: str) -> int:
        return len(self.out[self.one("Detachment", detachment)["id"]][edge])

    def rule_attr(self, detachment: str, kind: str, edge: str, name: str, attr: str) -> str:
        found = [n for n in self.children(self.one("Detachment", detachment), edge) if n["label"].casefold() == name.casefold()]
        if len(found) != 1:
            raise SystemExit(f"{detachment}/{name}: {len(found)} matches")
        return found[0]["attrs"][attr]

    def kind_count(self, kind: str) -> int:
        return sum(1 for n in self.nodes if n["kind"] == kind)


def item(id: str, kind: str, question: str, tools: list[str], args: dict[str, Any], answer: Any, notes: str = "") -> dict[str, Any]:
    entry = {"id": id, "question": question, "expected_tools": tools, "expected_args": args, "expected_answer": answer, "type": kind}
    if notes:
        entry["notes"] = notes
    return entry


def num(text: str) -> int:
    return int("".join(ch for ch in text if ch.isdigit()))


def build(g: Graph) -> list[dict[str, Any]]:
    items: list[dict[str, Any]] = []
    unit_args = lambda name: {"get_unit": {"unit": name}}

    # ---- lookups: one typed fact of one unit
    for slug, unit, ask, key, label in [
        ("angron_toughness", "Angron", "Angron", "T", "Toughness"),
        ("angron_wounds", "Angron", "Angron", "W", "Wounds"),
        ("angron_oc", "Angron", "Angron", "OC", "Objective Control"),
        ("warboss_leadership", "Warboss", "a Warboss", "Ld", "Leadership"),
        ("imotekh_save", "Imotekh The Stormlord", "Imotekh the Stormlord", "Sv", "Save"),
        ("hive_tyrant_move", "Hive Tyrant", "a Hive Tyrant", "M", "Move"),
        ("wraithknight_oc", "Wraithknight", "a Wraithknight", "OC", "Objective Control"),
        ("redemptor_wounds", "Redemptor Dreadnought", "a Redemptor Dreadnought", "W", "Wounds"),
        ("guilliman_toughness", "Roboute Guilliman", "Roboute Guilliman", "T", "Toughness"),
        ("warriors_leadership", "Necron Warriors", "Necron Warriors", "Ld", "Leadership"),
    ]:
        value = g.stat(unit, key)
        answer: Any = value if key in ("Sv", "Ld") else num(value)
        items.append(item(f"lookup_{slug}", "lookup", f"What is {ask}'s {label} value?", ["get_unit"], unit_args(unit), answer))
    for slug, unit, ask in [("avatar_invuln", "Avatar of Khaine", "the Avatar of Khaine"), ("angron_invuln", "Angron", "Angron")]:
        items.append(item(f"lookup_{slug}", "lookup", f"What invulnerable save does {ask} have?", ["get_unit"], unit_args(unit), g.stat(unit, "inv_sv") + "+"))
    for slug, unit, ask in [("warboss", "Warboss", "a Warboss"), ("hive_tyrant", "Hive Tyrant", "a Hive Tyrant"), ("wraithknight", "Wraithknight", "a Wraithknight"), ("trazyn", "Trazyn the Infinite", "Trazyn the Infinite")]:
        items.append(item(f"lookup_{slug}_points", "lookup", f"How many points does {ask} cost?", ["get_unit"], unit_args(unit), g.points(unit)))
    for slug, name, answer in [
        ("auric_reserves_cp", "Superhuman Reserves", num(g.rule_attr("Auric Champions", "Stratagem", "DETACHMENT_HAS_STRATAGEM", "SUPERHUMAN RESERVES", "cp_cost"))),
        ("auric_vigil_cp", "Vigil Unending", num(g.rule_attr("Auric Champions", "Stratagem", "DETACHMENT_HAS_STRATAGEM", "VIGIL UNENDING", "cp_cost"))),
    ]:
        items.append(item(f"lookup_{slug}", "lookup", f"In the Auric Champions detachment, how many command points does the {name} stratagem cost?", ["get_detachment"], {"get_detachment": {"detachment": "Auric Champions"}}, answer))
    for slug, detachment, name, ask in [
        ("martial_philosopher_cost", "Auric Champions", "Martial Philosopher", "the Martial Philosopher enhancement"),
        ("panoptispex_cost", "Shield Host", "Panoptispex", "the Panoptispex enhancement"),
    ]:
        cost = num(g.rule_attr(detachment, "Enhancement", "DETACHMENT_HAS_ENHANCEMENT", name, "cost"))
        items.append(item(f"lookup_{slug}", "lookup", f"In the {detachment} detachment, what is the points cost of {ask}?", ["get_detachment"], {"get_detachment": {"detachment": detachment}}, cost))
    items.append(item(
        "lookup_shoulder_phase", "lookup",
        "In the Auric Champions detachment, in which phase is the Shoulder the Mantle stratagem used?",
        ["get_detachment"], {"get_detachment": {"detachment": "Auric Champions"}},
        g.rule_attr("Auric Champions", "Stratagem", "DETACHMENT_HAS_STRATAGEM", "SHOULDER THE MANTLE", "phase").split()[0].lower(),
        "The phase is a typed field on the stratagem.",
    ))
    items.append(item(
        "lookup_dropship_transport", "lookup", "How many models can the Orion Assault Dropship transport?",
        ["get_unit"], unit_args("Orion Assault Dropship"), 12,
        "The capacity is free text in the datasheet, not a typed field. Known hard.",
    ))

    # ---- counts: the tool states the number, so a correct answer is always grounded
    for faction, ask in [("Adeptus Custodes", "Adeptus Custodes"), ("Necrons", "the Necrons"), ("Orks", "the Orks"), ("Tyranids", "the Tyranids"), ("Leagues of Votann", "the Leagues of Votann")]:
        items.append(item(f"count_roster_{faction.lower().replace(' ', '_')}", "count", f"How many units can {ask} field?", ["get_roster"], {"get_roster": {"subject": faction}}, g.faction_units(faction)))
    for keyword in ("Psyker", "Monster", "Aircraft", "Fortification"):
        items.append(item(f"count_keyword_{keyword.lower()}", "count", f"How many units have the {keyword} keyword?", ["units_with_keyword"], {"units_with_keyword": {"keyword": keyword}}, g.keyword_units(keyword)))
    for ability in ("Feel No Pain", "Deep Strike", "Lone Operative", "Scouts"):
        items.append(item(f"count_ability_{ability.lower().replace(' ', '_')}", "count", f"How many units have the {ability} ability?", ["units_with_ability"], {"units_with_ability": {"ability": ability}}, g.ability_units(ability)))
    for detachment, edge, noun in [
        ("Shield Host", "DETACHMENT_HAS_STRATAGEM", "stratagems"),
        ("Voyagers in Darkness", "DETACHMENT_HAS_STRATAGEM", "stratagems"),
        ("Blood Legion", "DETACHMENT_HAS_ABILITY", "rules"),
        ("Eldritch Raiders", "DETACHMENT_HAS_ABILITY", "rules"),
        ("Black Ship Guardians", "DETACHMENT_HAS_ENHANCEMENT", "enhancements"),
    ]:
        items.append(item(f"count_{detachment.lower().replace(' ', '_')}_{noun}", "count", f"How many {noun} does the {detachment} detachment have?", ["get_detachment"], {"get_detachment": {"detachment": detachment}}, g.detachment_count(detachment, edge)))
    items.append(item("count_factions", "count", "How many factions are in the rules data?", ["bundle_info|list_factions"], {}, g.kind_count("Faction"), "bundle_info states the count per kind."))

    # ---- comparisons: answers that stay inside what the tools print
    def pair(*units: str) -> dict[str, Any]:
        return {"get_unit": {"unit": list(units)}}

    def yes_no(slug: str, question: str, units: tuple[str, ...], answer: bool, tools: list[str] | None = None, args: dict[str, Any] | None = None) -> None:
        items.append(item(f"compare_{slug}", "compare", question + " Answer yes or no first.", tools or ["get_unit"], args if args is not None else pair(*units), "yes" if answer else "no"))

    t = lambda unit: num(g.stat(unit, "T"))
    w = lambda unit: num(g.stat(unit, "W"))
    yes_no("angron_warboss_toughness", "Does Angron have a higher Toughness than a Warboss?", ("Angron", "Warboss"), t("Angron") > t("Warboss"))
    yes_no("warboss_tyrant_toughness", "Is a Warboss's Toughness higher than a Hive Tyrant's?", ("Warboss", "Hive Tyrant"), t("Warboss") > t("Hive Tyrant"))
    yes_no("immortals_lychguard_toughness", "Do Immortals and Lychguard have the same Toughness?", ("Immortals", "Lychguard"), t("Immortals") == t("Lychguard"))
    yes_no("warriors_immortals_toughness", "Do Necron Warriors and Immortals have the same Toughness?", ("Necron Warriors", "Immortals"), t("Necron Warriors") == t("Immortals"))
    yes_no("wraithknight_avatar_wounds", "Does the Wraithknight have more Wounds than the Avatar of Khaine?", ("Wraithknight", "Avatar of Khaine"), w("Wraithknight") > w("Avatar of Khaine"))
    yes_no("tyrant_guilliman_points", "Is a Hive Tyrant cheaper in points than Roboute Guilliman?", ("Hive Tyrant", "Roboute Guilliman"), g.points("Hive Tyrant") < g.points("Roboute Guilliman"))
    yes_no("trazyn_warboss_points", "Do Trazyn the Infinite and a Warboss cost the same number of points?", ("Trazyn the Infinite", "Warboss"), g.points("Trazyn the Infinite") == g.points("Warboss"))
    yes_no(
        "psyker_monster_keyword", "Are there more units with the Psyker keyword than with the Monster keyword?", (),
        g.keyword_units("Psyker") > g.keyword_units("Monster"), ["units_with_keyword"], {"units_with_keyword": {"keyword": ["Psyker", "Monster"]}},
    )
    items.append(item("compare_higher_wounds", "compare", "What is the higher Wounds value of Eldrad Ulthran and a Canoness?", ["get_unit"], pair("Eldrad Ulthran", "Canoness"), max(w("Eldrad Ulthran"), w("Canoness"))))
    items.append(item("compare_higher_toughness", "compare", "What is the higher Toughness value of Angron and Roboute Guilliman?", ["get_unit"], pair("Angron", "Roboute Guilliman"), max(t("Angron"), t("Roboute Guilliman"))))

    # ---- refusals: the graph cannot answer these
    for slug, question, tools, args, why in [
        ("invented_unit_toughness", "What is the Toughness of Lord Zarthus the Undying?", ["get_unit|find_units"], {}, "No such unit."),
        ("invented_unit_points", "How many points does a Grimdark Kitten Brigade cost?", ["get_unit|find_units"], {}, "No such unit."),
        ("invented_stratagem", "In the Shield Host detachment, how many command points does the Banana Strike stratagem cost?", ["get_detachment"], {"get_detachment": {"detachment": "Shield Host"}}, "The detachment exists; the stratagem does not."),
        ("invented_faction", "How many units can the Squats faction field?", ["get_roster|list_factions"], {}, "Not a 10th edition faction."),
        ("invented_keyword", "How many units have the Cavalry keyword?", ["units_with_keyword"], {"units_with_keyword": {"keyword": "Cavalry"}}, "The keyword does not exist in this data."),
        ("missing_stat", "What is the Warp Charge value of Eldrad Ulthran?", [], {}, "No such characteristic in 10th edition."),
        ("other_edition", "What was Angron's Toughness in 9th edition?", [], {}, "The data is 10th edition only."),
        ("future_edition", "What is the 11th edition points cost of an Intercessor Squad?", [], {}, "Not in the data."),
        ("out_of_scope_price", "How much does the Angron miniature cost in dollars?", [], {}, "Real-world prices are not in the graph."),
    ]:
        items.append(item(f"refuse_{slug}", "refuse", question, tools, args, None, why))
    return items


# ---------------------------------------------------------------- the reachability check

async def check(items: list[dict[str, Any]], bundle: Path) -> list[str]:
    """Call the expected tools directly and confirm each expected number is in what they return."""
    problems: list[str] = []
    async with host.Client(host.server_params(host.find_executable(), bundle), mode="legacy") as client:
        for entry in items:
            numbers: set[str] = set()
            errored = False
            for tool_entry in entry["expected_tools"]:
                tool = tool_entry.split("|")[0]
                constraints = entry["expected_args"].get(tool, {})
                scalar = {k: v.split("|")[0] for k, v in constraints.items() if not isinstance(v, list)}
                lists = {k: v for k, v in constraints.items() if isinstance(v, list)}
                calls = [dict(scalar, **{k: value}) for k, values in lists.items() for value in values] or [scalar]
                for arguments in calls:
                    reply = await client.call_tool(tool, arguments)
                    errored |= bool(reply.is_error)
                    numbers |= payload_numbers(host.result_text(reply))
            expected = entry["expected_answer"]
            if entry["type"] == "refuse":
                if entry["id"].endswith("keyword") and not errored:
                    problems.append(f"{entry['id']}: the tool did not error, so the question may be answerable")
            elif isinstance(expected, int) and str(expected) not in numbers:
                problems.append(f"{entry['id']}: expected {expected} but the tools return only {sorted(numbers)[:12]}...")
    return problems


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--bundle", type=Path, default=ROOT / "bundle")
    parser.add_argument("--write", action="store_true", help="Write golden.jsonl.")
    parser.add_argument("--check", action="store_true", help="Check the expected answers against the real tools.")
    args = parser.parse_args(argv)
    if not (args.write or args.check):
        parser.error("choose --write, --check, or both")
    items = build(Graph(args.bundle))
    ids = [entry["id"] for entry in items]
    assert len(ids) == len(set(ids)), "duplicate ids"
    if args.write:
        GOLDEN.write_text("".join(json.dumps(entry, ensure_ascii=False) + "\n" for entry in items), encoding="utf-8", newline="\n")
        print(f"wrote {len(items)} questions to {GOLDEN}")
    if args.check:
        problems = asyncio.run(check(items, args.bundle))
        for line in problems:
            print("PROBLEM", line)
        print(f"checked {len(items)} questions: {len(problems)} problem(s)")
        return 1 if problems else 0
    return 0


if __name__ == "__main__":
    sys.stdout.reconfigure(encoding="utf-8", errors="replace")
    raise SystemExit(main())
