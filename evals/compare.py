"""A comparison table across graded runs.

    python -m evals.compare evals/results/*

Reads `summary.json` and `run.json` from each directory given (grade them first
with `python -m evals.grade`). Only completed runs are shown, and when a model
and server appear more than once, the latest run is used. Prints Markdown.
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any


def load(directory: Path) -> dict[str, Any] | None:
    """One run's row, or None if it is not a finished, graded run."""
    try:
        meta = json.loads((directory / "run.json").read_text(encoding="utf-8"))
        summary = json.loads((directory / "summary.json").read_text(encoding="utf-8"))
    except (OSError, ValueError):
        return None
    if meta.get("status") != "complete":
        return None
    agg = summary["aggregate"]
    return {
        "run_id": directory.name,
        "model": meta["model"],
        "backend": meta.get("backend", "ollama"),
        "questions": agg["questions"],
        "answer": agg["answer_exact_match"],
        "tool": agg["tool_correct"],
        "abstain": agg["abstain_correct"],
        "hallucination": agg["hallucination"],
        "median": agg["median_seconds"],
        "errors": agg["errors"],
        "step_limit": agg["step_limit"],
    }


def latest(rows: list[dict[str, Any]]) -> list[dict[str, Any]]:
    """The most recent run for each (model, backend). Run ids start with a timestamp, so they sort."""
    newest: dict[tuple[str, str], dict[str, Any]] = {}
    for row in sorted(rows, key=lambda r: r["run_id"]):
        newest[(row["model"], row["backend"])] = row
    return sorted(newest.values(), key=lambda r: (r["model"], r["backend"]))


def _pct(rate: dict[str, Any]) -> str:
    return "-" if rate["pct"] is None else f"{rate['pct']}%"


def table(rows: list[dict[str, Any]]) -> str:
    lines = [
        "| Model | Server | Answer | Tool | Abstain | Hallucination | Median s | Errors | Step limit |",
        "|---|---|---|---|---|---|---|---|---|",
    ]
    for r in rows:
        lines.append(
            f"| `{r['model']}` | {r['backend']} | {_pct(r['answer'])} | {_pct(r['tool'])} | {_pct(r['abstain'])} | "
            f"{_pct(r['hallucination'])} | {r['median']} | {r['errors']} | {r['step_limit']} |"
        )
    return "\n".join(lines) + "\n"


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("run_dirs", nargs="+", type=Path)
    args = parser.parse_args(argv)
    rows = latest([row for row in map(load, (d for d in args.run_dirs if d.is_dir())) if row])
    if not rows:
        print("compare: no completed, graded runs in the directories given", file=sys.stderr)
        return 1
    sys.stdout.reconfigure(encoding="utf-8", errors="replace")
    print(table(rows))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
