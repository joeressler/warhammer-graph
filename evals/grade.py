"""Deterministic grader for eval runs. No model is involved.

    python -m evals.grade evals/results/<run_id>

Reads the traces in a results directory and the golden set, and writes
`summary.json` and `summary.md` next to them. The scoring contract is in
`evals/README.md`.
"""

from __future__ import annotations

import argparse
import json
import re
import statistics
import sys
from pathlib import Path
from typing import Any

from evals.agent import plain

HERE = Path(__file__).resolve().parent
GOLDEN = HERE / "golden.jsonl"
ANSWER_TYPES = ("lookup", "count", "compare")

# ---------------------------------------------------------------- numbers

_ONES = {
    "zero": 0, "one": 1, "two": 2, "three": 3, "four": 4, "five": 5, "six": 6, "seven": 7, "eight": 8,
    "nine": 9, "ten": 10, "eleven": 11, "twelve": 12, "thirteen": 13, "fourteen": 14, "fifteen": 15,
    "sixteen": 16, "seventeen": 17, "eighteen": 18, "nineteen": 19,
}
_TENS = {"twenty": 20, "thirty": 30, "forty": 40, "fifty": 50, "sixty": 60, "seventy": 70, "eighty": 80, "ninety": 90}
_WORD_NUMBER = re.compile(
    r"\b(?:(" + "|".join(_TENS) + r")(?:[- ](" + "|".join(k for k in _ONES if 0 < _ONES[k] < 10) + r"))?|(" + "|".join(_ONES) + r"))\b"
)
# A standalone number: not part of a word (D6, 2D6, 10th, S5) and not a list marker.
_ANSWER_NUMBER = re.compile(r"(?<![\w.])(\d{1,3}(?:,\d{3})+|\d+)(\.\d+)?(?![\w])")
_PAYLOAD_NUMBER = re.compile(r"\d+(?:,\d{3})*(?:\.\d+)?")
_LIST_MARKER = re.compile(r"(?m)^\s*\d+[.)]\s+")
_NODE_ID = re.compile(r"\b\d+ed:[\w:.\-]+")
# The game's own name contains a number that is not a claim about the data.
_GAME_NAME = re.compile(r"\b(?:warhammer|wh)\s?40[, ]?(?:000|k)\b")


def canon(number: str) -> str:
    """`1,200` and `1200.0` and `0012` are all `1200`."""
    number = number.replace(",", "")
    if "." in number:
        number = number.rstrip("0").rstrip(".")
    return str(int(number)) if number.isdigit() else number


def answer_numbers(text: str, *, skip_one: bool = False) -> list[str]:
    """The standalone numbers in an answer, as digits, in the order written.

    Number words count (`twenty-one` is 21). With `skip_one`, the word "one" does
    not, because in prose it is usually not a quantity ("up to one enhancement").
    """
    text = _GAME_NAME.sub(" ", _LIST_MARKER.sub("", plain(text)))
    found = [(m.start(), canon(m.group(1) + (m.group(2) or ""))) for m in _ANSWER_NUMBER.finditer(text)]
    for m in _WORD_NUMBER.finditer(text):
        tens, ones, single = m.groups()
        value = _TENS[tens] + (_ONES[ones] if ones else 0) if tens else _ONES[single]
        if not (skip_one and value == 1 and single == "one"):
            found.append((m.start(), str(value)))
    return [value for _, value in sorted(found)]


def payload_numbers(text: str) -> set[str]:
    """Every number in a tool result, with node ids removed so id fragments do not count."""
    return {canon(m.group(0)) for m in _PAYLOAD_NUMBER.finditer(_NODE_ID.sub(" ", text))}


# ---------------------------------------------------------------- tools

def _key(text: Any) -> str:
    text = plain(str(text))
    text = re.sub(r"\b(a|an|the)\b", " ", text)
    return re.sub(r"[^a-z0-9]+", " ", text).strip()


def _arg_matches(value: Any, spec: str, calls: list[dict[str, Any]]) -> bool:
    """Whether a call's argument meets one expected value (`a|b` accepts either).

    A name matches when either normalized form contains the other. An id the
    agent got from an earlier lookup of that name also counts.
    """
    have = _key(value)
    for option in spec.split("|"):
        want = _key(option)
        if want and have and (want in have or have in want):
            return True
        if ":" in str(value) and any(
            str(value) in call.get("result", "") and any(_key(v) and want in _key(v) for v in call["arguments"].values())
            for call in calls
        ):
            return True
    return False


def _tool_met(tool: str, constraints: dict[str, Any], calls: list[dict[str, Any]]) -> bool:
    mine = [call for call in calls if call["tool"] == tool]
    scalar = {arg: spec for arg, spec in constraints.items() if not isinstance(spec, list)}
    required = {arg: spec for arg, spec in constraints.items() if isinstance(spec, list)}
    if not any(all(arg in call["arguments"] and _arg_matches(call["arguments"][arg], spec, calls) for arg, spec in scalar.items()) for call in mine):
        return False
    return all(
        any(arg in call["arguments"] and _arg_matches(call["arguments"][arg], item, calls) for call in mine)
        for arg, items in required.items()
        for item in items
    )


def tool_correct(item: dict[str, Any], trace: dict[str, Any]) -> bool | None:
    """Every expected tool was called with the expected arguments. None when none are expected."""
    expected = item.get("expected_tools") or []
    if not expected:
        return None
    constraints = item.get("expected_args") or {}
    return all(
        any(_tool_met(tool, constraints.get(tool, {}), trace["tool_calls"]) for tool in entry.split("|"))
        for entry in expected
    )


# ---------------------------------------------------------------- answers

REFUSAL = re.compile(
    "|".join(
        re.escape(phrase)
        for phrase in (
            "not in the", "not found", "could not find", "couldn't find", "can't find", "cannot find", "unable to find",
            "no unit", "no such", "no matching", "no record", "no data", "no information", "does not exist", "doesn't exist",
            "do not exist", "don't have", "do not have", "not available", "not part of", "not covered", "no results",
            "isn't in", "is not in", "not present", "does not contain", "doesn't contain", "no datasheet", "no faction",
            "no keyword", "no stratagem", "no enhancement", "does not appear", "not listed", "only covers", "only contains",
            "only has", "unable to", "cannot", "can't answer", "not able", "no entry", "no detachment", "not recognized",
            "not recognised", "don't know", "do not know", "isn't a", "is not a",
        )
    )
)
_YES_NO = re.compile(r"\b(yes|no)\b")


def check_answer(item: dict[str, Any], trace: dict[str, Any], grounded: set[str], question_numbers: set[str]) -> bool:
    """Whether the final answer states the expected fact."""
    text = trace.get("final_text") or ""
    if not text.strip():
        return False
    kind, expected = item["type"], item.get("expected_answer")
    if kind == "refuse":
        extra = set(answer_numbers(text, skip_one=True)) - question_numbers - grounded
        return bool(REFUSAL.search(plain(text))) and not extra
    if any(plain(str(word)) in plain(text) for word in item.get("forbidden") or []):
        return False
    if isinstance(expected, bool):
        expected = "yes" if expected else "no"
    if isinstance(expected, (int, float)):
        numbers = [n for n in answer_numbers(text) if n not in question_numbers]
        want = canon(str(expected))
        # A count is the first number stated: the headline, before any detail that follows it.
        return bool(numbers) and numbers[0] == want if kind == "count" else want in numbers
    expected = str(expected).strip().lower()
    if expected in ("yes", "no"):
        first = _YES_NO.search(plain(text))
        return bool(first) and first.group(1) == expected
    return any(plain(option) in plain(text) for option in expected.split("|"))


def hallucinated_numbers(trace: dict[str, Any], question: str) -> list[str]:
    """Numbers in the final answer that appear in no tool result of the trace.

    A number the user wrote in the question is not counted, since repeating it
    is not a claim about the data.
    """
    grounded = set().union(*(payload_numbers(call.get("result", "")) for call in trace["tool_calls"])) if trace["tool_calls"] else set()
    exempt = set(answer_numbers(question))
    return sorted({n for n in answer_numbers(trace.get("final_text") or "", skip_one=True) if n not in grounded and n not in exempt})


def grade_trace(item: dict[str, Any], trace: dict[str, Any]) -> dict[str, Any]:
    """One question's verdicts."""
    question_numbers = set(answer_numbers(item["question"]))
    grounded = set().union(*(payload_numbers(c.get("result", "")) for c in trace["tool_calls"])) if trace["tool_calls"] else set()
    bad = hallucinated_numbers(trace, item["question"])
    correct = check_answer(item, trace, grounded, question_numbers)
    refuse = item["type"] == "refuse"
    return {
        "id": item["id"],
        "type": item["type"],
        "tool_correct": tool_correct(item, trace),
        "answer_correct": None if refuse else correct,
        "abstain_correct": correct if refuse else None,
        "hallucinated": bool(bad),
        "hallucinated_numbers": bad,
        "expected": item.get("expected_answer"),
        "answer": trace.get("final_text") or "",
        "calls": [call["tool"] for call in trace["tool_calls"]],
        "stopped": trace.get("stopped", ""),
        "error": trace.get("error", ""),
        "seconds": trace.get("seconds", 0.0),
    }


# ---------------------------------------------------------------- aggregates

def _rate(flags: list[bool | None]) -> dict[str, Any]:
    """`{n, passed, pct}` over the entries that are not None."""
    scored = [f for f in flags if f is not None]
    return {"n": len(scored), "passed": sum(scored), "pct": round(100 * sum(scored) / len(scored), 1) if scored else None}


def aggregate(rows: list[dict[str, Any]]) -> dict[str, Any]:
    """The headline numbers for a set of graded rows."""
    return {
        "questions": len(rows),
        "answer_exact_match": _rate([r["answer_correct"] for r in rows]),
        "tool_correct": _rate([r["tool_correct"] for r in rows]),
        "abstain_correct": _rate([r["abstain_correct"] for r in rows]),
        "hallucination": _rate([r["hallucinated"] for r in rows]),
        "errors": sum(bool(r["error"]) for r in rows),
        "step_limit": sum(r["stopped"] == "step_limit" for r in rows),
        "median_seconds": round(statistics.median(r["seconds"] for r in rows), 1) if rows else None,
    }


def load_golden(path: Path = GOLDEN) -> list[dict[str, Any]]:
    return [json.loads(line) for line in path.read_text(encoding="utf-8").splitlines() if line.strip()]


def load_traces(directory: Path) -> dict[str, dict[str, Any]]:
    traces = {}
    for file in sorted(directory.glob("*.json")):
        if file.name in ("run.json", "summary.json"):
            continue
        trace = json.loads(file.read_text(encoding="utf-8"))
        traces[trace["id"]] = trace
    return traces


def grade_run(directory: Path, golden_path: Path = GOLDEN) -> dict[str, Any]:
    """Grade every trace in `directory` against the golden set and write the summaries."""
    golden = load_golden(golden_path)
    traces = load_traces(directory)
    meta_file = directory / "run.json"
    meta = json.loads(meta_file.read_text(encoding="utf-8")) if meta_file.exists() else {}
    rows = [grade_trace(item, traces[item["id"]]) for item in golden if item["id"] in traces]
    not_run = [item["id"] for item in golden if item["id"] not in traces]
    summary = {
        "run_id": directory.name,
        "meta": meta,
        "golden_questions": len(golden),
        "not_run": not_run,
        "aggregate": aggregate(rows),
        "by_type": {kind: aggregate([r for r in rows if r["type"] == kind]) for kind in (*ANSWER_TYPES, "refuse") if any(r["type"] == kind for r in rows)},
        "items": rows,
    }
    (directory / "summary.json").write_text(json.dumps(summary, indent=2, ensure_ascii=False), encoding="utf-8")
    (directory / "summary.md").write_text(render_markdown(summary), encoding="utf-8")
    return summary


# ---------------------------------------------------------------- output

def _pct(rate: dict[str, Any]) -> str:
    return "-" if rate["pct"] is None else f"{rate['pct']}% ({rate['passed']}/{rate['n']})"


def _mark(flag: bool | None) -> str:
    return "-" if flag is None else ("yes" if flag else "NO")


def _cell(text: Any, limit: int = 60) -> str:
    text = " ".join(str(text).split()).replace("|", "/")
    return text if len(text) <= limit else text[: limit - 3] + "..."


def render_markdown(summary: dict[str, Any]) -> str:
    meta, agg = summary["meta"], summary["aggregate"]
    lines = [f"# Eval run `{summary['run_id']}`", ""]
    if meta:
        lines += [
            f"- Model: `{meta.get('model')}`, temperature {meta.get('temperature')}, context {meta.get('num_ctx')}, "
            f"{meta.get('max_steps')} steps, tool results cut at {meta.get('max_tool_chars')} characters",
            f"- Bundle fingerprint: `{str(meta.get('corpus_fingerprint'))[:16]}`, commit `{str(meta.get('git_commit'))[:10]}`",
            f"- Questions graded: {agg['questions']} of {summary['golden_questions']}",
            "",
        ]
    lines += [
        "## Aggregate",
        "",
        "| Metric | Result |",
        "|---|---|",
        f"| Answer exact match (lookup, count, compare) | **{_pct(agg['answer_exact_match'])}** |",
        f"| Tool correct | {_pct(agg['tool_correct'])} |",
        f"| Abstain correct (refuse) | {_pct(agg['abstain_correct'])} |",
        f"| Hallucination rate | {_pct(agg['hallucination'])} |",
        f"| Runs with an error / hit the step limit | {agg['errors']} / {agg['step_limit']} |",
        f"| Median seconds per question | {agg['median_seconds']} |",
        "",
        "## By type",
        "",
        "| Type | Questions | Answer | Tool | Abstain | Hallucination |",
        "|---|---|---|---|---|---|",
    ]
    for kind, part in summary["by_type"].items():
        lines.append(
            f"| {kind} | {part['questions']} | {_pct(part['answer_exact_match'])} | {_pct(part['tool_correct'])} | "
            f"{_pct(part['abstain_correct'])} | {_pct(part['hallucination'])} |"
        )
    lines += ["", "## Per question", "", "| Id | Type | Tool | Answer | Abstain | Halluc. | Expected | Got |", "|---|---|---|---|---|---|---|---|"]
    for row in summary["items"]:
        halluc = ", ".join(row["hallucinated_numbers"]) if row["hallucinated"] else "-"
        got = row["answer"] if not row["error"] else f"ERROR {row['error']}"
        lines.append(
            f"| `{row['id']}` | {row['type']} | {_mark(row['tool_correct'])} | {_mark(row['answer_correct'])} | "
            f"{_mark(row['abstain_correct'])} | {halluc} | {_cell(row['expected'], 24)} | {_cell(got)} |"
        )
    if summary["not_run"]:
        lines += ["", f"Not run: {', '.join(summary['not_run'])}"]
    return "\n".join(lines) + "\n"


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="Grade an eval run directory.")
    parser.add_argument("run_dir", type=Path)
    parser.add_argument("--golden", type=Path, default=GOLDEN)
    args = parser.parse_args(argv)
    if not args.run_dir.is_dir():
        print(f"grade: {args.run_dir} is not a directory", file=sys.stderr)
        return 2
    summary = grade_run(args.run_dir, args.golden)
    sys.stdout.reconfigure(encoding="utf-8", errors="replace")
    print((args.run_dir / "summary.md").read_text(encoding="utf-8"))
    return 0 if summary["aggregate"]["questions"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
