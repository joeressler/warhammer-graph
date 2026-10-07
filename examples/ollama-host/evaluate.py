"""Compare local Ollama models on the question set and print a results table.

    python examples/ollama-host/evaluate.py --models granite4.1:8b,granite4.1:3b --runs 2

Needs Ollama running, the models pulled, the real bundle built (`wh-graph build`),
and wh-mcp built. Local models are not deterministic, so each question is run
several times and the table shows passes out of runs.
"""

from __future__ import annotations

import argparse
import asyncio
import json
import statistics
import sys
import time
from dataclasses import asdict, dataclass, field
from pathlib import Path
from typing import Any

from mcp import Client

import host
from questions import BY_ID, QUESTIONS, Question, passes


@dataclass
class Result:
    model: str
    question: str
    run: int
    passed: bool
    tools_ok: bool
    text_ok: bool
    seconds: float
    tools: list[str] = field(default_factory=list)
    stopped: str = "answered"
    error: str = ""
    answer: str = ""


async def run_one(client: Client, tools: list[dict[str, Any]], model: str, question: Question, run: int, args: argparse.Namespace) -> Result:
    url = args.base_url if args.backend == "llamacpp" else args.ollama_host
    chat = host.make_chat(args.backend, model, url=url, num_ctx=args.num_ctx, temperature=args.temperature)
    conversation = host.Host(client, chat, tools, host.system_prompt(client.instructions), max_steps=args.max_steps, max_tool_chars=args.max_tool_chars)
    started = time.monotonic()
    try:
        answer = await asyncio.wait_for(conversation.ask(question.text), timeout=args.timeout)
    except Exception as error:  # a timeout or a model/server failure is a failed run, not a crash
        return Result(model, question.id, run, False, False, False, time.monotonic() - started, error=f"{type(error).__name__}: {error}")
    tools_ok, text_ok = passes(question, answer.text, {step.tool for step in answer.steps})
    return Result(
        model, question.id, run, tools_ok and text_ok, tools_ok, text_ok, time.monotonic() - started,
        tools=[step.tool for step in answer.steps], stopped=answer.stopped, answer=answer.text,
    )


async def evaluate(args: argparse.Namespace) -> list[Result]:
    exe = args.exe or host.find_executable()
    questions = [BY_ID[name] for name in args.questions.split(",")] if args.questions else list(QUESTIONS)
    results: list[Result] = []
    async with Client(host.server_params(exe, args.bundle), mode="legacy") as client:
        tools = host.to_ollama_tools((await client.list_tools()).tools)
        for model in args.models.split(","):
            for question in questions:
                for run in range(1, args.runs + 1):
                    result = await run_one(client, tools, model, question, run, args)
                    results.append(result)
                    verdict = "PASS" if result.passed else "FAIL"
                    why = "" if result.passed else f" (tools_ok={result.tools_ok} text_ok={result.text_ok} {result.error})"
                    print(f"{model:<22}{question.id:<20}run {run} {verdict} {result.seconds:5.1f}s tools={result.tools}{why}", file=sys.stderr, flush=True)
    return results


def table(results: list[Result], models: list[str], runs: int) -> str:
    """A markdown table of passes out of runs, with the median time per model."""
    lines = ["| Question | " + " | ".join(models) + " |", "|---|" + "---|" * len(models)]
    for question in dict.fromkeys(result.question for result in results):
        cells = []
        for model in models:
            mine = [r for r in results if r.model == model and r.question == question]
            cells.append(f"{sum(r.passed for r in mine)}/{len(mine)}")
        lines.append(f"| `{question}` | " + " | ".join(cells) + " |")
    totals, medians = [], []
    for model in models:
        mine = [r for r in results if r.model == model]
        totals.append(f"**{sum(r.passed for r in mine)}/{len(mine)}**")
        medians.append(f"{statistics.median(r.seconds for r in mine):.0f} s" if mine else "-")
    lines.append("| **Total** | " + " | ".join(totals) + " |")
    lines.append("| Median time per question | " + " | ".join(medians) + " |")
    return "\n".join(lines)


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--models", default=host.DEFAULT_MODEL, help="Comma-separated Ollama models.")
    parser.add_argument("--runs", type=int, default=2, help="Runs per question per model.")
    parser.add_argument("--questions", default=None, help="Comma-separated question ids (default: all).")
    parser.add_argument("--bundle", type=Path, default=host.ROOT / "bundle")
    parser.add_argument("--exe", type=Path, default=None)
    parser.add_argument("--backend", choices=host.BACKENDS, default="ollama")
    parser.add_argument("--ollama-host", default=None)
    parser.add_argument("--base-url", default=None, help="llama-server URL for --backend llamacpp.")
    parser.add_argument("--num-ctx", type=int, default=host.DEFAULT_NUM_CTX)
    parser.add_argument("--temperature", type=float, default=0.0)
    parser.add_argument("--max-steps", type=int, default=host.DEFAULT_MAX_STEPS)
    parser.add_argument("--max-tool-chars", type=int, default=host.DEFAULT_MAX_TOOL_CHARS)
    parser.add_argument("--timeout", type=float, default=240.0, help="Seconds allowed per question.")
    parser.add_argument("--json", type=Path, default=None, help="Also write every run's details here.")
    return parser.parse_args(argv)


def main() -> int:
    host.use_utf8(sys.stdout, sys.stderr)
    args = parse_args()
    results = asyncio.run(evaluate(args))
    print(table(results, args.models.split(","), args.runs))
    if args.json:
        args.json.write_text(json.dumps([asdict(r) for r in results], indent=2, ensure_ascii=False), encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
