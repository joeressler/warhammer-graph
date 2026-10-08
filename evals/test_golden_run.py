"""The golden set, end to end, through the production agent and a real model.

    pytest evals -m eval --eval-model granite4.1:8b

This is the baseline run. It asserts that the pipeline works (every question has
a trace, the grader produced a summary), not that the model scores well: the
score is the measurement, recorded in `summary.md`.
"""

from __future__ import annotations

import asyncio
import json

import pytest

from evals.agent import ROOT, host
from evals.grade import GOLDEN, grade_run, load_golden
from evals.run import RESULTS, make_run_id, preflight, run_golden, select

pytestmark = pytest.mark.eval


def _server(backend: str, base_url: str | None) -> dict:
    """What llama-server reports about itself, recorded in run.json. Empty for Ollama."""
    if backend != "llamacpp":
        return {}
    from llamacpp import server_info

    return asyncio.run(server_info(base_url))


def test_the_golden_set_runs_through_the_real_agent_and_is_graded(request: pytest.FixtureRequest) -> None:
    option = request.config.getoption
    model = option("--eval-model") or host.DEFAULT_MODEL
    real_bundle = ROOT / "bundle"
    if not (real_bundle / "graph.db").exists():
        pytest.skip("build the real bundle first: python scripts/setup.py")
    backend, base_url = option("--eval-backend"), option("--eval-base-url")
    problem = asyncio.run(preflight(model, None, backend, base_url))
    if problem:
        pytest.skip(problem)

    items = select(load_golden(GOLDEN), option("--eval-ids"), None)
    run_id = option("--eval-run-id") or make_run_id(model, backend=backend)
    results_dir = option("--eval-results-dir") or RESULTS
    directory = asyncio.run(
        run_golden(
            items, model=model, run_id=run_id, results_dir=results_dir, bundle=real_bundle, temperature=option("--eval-temperature"),
            backend=backend, base_url=base_url, server=_server(backend, base_url),
        )
    )

    summary = grade_run(directory)
    assert sorted(path.stem for path in directory.glob("*.json") if path.stem not in ("run", "summary")) == sorted(i["id"] for i in items)
    assert summary["aggregate"]["questions"] == len(items)
    assert (directory / "summary.md").exists()
    assert json.loads((directory / "run.json").read_text(encoding="utf-8"))["status"] == "complete"
    print(f"\n{directory / 'summary.md'}\n{json.dumps(summary['aggregate'], indent=2)}")
