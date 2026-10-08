"""Start llama.cpp's `llama-server` on a model Ollama already downloaded.

    python examples/ollama-host/serve_llamacpp.py --ollama-model granite4.1:8b
    python examples/ollama-host/serve_llamacpp.py --gguf path/to/model.gguf --alias my-model

Ollama stores each model as a plain GGUF file (a "blob"). llama-server can load
that file directly, so the same weights can be served by either program and a
comparison between them is fair. Nothing is copied.

`--jinja` is always passed: without it llama-server ignores the model's chat
template and tool calling does not work. The window is set here with `-c`
(Ollama sets it per request instead). Stop the server with Ctrl+C.

llama-server is found, in order, from `--llama-server`, the LLAMA_SERVER
environment variable, PATH, then the newest `~/llama.cpp/*/llama-server`.
Download a build from https://github.com/ggml-org/llama.cpp/releases (on this
machine, the `win-cuda-13` zip plus its `cudart` zip).
"""

from __future__ import annotations

import argparse
import json
import os
import shutil
import subprocess
import sys
from pathlib import Path

DEFAULT_CTX = 16384
DEFAULT_PORT = 8080


class ServeError(Exception):
    """Something the user can fix; the message says what."""


def models_dir() -> Path:
    """Ollama's model store: OLLAMA_MODELS, or ~/.ollama/models."""
    return Path(os.environ.get("OLLAMA_MODELS") or Path.home() / ".ollama" / "models")


def resolve_blob(name: str, store: Path | None = None) -> Path:
    """The GGUF file behind an Ollama model name such as `granite4.1:8b` or `library/gpt-oss:20b`."""
    store = store or models_dir()
    repo, _, tag = name.partition(":")
    repo = repo if "/" in repo else f"library/{repo}"
    manifest = store / "manifests" / "registry.ollama.ai" / repo / (tag or "latest")
    if not manifest.exists():
        raise ServeError(f"Ollama has no model {name!r} in {store}. Pull it first: ollama pull {name}")
    for layer in json.loads(manifest.read_text(encoding="utf-8"))["layers"]:
        if layer["mediaType"].endswith("image.model"):
            blob = store / "blobs" / layer["digest"].replace(":", "-")
            if not blob.exists():
                raise ServeError(f"{name!r} is listed but its file is missing: {blob}")
            return blob
    raise ServeError(f"{name!r} has no model file in its manifest")


def find_server(explicit: str | None = None) -> Path:
    """Locate llama-server (see the module docstring for the search order)."""
    exe = "llama-server.exe" if os.name == "nt" else "llama-server"
    for candidate in (explicit, os.environ.get("LLAMA_SERVER")):
        if candidate:
            path = Path(candidate)
            if not path.exists():
                raise ServeError(f"llama-server not found at {path}")
            return path
    on_path = shutil.which("llama-server")
    if on_path:
        return Path(on_path)
    builds = sorted((Path.home() / "llama.cpp").glob(f"*/{exe}"))
    if builds:
        return builds[-1]
    raise ServeError(
        "llama-server not found. Download a build from https://github.com/ggml-org/llama.cpp/releases, "
        "then pass --llama-server, set LLAMA_SERVER, or put it on PATH."
    )


def build_command(server: Path, model: Path, alias: str, *, ctx: int, port: int, gpu_layers: int, extra: list[str]) -> list[str]:
    return [
        str(server), "-m", str(model), "--jinja", "-c", str(ctx), "-ngl", str(gpu_layers),
        "--port", str(port), "--alias", alias, *extra,
    ]


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    source = parser.add_mutually_exclusive_group(required=True)
    source.add_argument("--ollama-model", help="An Ollama model name, e.g. granite4.1:8b.")
    source.add_argument("--gguf", type=Path, help="A GGUF file.")
    parser.add_argument("--alias", default=None, help="Name the server reports (default: the Ollama name, or the file name).")
    parser.add_argument("--llama-server", default=None, help="Path to llama-server.")
    parser.add_argument("-c", "--ctx", type=int, default=DEFAULT_CTX, help=f"Context window in tokens (default {DEFAULT_CTX}).")
    parser.add_argument("--port", type=int, default=DEFAULT_PORT)
    parser.add_argument("--ngl", type=int, default=99, help="Layers to offload to the GPU (default 99: all).")
    parser.add_argument("--dry-run", action="store_true", help="Print the command and exit.")
    parser.add_argument("extra", nargs="*", help="More llama-server flags, after `--`.")
    args = parser.parse_args(argv)
    try:
        model = resolve_blob(args.ollama_model) if args.ollama_model else args.gguf
        if not model.exists():
            raise ServeError(f"no such file: {model}")
        command = build_command(
            find_server(args.llama_server), model, args.alias or args.ollama_model or model.stem,
            ctx=args.ctx, port=args.port, gpu_layers=args.ngl, extra=args.extra,
        )
    except ServeError as error:
        print(f"serve: {error}", file=sys.stderr)
        return 2
    print(" ".join(command), file=sys.stderr)
    if args.dry_run:
        return 0
    try:
        return subprocess.run(command).returncode
    except KeyboardInterrupt:
        return 0


if __name__ == "__main__":
    raise SystemExit(main())
