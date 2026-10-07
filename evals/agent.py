"""The bridge to the production agent.

The agent is `examples/ollama-host/host.py`: it starts wh-mcp, shows its tools to
an Ollama model, and runs the loop. The evals do not have a chat loop of their
own. They import that module and use its `Host`, `OllamaChat`, and helpers, so
what is measured is the code that really runs.
"""

from __future__ import annotations

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
HOST_DIR = ROOT / "examples" / "ollama-host"

if str(HOST_DIR) not in sys.path:
    sys.path.insert(0, str(HOST_DIR))

import host  # noqa: E402  (needs the path above)
from questions import plain  # noqa: E402

__all__ = ["ROOT", "HOST_DIR", "host", "plain"]
