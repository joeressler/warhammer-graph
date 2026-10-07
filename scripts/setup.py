"""One-step setup: download the export, build the graph bundle, build the MCP server.

Run from anywhere with:  python scripts/setup.py

It runs the same commands the guides list, one after another, and stops at the
first one that fails. Every step is a separate program started with `subprocess`,
so Python never needs to know how the Rust side works. Standard library only.
"""

from __future__ import annotations

import argparse
import importlib.util
import os
import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
MIN_PYTHON = (3, 12)
EXE = ".exe" if os.name == "nt" else ""
MCP_NAME = "warhammer"


class SetupError(Exception):
    """A step failed in a way the user can fix. The message says how."""


def release_binary(root: Path, name: str) -> Path:
    return root / "target" / "release" / f"{name}{EXE}"


def check_tools(which=shutil.which, version=sys.version_info) -> None:
    """Fail early, with the fix, when Python or Rust is missing."""
    if tuple(version[:2]) < MIN_PYTHON:
        need = ".".join(map(str, MIN_PYTHON))
        raise SetupError(f"Python {need} or newer is required (this is {version[0]}.{version[1]}).")
    if which("cargo") is None:
        raise SetupError("Rust is not installed (`cargo` was not found). Install it from https://rustup.rs/ and run this again.")


def plan(root: Path, edition: str, skip_export: bool, install: bool, register: bool) -> list[tuple[str, list[str]]]:
    """The commands to run, in order, each with a line saying what it does."""
    py = sys.executable
    corpus, cache, bundle = root / "corpus", root / "cache", root / "bundle"
    graph = str(release_binary(root, "wh-graph"))
    mcp = str(release_binary(root, "wh-mcp"))
    steps: list[tuple[str, list[str]]] = []
    if install:
        steps.append(("Installing the wh-corpus package", [py, "-m", "pip", "install", "-e", str(root)]))
    if not skip_export:
        steps.append((
            "Downloading the Wahapedia export and writing the corpus (needs the network)",
            [py, "-m", "wh_corpus", "export", "--edition", edition, "--cache-dir", str(cache), "--out", str(corpus)],
        ))
    steps.append((
        "Compiling wh-graph and wh-mcp (the first build takes a few minutes)",
        ["cargo", "build", "--release", "-p", "wh-graph", "-p", "wh-mcp"],
    ))
    steps.append(("Building the graph bundle", [graph, "build", "--corpus", str(corpus), "--out", str(bundle)]))
    steps.append(("Checking the bundle", [graph, "validate", "--bundle", str(bundle)]))
    if register:
        steps.append((
            "Registering the server with Claude Code",
            ["claude", "mcp", "add", "--scope", "user", MCP_NAME, "--", mcp, "--bundle", str(bundle)],
        ))
    return steps


def run_step(title: str, command: list[str], cwd: Path, runner=subprocess.run) -> None:
    print(f"\n==> {title}\n    {' '.join(command)}", flush=True)
    try:
        result = runner(command, cwd=cwd)
    except FileNotFoundError:
        raise SetupError(f"Could not start `{command[0]}`. Is it installed and on your PATH?") from None
    if result.returncode != 0:
        raise SetupError(f"Step failed with exit code {result.returncode}: {title}")


def main(argv: list[str] | None = None, runner=subprocess.run, which=shutil.which, root: Path = ROOT) -> int:
    parser = argparse.ArgumentParser(description="Download the data, build the graph bundle, and build the MCP server.")
    parser.add_argument("--edition", default="10ed", help="Wahapedia edition (default 10ed).")
    parser.add_argument("--skip-export", action="store_true", help="Reuse the existing ./corpus instead of downloading.")
    parser.add_argument("--no-install", action="store_true", help="Do not pip-install wh-corpus (use if it is already installed).")
    parser.add_argument("--register", action="store_true", help="Also register the server with Claude Code (user scope, all projects on this machine).")
    args = parser.parse_args(argv)

    try:
        check_tools(which)
        if args.register and which("claude") is None:
            raise SetupError("`claude` was not found, so --register cannot run. Install Claude Code, or leave --register off.")
        # Skip the install when the package is already importable.
        install = not args.no_install and importlib.util.find_spec("wh_corpus") is None
        if sys.prefix == sys.base_prefix and install:
            print("Note: no virtual environment is active, so wh-corpus installs into this Python.")
        for title, command in plan(root, args.edition, args.skip_export, install, args.register):
            run_step(title, command, root, runner)
    except SetupError as error:
        print(f"\nSetup stopped: {error}", file=sys.stderr)
        return 1

    mcp, bundle = release_binary(root, "wh-mcp"), root / "bundle"
    print("\nDone. The bundle is in ./bundle and the server is built.")
    if not args.register:
        print("To use it from Claude Code, register it once:\n")
        print(f'    claude mcp add --scope user {MCP_NAME} -- "{mcp}" --bundle "{bundle}"')
    return 0


if __name__ == "__main__":
    sys.exit(main())
