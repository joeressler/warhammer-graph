"""The harness itself, offline: the golden file, the grader, and the runner with a scripted model.

No Ollama and no real bundle. The runner tests drive the real `host.Host` and the
real wh-mcp against the synthetic bundle, the way the host's own tests do.
"""

from __future__ import annotations

import asyncio
import json
from pathlib import Path
from types import SimpleNamespace
from typing import Any

import httpx
import pytest

from evals import grade
from evals.agent import host
from evals.run import HardFailure, is_hard, make_run_id, run_golden, select

GOLDEN = grade.GOLDEN
TYPES = {"lookup", "count", "compare", "refuse"}
FIELDS = {"id", "question", "expected_tools", "expected_args", "expected_answer", "type"}
OPTIONAL = {"notes"}


# ---------------------------------------------------------------- the golden file

def test_the_golden_file_is_well_formed() -> None:
    items = grade.load_golden()
    assert 40 <= len(items) <= 60
    ids = [i["id"] for i in items]
    assert len(ids) == len(set(ids)), "ids must be unique and stable"
    for item in items:
        assert FIELDS <= set(item) <= FIELDS | OPTIONAL, item["id"]
        assert item["type"] in TYPES
        assert isinstance(item["expected_tools"], list) and isinstance(item["expected_args"], dict)
        if item["type"] == "refuse":
            assert item["expected_answer"] is None
        else:
            assert item["expected_answer"] is not None
    assert {i["type"] for i in items} == TYPES, "every question type is present"


def test_a_count_answer_is_a_number_the_tools_state() -> None:
    """Counting questions must not need the model to count rows (see the README)."""
    for item in grade.load_golden():
        if item["type"] == "count":
            assert isinstance(item["expected_answer"], int) and item["expected_tools"]


# ---------------------------------------------------------------- numbers

@pytest.mark.parametrize(
    "text, expected",
    [
        ("Angron has 16 wounds.", ["16"]),
        ("There are twenty-one units", ["21"]),
        ("There are twenty one units", ["21"]),
        ("That is 1,200 points", ["1200"]),
        ("It costs 75.0 points", ["75"]),
        ("Roll 2D6 and add D3, in the 10th edition, on S5", []),
        ("1. First thing\n2. Second thing", []),
        ("Save 4+ and move 6\"", ["4", "6"]),
        ("The 10th edition Warhammer 40,000 rules say 116 units", ["116"]),
        ("In Warhammer 40000 and WH40k, 7 units", ["7"]),
    ],
)
def test_numbers_in_an_answer_are_standalone_values(text: str, expected: list[str]) -> None:
    assert sorted(grade.answer_numbers(text)) == sorted(expected)


def test_node_ids_in_a_tool_result_do_not_ground_a_number() -> None:
    assert "882" not in grade.payload_numbers('{"id":"10ed:datasheet:000000882","wounds":16}')
    assert "16" in grade.payload_numbers('{"id":"10ed:datasheet:000000882","wounds":16}')


# ---------------------------------------------------------------- the grader

def trace(final: str, *calls: tuple[str, dict[str, Any], str], error: str = "") -> dict[str, Any]:
    return {
        "final_text": final, "error": error, "stopped": "answered", "seconds": 1.0,
        "tool_calls": [{"tool": t, "arguments": a, "result": r} for t, a, r in calls],
    }


def case(kind: str, answer: Any, tools: list[str] | None = None, args: dict[str, Any] | None = None, question: str = "A question?", **extra: Any) -> dict[str, Any]:
    return {"id": "x", "type": kind, "question": question, "expected_answer": answer, "expected_tools": tools or [], "expected_args": args or {}, **extra}


def test_a_lookup_is_exact_and_hallucination_is_flagged_even_when_the_tool_is_wrong() -> None:
    item = case("lookup", 11, ["get_unit"], {"get_unit": {"unit": "Angron"}})
    good = grade.grade_trace(item, trace("Angron has Toughness 11.", ("get_unit", {"unit": "Angron"}, '{"T":"11"}')))
    assert good["answer_correct"] and good["tool_correct"] and not good["hallucinated"]

    # The right number, from memory: no tool was called, so the number is ungrounded.
    memory = grade.grade_trace(item, trace("Angron has Toughness 11."))
    assert memory["answer_correct"] and not memory["tool_correct"] and memory["hallucinated"]
    assert memory["hallucinated_numbers"] == ["11"]

    # A wrong number the tool never returned.
    wrong = grade.grade_trace(item, trace("Angron has Toughness 12.", ("get_unit", {"unit": "Angron"}, '{"T":"11"}')))
    assert not wrong["answer_correct"] and wrong["tool_correct"] and wrong["hallucinated"]


def test_a_number_from_the_question_is_not_a_hallucination() -> None:
    item = case("lookup", 12, ["get_unit"], {}, question="Among the 30 models, how many fit?")
    row = grade.grade_trace(item, trace("Of the 30 models, 12 fit.", ("get_unit", {"unit": "x"}, '{"capacity":12}')))
    assert not row["hallucinated"] and row["answer_correct"]


def test_a_count_is_the_first_number_stated() -> None:
    item = case("count", 6, ["get_detachment"])
    call = ("get_detachment", {"detachment": "D"}, '{"summary":"6 stratagems and 4 enhancements"}')
    assert grade.grade_trace(item, trace("It has 6 stratagems.", call))["answer_correct"]
    assert grade.grade_trace(item, trace("It has six stratagems.", call))["answer_correct"]
    # Detail after the headline is fine; a wrong headline is not.
    assert grade.grade_trace(item, trace("It has 6 stratagems. 1. A costs 4 CP.", call))["answer_correct"]
    assert not grade.grade_trace(item, trace("It has 4 enhancements and 6 stratagems.", call))["answer_correct"]
    assert not grade.grade_trace(item, trace("There are no numbers here.", call))["answer_correct"]


def test_the_word_one_in_prose_is_not_a_quantity_for_hallucination() -> None:
    item = case("count", 2, ["get_detachment"])
    call = ("get_detachment", {"detachment": "D"}, '{"rules":2}')
    row = grade.grade_trace(item, trace("It has 2 rules; a unit gets up to one enhancement.", call))
    assert row["answer_correct"] and not row["hallucinated"]
    # But "one" still counts as the answer when 1 is expected.
    single = case("count", 1, ["get_detachment"])
    assert grade.grade_trace(single, trace("There is one rule.", ("get_detachment", {"detachment": "D"}, '{"n":1}')))["answer_correct"]


def test_string_answers_ignore_case_and_typographic_punctuation() -> None:
    item = case("lookup", "movement")
    assert grade.grade_trace(item, trace("Use it in the Movement phase."))["answer_correct"]
    assert not grade.grade_trace(item, trace("Use it in the Shooting phase."))["answer_correct"]
    dash = case("lookup", "twenty-one|21")
    assert grade.grade_trace(dash, trace("There are twenty‑one."))["answer_correct"]


def test_yes_no_answers_use_the_first_yes_or_no() -> None:
    item = case("compare", "yes")
    assert grade.grade_trace(item, trace("Yes, it is higher. No other unit is."))["answer_correct"]
    assert not grade.grade_trace(item, trace("No, it is not."))["answer_correct"]
    assert not grade.grade_trace(item, trace("It is higher."))["answer_correct"]


def test_a_refusal_is_correct_only_if_it_declines_and_states_no_ungrounded_number() -> None:
    item = case("refuse", None, ["get_unit|find_units"])
    ok = grade.grade_trace(item, trace("I could not find a unit with that name.", ("find_units", {"name": "Zed"}, "[]")))
    assert ok["abstain_correct"] and ok["answer_correct"] is None
    made_up = grade.grade_trace(item, trace("Its Toughness is 9.", ("find_units", {"name": "Zed"}, "[]")))
    assert not made_up["abstain_correct"] and made_up["hallucinated"]
    hedge_with_number = grade.grade_trace(item, trace("I could not find it, but it is probably 9.", ("find_units", {"name": "Zed"}, "[]")))
    assert not hedge_with_number["abstain_correct"]


def test_tool_correct_checks_names_alternatives_and_arguments() -> None:
    item = case("lookup", 1, ["get_unit"], {"get_unit": {"unit": "Imotekh the Stormlord"}})
    called = lambda name: trace("1", ("get_unit", {"unit": name}, "{}"))
    assert grade.tool_correct(item, called("Imotekh The Stormlord"))
    assert not grade.tool_correct(item, called("Warboss"))
    assert not grade.tool_correct(item, trace("1", ("search", {"query": "Imotekh"}, "{}")))

    either = case("count", 1, ["bundle_info|list_factions"])
    assert grade.tool_correct(either, trace("1", ("list_factions", {}, "[]")))

    both = case("compare", "yes", ["get_unit"], {"get_unit": {"unit": ["Angron", "Warboss"]}})
    assert not grade.tool_correct(both, called("Angron"))
    assert grade.tool_correct(both, trace("yes", ("get_unit", {"unit": "Angron"}, "{}"), ("get_unit", {"unit": "A Warboss"}, "{}")))

    none_expected = case("refuse", None)
    assert grade.tool_correct(none_expected, trace("no")) is None


def test_an_id_from_an_earlier_lookup_satisfies_a_name_argument() -> None:
    item = case("lookup", 11, ["get_unit"], {"get_unit": {"unit": "Angron"}})
    found = ("find_units", {"name": "Angron"}, '[{"id":"10ed:datasheet:000000099","name":"Angron"}]')
    by_id = ("get_unit", {"unit": "10ed:datasheet:000000099"}, '{"T":"11"}')
    assert grade.tool_correct(item, trace("11", found, by_id))
    assert not grade.tool_correct(item, trace("11", by_id))


def test_a_run_that_errored_before_answering_is_a_failure_not_a_crash() -> None:
    item = case("lookup", 11, ["get_unit"])
    row = grade.grade_trace(item, trace("", error="TimeoutError: "))
    assert row["answer_correct"] is False and not row["hallucinated"] and row["error"]


def test_aggregates_and_the_markdown_summary(tmp_path: Path) -> None:
    items = [
        case("lookup", 11, ["get_unit"]) | {"id": "a"},
        case("count", 6, ["get_detachment"]) | {"id": "b"},
        case("refuse", None, []) | {"id": "c"},
    ]
    golden = tmp_path / "golden.jsonl"
    golden.write_text("".join(json.dumps(i) + "\n" for i in items), encoding="utf-8")
    run = tmp_path / "20260101-000000_m"
    run.mkdir()
    traces = {
        "a": trace("Toughness 11.", ("get_unit", {"unit": "A"}, '{"T":11}')),
        "b": trace("It has 7.", ("get_detachment", {"detachment": "D"}, '{"n":6}')),
        "c": trace("There is no such unit."),
    }
    for name, t in traces.items():
        (run / f"{name}.json").write_text(json.dumps({"id": name, **t}), encoding="utf-8")
    summary = grade.grade_run(run, golden)
    agg = summary["aggregate"]
    assert agg["answer_exact_match"] == {"n": 2, "passed": 1, "pct": 50.0}
    assert agg["tool_correct"]["n"] == 2 and agg["tool_correct"]["passed"] == 2
    assert agg["abstain_correct"] == {"n": 1, "passed": 1, "pct": 100.0}
    assert agg["hallucination"] == {"n": 3, "passed": 1, "pct": 33.3}, "only the 7 is ungrounded"
    markdown = (run / "summary.md").read_text(encoding="utf-8")
    assert "Answer exact match" in markdown and "`b`" in markdown and "| 7 |" in markdown
    assert json.loads((run / "summary.json").read_text(encoding="utf-8"))["items"][1]["hallucinated_numbers"] == ["7"]


# ---------------------------------------------------------------- the runner

def reply(content: str = "", *calls: tuple[str, dict[str, Any]]) -> SimpleNamespace:
    return SimpleNamespace(message=SimpleNamespace(
        content=content,
        tool_calls=[SimpleNamespace(function=SimpleNamespace(name=n, arguments=a)) for n, a in calls],
    ))


class Scripted:
    """A fake model that returns one reply per call, from a list per question."""

    def __init__(self, *turns: Any) -> None:
        self.turns = list(turns)

    async def __call__(self, messages: list[dict[str, Any]], tools: list[dict[str, Any]]) -> Any:
        turn = self.turns.pop(0)
        if isinstance(turn, BaseException):
            raise turn
        return turn


ITEMS = [
    {"id": "q_factions", "type": "count", "question": "How many factions are in the rules data?", "expected_tools": ["bundle_info|list_factions"],
     "expected_args": {}, "expected_answer": 1},
    {"id": "q_second", "type": "count", "question": "How many edges are there?", "expected_tools": ["bundle_info"],
     "expected_args": {}, "expected_answer": 15},
]


def run(exe: Path, bundle: Path, tmp_path: Path, chat: Scripted, items: list[dict[str, Any]] = ITEMS, **options: Any) -> Path:
    return asyncio.run(run_golden(
        items, model="fake:1b", run_id="test-run", results_dir=tmp_path, bundle=bundle, exe=exe,
        chat_factory=lambda name: chat, golden_path=GOLDEN, **options,
    ))


def test_the_runner_saves_a_full_trace_per_question_through_the_real_host_and_server(exe, bundle, tmp_path) -> None:
    chat = Scripted(
        reply("", ("bundle_info", {})), reply("There is 1 faction."),
        reply("", ("bundle_info", {})), reply("There are 15 edges."),
    )
    directory = run(exe, bundle, tmp_path, chat)
    assert directory == tmp_path / "test-run"
    first = json.loads((directory / "q_factions.json").read_text(encoding="utf-8"))
    assert first["final_text"] == "There is 1 faction." and first["model"] == "fake:1b" and first["error"] == ""
    call = first["tool_calls"][0]
    assert call["tool"] == "bundle_info" and '"node_counts_by_kind"' in call["result"] and call["is_error"] is False
    assert [m["role"] for m in first["messages"]] == ["system", "user", "assistant", "tool", "assistant"]
    assert json.loads((directory / "run.json").read_text(encoding="utf-8"))["status"] == "complete"

    graded = [grade.grade_trace(item, json.loads((directory / f"{item['id']}.json").read_text(encoding="utf-8"))) for item in ITEMS]
    assert [row["answer_correct"] for row in graded] == [True, True]
    assert not any(row["hallucinated"] for row in graded)


def test_each_question_starts_a_fresh_conversation(exe, bundle, tmp_path) -> None:
    chat = Scripted(reply("one"), reply("two"))
    directory = run(exe, bundle, tmp_path, chat)
    second = json.loads((directory / "q_second.json").read_text(encoding="utf-8"))
    assert [m["role"] for m in second["messages"]] == ["system", "user", "assistant"], "the first question is not in the second's history"


def test_a_timeout_is_a_failed_question_and_the_run_continues(exe, bundle, tmp_path) -> None:
    class Slow(Scripted):
        async def __call__(self, messages, tools):
            if len(self.turns) == 2:
                self.turns.pop(0)
                await asyncio.sleep(5)
            return await super().__call__(messages, tools)

    directory = run(exe, bundle, tmp_path, Slow(reply("never"), reply("There are 15 edges.")), timeout=0.2)
    slow = json.loads((directory / "q_factions.json").read_text(encoding="utf-8"))
    assert slow["error"].startswith("TimeoutError") and slow["final_text"] == ""
    assert json.loads((directory / "q_second.json").read_text(encoding="utf-8"))["final_text"] == "There are 15 edges."
    assert json.loads((directory / "run.json").read_text(encoding="utf-8"))["status"] == "complete"


def test_a_hard_failure_stops_the_run_but_keeps_the_partial_traces(exe, bundle, tmp_path) -> None:
    chat = Scripted(reply("", ("bundle_info", {})), ConnectionError("ollama is gone"))
    with pytest.raises(HardFailure, match="q_factions"):
        run(exe, bundle, tmp_path, chat)
    partial = json.loads((tmp_path / "test-run" / "q_factions.json").read_text(encoding="utf-8"))
    assert partial["tool_calls"][0]["tool"] == "bundle_info", "what happened before the failure is kept"
    assert "ConnectionError" in partial["error"]
    meta = json.loads((tmp_path / "test-run" / "run.json").read_text(encoding="utf-8"))
    assert meta["status"] == "aborted" and not (tmp_path / "test-run" / "q_second.json").exists()


def test_which_errors_end_a_run() -> None:
    assert is_hard(ConnectionError("x")) and is_hard(httpx.ConnectError("x"))
    assert not is_hard(asyncio.TimeoutError()) and not is_hard(ValueError("x"))


def test_run_ids_carry_the_time_and_the_model() -> None:
    from datetime import datetime, timezone

    assert make_run_id("granite4.1:8b", datetime(2026, 10, 7, 9, 5, 3, tzinfo=timezone.utc)) == "20261007-090503_granite4.1-8b"


def test_select_filters_by_id_and_limit_and_rejects_unknown_ids() -> None:
    assert [i["id"] for i in select(ITEMS, "q_second", None)] == ["q_second"]
    assert len(select(ITEMS, None, 1)) == 1
    with pytest.raises(SystemExit):
        select(ITEMS, "nope", None)
