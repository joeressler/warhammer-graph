"""pytest wiring for the evals.

Fixtures `exe` (the built wh-mcp) and `bundle` (a small synthetic bundle) are the
ones the Ollama host's own tests use, loaded from their conftest rather than
copied. Tests marked `eval` talk to a real model and run only when selected:

    pytest evals -m eval --eval-model granite4.1:8b
"""

from __future__ import annotations

import importlib.util
from pathlib import Path

import pytest

from evals.agent import HOST_DIR

_spec = importlib.util.spec_from_file_location("ollama_host_conftest", HOST_DIR / "tests" / "conftest.py")
_host_conftest = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(_host_conftest)
exe = _host_conftest.exe
bundle = _host_conftest.bundle


def pytest_addoption(parser: pytest.Parser) -> None:
    group = parser.getgroup("eval")
    group.addoption("--eval-model", default=None, help="Model for -m eval (default: the host's default).")
    group.addoption("--eval-backend", choices=("ollama", "llamacpp"), default="ollama", help="Model server for -m eval.")
    group.addoption("--eval-base-url", default=None, help="llama-server URL for --eval-backend llamacpp.")
    group.addoption("--eval-run-id", default=None, help="Run id (default: timestamp + model).")
    group.addoption("--eval-ids", default=None, help="Comma-separated question ids (default: all).")
    group.addoption("--eval-temperature", type=float, default=0.0)
    group.addoption("--eval-results-dir", type=Path, default=None)


def pytest_collection_modifyitems(config: pytest.Config, items: list[pytest.Item]) -> None:
    """Live evals are skipped unless `-m eval` selects them."""
    if "eval" in (config.getoption("-m") or ""):
        return
    skip = pytest.mark.skip(reason="live eval: run with -m eval (needs Ollama, the real bundle, and wh-mcp)")
    for item in items:
        if "eval" in item.keywords:
            item.add_marker(skip)
