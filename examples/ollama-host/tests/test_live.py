"""Live checks against a real Ollama model and the real bundle.

Skipped unless OLLAMA_LIVE=1, because they need Ollama running, the model pulled,
and `bundle/` built, and because a local model is not deterministic. Run them with:

    OLLAMA_LIVE=1 python -m pytest examples/ollama-host/tests/test_live.py -v
    OLLAMA_LIVE=1 OLLAMA_MODEL=granite4.1:3b python -m pytest ...
"""

from __future__ import annotations

import argparse
import asyncio
import os

import pytest
from mcp import Client

import host
from evaluate import run_one
from questions import BY_ID

pytestmark = pytest.mark.skipif(os.environ.get("OLLAMA_LIVE") != "1", reason="set OLLAMA_LIVE=1 to talk to a real model")

MODEL = os.environ.get("OLLAMA_MODEL", host.DEFAULT_MODEL)
# Questions a capable 8B model should answer every time: one lookup each, and a fact in a typed field.
CORE = ("angron_invuln", "warboss_points", "imotekh_leads")


@pytest.fixture(scope="module")
def real_bundle():
    bundle = host.ROOT / "bundle"
    if not (bundle / "graph.db").exists():
        pytest.skip("build the real bundle first: cargo run -p wh-graph -- build --corpus ./corpus --out ./bundle")
    return bundle


@pytest.mark.parametrize("question_id", CORE)
def test_the_model_answers_a_core_question_from_the_tools(exe, real_bundle, question_id: str) -> None:
    args = argparse.Namespace(
        backend="ollama", base_url=None, ollama_host=None, num_ctx=host.DEFAULT_NUM_CTX, temperature=0.0,
        max_steps=host.DEFAULT_MAX_STEPS, max_tool_chars=host.DEFAULT_MAX_TOOL_CHARS, timeout=240.0,
    )

    async def go():
        async with Client(host.server_params(exe, real_bundle), mode="legacy") as client:
            tools = host.to_ollama_tools((await client.list_tools()).tools)
            return await run_one(client, tools, MODEL, BY_ID[question_id], 1, args)

    result = asyncio.run(go())
    assert result.tools_ok, f"{MODEL} did not use the expected tool: called {result.tools}; {result.error}"
    assert result.text_ok, f"{MODEL} answered without the expected fact: {result.answer!r}"
