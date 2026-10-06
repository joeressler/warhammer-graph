"""Shared fixtures: the real wh-mcp binary and a small synthetic bundle."""

from __future__ import annotations

import subprocess
import sys
from pathlib import Path

import pytest

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent))

import host  # noqa: E402

FIXTURE_CORPUS = host.ROOT / "crates" / "wh-graph" / "tests" / "fixtures" / "rich"


def _find(name: str) -> Path | None:
    suffix = ".exe" if sys.platform == "win32" else ""
    for profile in ("release", "debug"):
        candidate = host.ROOT / "target" / profile / f"{name}{suffix}"
        if candidate.exists():
            return candidate
    return None


@pytest.fixture(scope="session")
def exe() -> Path:
    found = _find("wh-mcp")
    if found is None:
        pytest.skip("build the server first: cargo build -p wh-mcp")
    return found


@pytest.fixture(scope="session")
def bundle(tmp_path_factory: pytest.TempPathFactory) -> Path:
    """A bundle built from the repository's synthetic corpus ("Example Unit")."""
    graph = _find("wh-graph")
    if graph is None:
        pytest.skip("build the bundle builder first: cargo build -p wh-graph")
    out = tmp_path_factory.mktemp("bundle") / "bundle"
    subprocess.run([str(graph), "build", "--corpus", str(FIXTURE_CORPUS), "--out", str(out)], check=True, capture_output=True)
    return out
