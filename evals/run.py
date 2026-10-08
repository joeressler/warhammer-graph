"""Run the golden questions through the production agent and save a trace for each.

    python -m evals.run --model granite4.1:8b
    python -m evals.run --model granite4.1:8b --ids lookup_angron_toughness,count_factions
    python -m evals.run --backend llamacpp --model granite4.1:8b     # llama-server on :8080
    pytest evals -m eval --eval-model granite4.1:8b

Each question gets a fresh conversation (`host.Host`) over one running wh-mcp
server. The model server (Ollama, or llama.cpp's llama-server with
`--backend llamacpp`) only serves the model. The loop that calls the tools is the one in
`examples/ollama-host/host.py`. Traces go to `evals/results/<run_id>/<id>.json`,
then `evals.grade` writes `summary.json` and `summary.md` beside them.

Exit codes: 0 the run finished (whatever the score), 1 the agent path failed hard
partway (the partial traces are saved and graded), 2 it could not start.
"""

from __future__ import annotations

import argparse
import asyncio
import hashlib
import json
import re
import subprocess
import sys
import time
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Callable

import httpx
import ollama

from evals.agent import ROOT, host
from evals.grade import GOLDEN, grade_run, grade_trace, load_golden

RESULTS = Path(__file__).resolve().parent / "results"
ChatFactory = Callable[[str], host.ChatFn]


class HardFailure(Exception):
    """The agent path itself broke (Ollama or the server is unreachable), not one bad answer."""


def is_hard(error: BaseException) -> bool:
    """A connection failure or a missing model ends the run. A timeout or a bad answer does not."""
    if isinstance(error, (asyncio.TimeoutError, TimeoutError)):
        return False
    if isinstance(error, ollama.ResponseError):
        return error.status_code in (404, 401, 403)
    return isinstance(error, (ConnectionError, httpx.TransportError, OSError))


def make_run_id(model: str, now: datetime | None = None, backend: str = "ollama") -> str:
    """`<UTC timestamp>_<model>`, with `_<backend>` added for anything but Ollama."""
    stamp = (now or datetime.now(timezone.utc)).strftime("%Y%m%d-%H%M%S")
    suffix = "" if backend == "ollama" else f"_{backend}"
    return f"{stamp}_{re.sub(r'[^A-Za-z0-9.-]+', '-', model)}{suffix}"


def build_trace(item: dict[str, Any], run_id: str, model: str, conversation: Any, answer: Any, seconds: float, error: str) -> dict[str, Any]:
    """The full record of one question: messages, tool calls with their results, and the final text.

    Built from the conversation's messages, so a partial trace (a timeout, a
    crash) still has everything that happened up to that point.
    """
    calls: list[dict[str, Any]] = []
    results = [m["content"] for m in conversation.messages if m["role"] == "tool"]
    for message in conversation.messages:
        for call in message.get("tool_calls", []) if message["role"] == "assistant" else []:
            calls.append({"index": len(calls), "tool": call["function"]["name"], "arguments": call["function"]["arguments"]})
    for position, call in enumerate(calls):
        call["result"] = results[position] if position < len(results) else ""
        call["result_chars"] = len(call["result"])
        if answer is not None and position < len(answer.steps):
            call["is_error"], call["seconds"] = answer.steps[position].is_error, round(answer.steps[position].seconds, 3)
    return {
        "id": item["id"],
        "run_id": run_id,
        "model": model,
        "type": item["type"],
        "question": item["question"],
        "seconds": round(seconds, 2),
        "final_text": answer.text if answer is not None else "",
        "stopped": answer.stopped if answer is not None else "error",
        "error": error,
        "tool_calls": calls,
        "messages": conversation.messages,
    }


def git_commit() -> str:
    try:
        return subprocess.run(["git", "rev-parse", "HEAD"], cwd=ROOT, capture_output=True, text=True, check=True).stdout.strip()
    except Exception:
        return ""


def fingerprint(bundle: Path) -> str:
    try:
        return json.loads((bundle / "manifest.json").read_text(encoding="utf-8")).get("corpus_fingerprint", "")
    except Exception:
        return ""


async def run_golden(
    items: list[dict[str, Any]],
    *,
    model: str,
    run_id: str,
    results_dir: Path = RESULTS,
    bundle: Path = ROOT / "bundle",
    exe: Path | None = None,
    chat_factory: ChatFactory | None = None,
    temperature: float = 0.0,
    num_ctx: int = host.DEFAULT_NUM_CTX,
    max_steps: int = host.DEFAULT_MAX_STEPS,
    max_tool_chars: int = host.DEFAULT_MAX_TOOL_CHARS,
    timeout: float = 240.0,
    ollama_host: str | None = None,
    backend: str = "ollama",
    base_url: str | None = None,
    server: dict[str, Any] | None = None,
    golden_path: Path = GOLDEN,
    progress: Callable[[str], None] | None = None,
) -> Path:
    """Ask every item in a fresh conversation, saving a trace for each. Returns the run directory.

    Raises HardFailure after saving the partial traces if the agent path breaks.
    """
    directory = results_dir / run_id
    directory.mkdir(parents=True, exist_ok=True)
    url = base_url if backend == "llamacpp" else ollama_host
    chat_factory = chat_factory or (lambda name: host.make_chat(backend, name, url=url, num_ctx=num_ctx, temperature=temperature))
    server = server or {}
    meta: dict[str, Any] = {
        "run_id": run_id, "model": model, "backend": backend, "temperature": temperature,
        # llama-server fixes its own context window when it starts, so report the one it reports.
        "num_ctx": server.get("n_ctx") or num_ctx, "server": server, "max_steps": max_steps,
        "max_tool_chars": max_tool_chars, "timeout": timeout, "questions": len(items), "git_commit": git_commit(),
        "corpus_fingerprint": fingerprint(bundle), "golden_sha256": hashlib.sha256(golden_path.read_bytes()).hexdigest() if golden_path.exists() else "",
        "started_at": datetime.now(timezone.utc).isoformat(timespec="seconds"), "status": "running",
    }
    save = lambda: (directory / "run.json").write_text(json.dumps(meta, indent=2), encoding="utf-8")
    save()
    say = progress or (lambda line: None)
    failure: HardFailure | None = None
    try:
        async with host.Client(host.server_params(exe or host.find_executable(), bundle), mode="legacy") as client:
            tools = host.to_ollama_tools((await client.list_tools()).tools)
            chat = chat_factory(model)
            for number, item in enumerate(items, 1):
                conversation = host.Host(
                    client, chat, tools, host.system_prompt(client.instructions),
                    max_steps=max_steps, max_tool_chars=max_tool_chars,
                )
                started, answer, error, hard = time.monotonic(), None, "", None
                try:
                    answer = await asyncio.wait_for(conversation.ask(item["question"]), timeout=timeout)
                except Exception as problem:  # a timeout or one bad reply is a failed question, not a failed run
                    error = f"{type(problem).__name__}: {problem}"
                    hard = problem if is_hard(problem) else None
                trace = build_trace(item, run_id, model, conversation, answer, time.monotonic() - started, error)
                (directory / f"{item['id']}.json").write_text(json.dumps(trace, indent=2, ensure_ascii=False), encoding="utf-8")
                verdict = grade_trace(item, trace)
                right = verdict["answer_correct"] if verdict["answer_correct"] is not None else verdict["abstain_correct"]
                say(f"[{number}/{len(items)}] {item['id']:<44} {'ok  ' if right else 'FAIL'} {trace['seconds']:>5.1f}s {error}")
                if hard is not None:
                    # Leave the `async with` first: raising inside it would reach the caller
                    # wrapped in an exception group from the MCP client's task group.
                    failure = HardFailure(f"{item['id']}: {error}")
                    break
        meta["status"] = "aborted" if failure else "complete"
        if failure:
            meta["failure"] = str(failure)
    except BaseException as crash:
        meta["status"], meta["failure"] = "failed", f"{type(crash).__name__}: {crash}"
        raise
    finally:
        meta["finished_at"] = datetime.now(timezone.utc).isoformat(timespec="seconds")
        save()
    if failure:
        raise failure
    return directory


async def preflight(model: str, ollama_host: str | None, backend: str = "ollama", base_url: str | None = None) -> str | None:
    """None if the model is ready, otherwise a message saying what to fix."""
    if backend == "llamacpp":
        from llamacpp import DEFAULT_BASE_URL, server_info

        try:
            await server_info(base_url)
        except Exception as error:
            return (
                f"Could not reach llama-server at {base_url or DEFAULT_BASE_URL} ({type(error).__name__}). "
                "Start it with: python examples/ollama-host/serve_llamacpp.py --ollama-model <name>"
            )
        return None
    try:
        await ollama.AsyncClient(host=ollama_host).show(model)
    except ollama.ResponseError as error:
        return f"Ollama does not have {model!r} ({error}). Pull it with: ollama pull {model}"
    except Exception as error:
        return f"Could not reach Ollama ({type(error).__name__}). Start it, or pass --ollama-host."
    return None


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--backend", choices=host.BACKENDS, default="ollama", help="Model server (default ollama).")
    parser.add_argument("--model", default=host.DEFAULT_MODEL, help="Model name; for llama-server it is only a label.")
    parser.add_argument("--run-id", default=None, help="Default: <UTC timestamp>_<model>, plus _llamacpp for that backend.")
    parser.add_argument("--results-dir", type=Path, default=RESULTS)
    parser.add_argument("--golden", type=Path, default=GOLDEN)
    parser.add_argument("--ids", default=None, help="Comma-separated question ids (default: all).")
    parser.add_argument("--limit", type=int, default=None, help="Only the first N questions.")
    parser.add_argument("--bundle", type=Path, default=ROOT / "bundle")
    parser.add_argument("--exe", type=Path, default=None)
    parser.add_argument("--ollama-host", default=None)
    parser.add_argument("--base-url", default=None, help="llama-server URL for --backend llamacpp (default http://localhost:8080).")
    parser.add_argument("--temperature", type=float, default=0.0)
    parser.add_argument("--num-ctx", type=int, default=host.DEFAULT_NUM_CTX)
    parser.add_argument("--max-steps", type=int, default=host.DEFAULT_MAX_STEPS)
    parser.add_argument("--max-tool-chars", type=int, default=host.DEFAULT_MAX_TOOL_CHARS)
    parser.add_argument("--timeout", type=float, default=240.0, help="Seconds allowed per question.")
    parser.add_argument("--no-grade", action="store_true", help="Save traces only.")
    return parser.parse_args(argv)


def select(items: list[dict[str, Any]], ids: str | None, limit: int | None) -> list[dict[str, Any]]:
    if ids:
        wanted = [name.strip() for name in ids.split(",") if name.strip()]
        unknown = [name for name in wanted if name not in {i["id"] for i in items}]
        if unknown:
            raise SystemExit(f"run: unknown question id(s): {', '.join(unknown)}")
        items = [i for i in items if i["id"] in wanted]
    return items[:limit] if limit else items


async def amain(args: argparse.Namespace) -> int:
    exe = args.exe or host.find_executable()
    if not exe.exists():
        print(f"run: {exe} not found. Build it with: cargo build -p wh-mcp --release", file=sys.stderr)
        return 2
    if not (args.bundle / "graph.db").exists():
        print(f"run: no bundle at {args.bundle}. Build it with: python scripts/setup.py", file=sys.stderr)
        return 2
    problem = await preflight(args.model, args.ollama_host, args.backend, args.base_url)
    if problem:
        print(f"run: {problem}", file=sys.stderr)
        return 2
    server: dict[str, Any] = {}
    if args.backend == "llamacpp":
        from llamacpp import server_info

        server = await server_info(args.base_url)
    items = select(load_golden(args.golden), args.ids, args.limit)
    run_id = args.run_id or make_run_id(args.model, backend=args.backend)
    print(f"run {run_id}: {len(items)} questions with {args.model} ({args.backend})", file=sys.stderr)
    status = 0
    try:
        directory = await run_golden(
            items, model=args.model, run_id=run_id, results_dir=args.results_dir, bundle=args.bundle, exe=exe,
            temperature=args.temperature, num_ctx=args.num_ctx, max_steps=args.max_steps, max_tool_chars=args.max_tool_chars,
            timeout=args.timeout, ollama_host=args.ollama_host, backend=args.backend, base_url=args.base_url,
            server=server, golden_path=args.golden,
            progress=lambda line: print(line, file=sys.stderr, flush=True),
        )
    except HardFailure as failure:
        print(f"run: the agent path failed, so the run stopped early: {failure}", file=sys.stderr)
        directory, status = args.results_dir / run_id, 1
    if not args.no_grade and any(directory.glob("*.json")):
        summary = grade_run(directory, args.golden)
        agg = summary["aggregate"]
        print(
            f"answer exact match {agg['answer_exact_match']['pct']}%  tool correct {agg['tool_correct']['pct']}%  "
            f"abstain correct {agg['abstain_correct']['pct']}%  hallucination {agg['hallucination']['pct']}%",
            file=sys.stderr,
        )
    print(directory)
    return status


def main() -> int:
    host.use_utf8(sys.stdout, sys.stderr)
    return asyncio.run(amain(parse_args()))


if __name__ == "__main__":
    raise SystemExit(main())
