"""Get the model and the tools ready, then print everything needed to connect your own client.

Run from anywhere with:  python scripts/quickstart.py

It does these steps, skipping whatever is already done:

  1. Build the tools: runs scripts/setup.py (downloads the Wahapedia export, builds the graph
     bundle, compiles the wh-mcp server). Skipped when ./bundle and the wh-mcp binary exist.
  2. Download the model: the 2.1 GB Q4_K_M GGUF from Hugging Face into ./models, with resume, and
     check its SHA-256. Skipped when the file is there and correct (or pass --gguf).
  3. Install the host's Python packages (mcp, ollama, httpx) if they are missing.
  4. Save the system prompt the model expects to ./models/system-prompt.txt.
  5. Print a connection sheet: where everything is, the command that starts the model, the command or
     JSON that adds the tools to an MCP client, and the settings the model expects.

Nothing is left running unless you ask: --serve keeps the model server up, --ask / --chat try the model
through the bundled host first. --info only reprints the sheet.

The one thing it cannot fetch for you is llama-server itself (a prebuilt program, different for each
machine). Download a build from https://github.com/ggml-org/llama.cpp/releases (tested with b11476) and
put it on PATH, or pass --llama-server, or set LLAMA_SERVER. Standard library only.
"""

from __future__ import annotations

import argparse
import asyncio
import hashlib
import importlib.util
import json
import os
import subprocess
import sys
import time
import urllib.error
import urllib.request
from dataclasses import dataclass
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
HOST_DIR = ROOT / "examples" / "ollama-host"
EXE = ".exe" if os.name == "nt" else ""

MODEL_REPO = "jressler/granite-4.1-3b-wh40k-tools-GGUF"
MODEL_FILE = "granite-4.1-3b-wh40k-tools-sft-v1-Q4_K_M.gguf"
MODEL_SHA256 = "be6854561ff03ed3110390704a1b6edff88d3cf1eca9ef63c7c66e0a0e5e790a"
MODEL_BYTES = 2099501568
MODEL_URL = f"https://huggingface.co/{MODEL_REPO}/resolve/main/{MODEL_FILE}"
MODEL_ALIAS = "granite-wh40k"
HOST_PACKAGES = {"mcp": "mcp", "ollama": "ollama", "httpx": "httpx"}  # import name -> pip name
DEFAULT_QUESTION = "What is the Save characteristic of a Land Raider?"
CONTEXT = 16384


class QuickstartError(Exception):
    """Something the user can fix. The message says how."""


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1 << 20), b""):
            digest.update(block)
    return digest.hexdigest()


def mcp_binary(root: Path = ROOT) -> Path:
    return root / "target" / "release" / f"wh-mcp{EXE}"


def tools_built(root: Path = ROOT) -> bool:
    """The graph bundle and the wh-mcp binary are both there."""
    return (root / "bundle" / "graph.db").exists() and mcp_binary(root).exists()


def model_ok(path: Path) -> bool:
    """The file exists, is the right size, and has the right hash."""
    return path.exists() and path.stat().st_size == MODEL_BYTES and sha256_file(path) == MODEL_SHA256


def download(url: str, dest: Path, *, expected_bytes: int, opener=urllib.request.urlopen, out=sys.stderr) -> None:
    """Download `url` to `dest`, resuming a partial `dest.part` file from an earlier try."""
    dest.parent.mkdir(parents=True, exist_ok=True)
    part = dest.with_name(dest.name + ".part")
    have = part.stat().st_size if part.exists() else 0
    if have > expected_bytes:
        part.unlink()
        have = 0
    request = urllib.request.Request(url, headers={"Range": f"bytes={have}-"} if have else {})
    try:
        response = opener(request)
    except urllib.error.URLError as error:
        raise QuickstartError(f"Could not download the model from {url}: {error}") from None
    with response:
        if have and getattr(response, "status", 200) != 206:
            have = 0  # the server ignored the range request; start over
        mode = "ab" if have else "wb"
        done, last = have, 0.0
        with part.open(mode) as handle:
            for block in iter(lambda: response.read(1 << 20), b""):
                handle.write(block)
                done += len(block)
                if time.monotonic() - last > 2:
                    print(f"\r    {done / 1e9:.2f} / {expected_bytes / 1e9:.2f} GB", end="", file=out, flush=True)
                    last = time.monotonic()
    print(file=out)
    if part.stat().st_size != expected_bytes:
        raise QuickstartError(f"The download is {part.stat().st_size} bytes, expected {expected_bytes}. Run this again to resume.")
    part.replace(dest)


def ensure_model(path: Path, *, fetch=download) -> None:
    if model_ok(path):
        print(f"==> Model already downloaded and verified: {path}")
        return
    print(f"\n==> Downloading the model (2.1 GB) from Hugging Face\n    {MODEL_URL}\n    to {path}")
    fetch(MODEL_URL, path, expected_bytes=MODEL_BYTES)
    if sha256_file(path) != MODEL_SHA256:
        path.unlink()
        raise QuickstartError("The downloaded file failed its SHA-256 check and was deleted. Run this again.")
    print("    verified (SHA-256 matches)")


def ensure_tools(root: Path, runner=subprocess.run) -> None:
    if tools_built(root):
        print("==> Tools already built (./bundle and wh-mcp exist). Delete ./bundle to rebuild.")
        return
    print("\n==> Building the tools (Wahapedia data, graph bundle, wh-mcp). The first run takes several minutes.")
    if runner([sys.executable, str(root / "scripts" / "setup.py")], cwd=root).returncode != 0:
        raise QuickstartError("Building the tools failed. The message above says which step; fix it and run this again.")


def ensure_host_packages(runner=subprocess.run, find_spec=importlib.util.find_spec) -> None:
    missing = [pip for module, pip in HOST_PACKAGES.items() if find_spec(module) is None]
    if not missing:
        return
    print(f"\n==> Installing the host's Python packages: {', '.join(missing)}")
    if runner([sys.executable, "-m", "pip", "install", *missing]).returncode != 0:
        raise QuickstartError("pip could not install the host packages. Try: python -m pip install -r examples/ollama-host/requirements.txt")


def load_module(name: str, path: Path):
    sys.path.insert(0, str(path.parent))  # the host imports its sibling modules by name
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


def export_system_prompt(dest: Path, root: Path = ROOT) -> str:
    """Ask wh-mcp for its own instructions and save the exact system prompt the model was trained with.

    That prompt is the server's instructions followed by the host's short guidance. Any client that runs
    the model should send it as the system message.
    """
    host = load_module("wh_host", HOST_DIR / "host.py")

    async def fetch() -> str:
        async with host.Client(host.server_params(mcp_binary(root), root / "bundle"), mode="legacy") as client:
            return host.system_prompt(client.instructions)

    text = asyncio.run(fetch())
    dest.parent.mkdir(parents=True, exist_ok=True)
    dest.write_text(text + "\n", encoding="utf-8", newline="\n")
    return text


@dataclass
class Paths:
    """Where everything is, as absolute paths, for the connection sheet."""

    model: Path
    bundle: Path
    mcp: Path
    system_prompt: Path
    llama_server: Path | None
    port: int
    ngl: int


def q(path: Path | str) -> str:
    return f'"{path}"'


def server_args(p: Paths, serve) -> list[str]:
    """The llama-server command as a list (what we run)."""
    return serve.build_command(p.llama_server or Path("llama-server"), p.model, MODEL_ALIAS, ctx=CONTEXT, port=p.port, gpu_layers=p.ngl, extra=[])


def server_command(p: Paths) -> str:
    """The same command as one line to copy and paste."""
    server = q(p.llama_server) if p.llama_server else "llama-server"
    return f"{server} -m {q(p.model)} --jinja -c {CONTEXT} -ngl {p.ngl} --port {p.port} --alias {MODEL_ALIAS}"


def connection_sheet(p: Paths, root: Path = ROOT) -> str:
    """Everything needed to connect your own client, ready to copy and paste."""
    mcp_json = json.dumps({"mcpServers": {"warhammer": {"command": str(p.mcp), "args": ["--bundle", str(p.bundle)]}}}, indent=2)
    mcp_json = "\n".join("    " + line for line in mcp_json.splitlines())
    host = f'python {q(root / "examples" / "ollama-host" / "host.py")} --backend llamacpp --model {MODEL_ALIAS} --base-url http://localhost:{p.port} --bundle {q(p.bundle)}'
    missing = (
        "\n  llama-server was not found. Install a build from https://github.com/ggml-org/llama.cpp/releases (tested with b11476),\n"
        "  put it on PATH or set LLAMA_SERVER, and use its full path in place of `llama-server` below."
        if not p.llama_server else ""
    )
    return f"""
================================================================================
 READY. Connect your own client
================================================================================

WHERE EVERYTHING IS
  Model (GGUF)      {p.model}
  Graph bundle      {p.bundle}
  MCP server        {p.mcp}
  System prompt     {p.system_prompt}
  llama-server      {p.llama_server or "NOT FOUND"}{missing}

1. START THE MODEL (an OpenAI-compatible server; leave it running)

  {server_command(p)}

  Endpoint    http://localhost:{p.port}/v1   (chat completions, with tools)
  Model name  {MODEL_ALIAS}   (any name works; llama-server serves the one model it loaded)
  API key     none needed (if your client insists, any text)
  Keep --jinja: without it llama-server ignores the chat template and tool calling does not work.
  Or let this script start it:  python scripts/quickstart.py --serve

2. ADD THE TOOLS TO YOUR CLIENT (the wh-mcp MCP server: 15 read-only tools, over stdio)
  Your client starts this program itself. Command and arguments:

    {q(p.mcp)} --bundle {q(p.bundle)}

  The same as JSON, for clients that use an `mcpServers` config (many do):

{mcp_json}

  Claude Code (registers the tools there; Claude Code talks to Claude models, not to this model):

    claude mcp add --scope user warhammer -- {q(p.mcp)} --bundle {q(p.bundle)}

3. SETTINGS THE MODEL EXPECTS
  System prompt   Send the contents of this file as the system message:
                  {p.system_prompt}
                  The model was trained with exactly this text (the server's own instructions plus one line
                  of guidance). A different prompt gives worse answers.
  Tools           The 15 tools from step 2, passed as ordinary tool definitions, with the names and schemas
                  the server reports.
  Temperature     0
  Context window  {CONTEXT} tokens (the -c in step 1)
  Loop            Run each tool call the model makes, send the result back as a `tool` message, and repeat
                  until it answers in plain text. Allow about 8 rounds per question.
  Long results    Cut a tool result at 12,000 characters, with a note that it was cut (as the bundled host does).
  Errors          Return a tool error to the model as text. It reads it and retries with a corrected name.

TRY IT WITHOUT YOUR OWN CLIENT (the bundled host does steps 2 and 3 for you; needs step 1 running)

  {host} --ask "{DEFAULT_QUESTION}"

  Leave off --ask for an interactive prompt.

NOTES
  The model is trained for these tools and this prompt and is unreliable without them. It is an unofficial fan
  project, powered by Wahapedia (https://wahapedia.ru); Warhammer 40,000 and its rules belong to Games
  Workshop. Show Wahapedia's credit wherever you show answers to people.
  Print this sheet again any time:  python scripts/quickstart.py --info
"""


def wait_healthy(port: int, process, timeout: float = 240.0) -> None:
    url = f"http://localhost:{port}/health"
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        if process.poll() is not None:
            raise QuickstartError(f"llama-server exited early (code {process.returncode}). Is port {port} already in use? Try --port.")
        try:
            with urllib.request.urlopen(url, timeout=2) as response:
                if response.status == 200:
                    return
        except (urllib.error.URLError, OSError):
            pass
        time.sleep(1)
    raise QuickstartError("llama-server did not become ready in time.")


def try_the_model(paths: Paths, serve, question: str | None, trace: bool) -> None:
    """Start llama-server just long enough to ask through the bundled host, then stop it."""
    command = server_args(paths, serve)
    print(f"\n==> Starting llama-server to try the model\n    {' '.join(command)}")
    log = open(ROOT / "cache" / "quickstart-llama-server.log", "w", encoding="utf-8") if (ROOT / "cache").exists() else subprocess.DEVNULL
    process = subprocess.Popen(command, stdout=log, stderr=subprocess.STDOUT)
    try:
        wait_healthy(paths.port, process)
        host = [sys.executable, str(HOST_DIR / "host.py"), "--backend", "llamacpp", "--model", MODEL_ALIAS,
                "--base-url", f"http://localhost:{paths.port}", "--bundle", str(paths.bundle)]
        if trace:
            host.append("--trace")
        if question is not None:
            host += ["--ask", question]
        print("\n==> Ready. " + ("" if question is not None else "Type a question (empty line or Ctrl+C to quit).") + "\n")
        subprocess.run(host, cwd=ROOT)
    finally:
        process.terminate()
        try:
            process.wait(timeout=15)
        except subprocess.TimeoutExpired:
            process.kill()


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--serve", action="store_true", help="After preparing, start the model server and keep it running (Ctrl+C stops it).")
    parser.add_argument("--ask", nargs="?", const=DEFAULT_QUESTION, default=None, metavar="QUESTION", help="Try the model: ask one question through the bundled host first.")
    parser.add_argument("--chat", action="store_true", help="Try the model: an interactive prompt through the bundled host first.")
    parser.add_argument("--info", action="store_true", help="Only print the connection sheet (nothing is built or downloaded).")
    parser.add_argument("--gguf", type=Path, default=None, help="Use this GGUF file instead of downloading one.")
    parser.add_argument("--models-dir", type=Path, default=ROOT / "models", help="Where the model and system prompt go (default ./models).")
    parser.add_argument("--llama-server", default=None, help="Path to llama-server (else LLAMA_SERVER, PATH, or ~/llama.cpp/*/).")
    parser.add_argument("--port", type=int, default=8080)
    parser.add_argument("--ngl", type=int, default=99, help="Layers to put on the GPU (default 99: all). Use 0 for CPU only.")
    parser.add_argument("--trace", action="store_true", help="With --ask/--chat, show each tool call the model makes.")
    parser.add_argument("--skip-tools", action="store_true", help="Do not build the tools (you already did).")
    parser.add_argument("--dry-run", action="store_true", help="Print what would happen and exit.")
    args = parser.parse_args(argv)
    sys.stdout.reconfigure(line_buffering=True)  # keep our lines in order with the programs we start

    model = (args.gguf or args.models_dir / MODEL_FILE).resolve()
    prompt_file = (args.models_dir / "system-prompt.txt").resolve()
    serve = load_module("serve_llamacpp", HOST_DIR / "serve_llamacpp.py")

    def locate_server(required: bool) -> Path | None:
        try:
            return Path(serve.find_server(args.llama_server)).resolve()
        except serve.ServeError:
            if required:
                raise
            return None

    try:
        if args.dry_run:
            print(f"tools:  {'built' if tools_built() else 'will be built (python scripts/setup.py)'}")
            print(f"model:  {model} ({'present' if model.exists() else 'will be downloaded, 2.1 GB'})")
            print(f"server: {locate_server(False) or 'NOT FOUND (see the README for where to get llama-server)'}")
            return 0
        server = locate_server(required=args.serve or args.ask is not None or args.chat)  # fail before the long steps
        if args.info:
            if not tools_built() or not model.exists():
                raise QuickstartError("Nothing is set up yet. Run: python scripts/quickstart.py")
        else:
            if not args.skip_tools:
                ensure_tools(ROOT)
            if args.gguf is None:
                ensure_model(model)
            elif not model.exists():
                raise QuickstartError(f"No such file: {model}")
            ensure_host_packages()
            print("\n==> Saving the system prompt the model expects")
            export_system_prompt(prompt_file)
        paths = Paths(model, (ROOT / "bundle").resolve(), mcp_binary().resolve(), prompt_file, server or locate_server(False), args.port, args.ngl)
        if args.ask is not None or args.chat:
            try_the_model(paths, serve, args.ask, args.trace)
        print(connection_sheet(paths))
        if args.serve:
            print("==> Model server running. Press Ctrl+C to stop it.\n")
            try:
                return subprocess.run(server_args(paths, serve)).returncode
            except KeyboardInterrupt:
                return 0
        return 0
    except (QuickstartError, serve.ServeError) as error:
        print(f"\nQuickstart stopped: {error}", file=sys.stderr)
        return 1
    except KeyboardInterrupt:
        return 0


if __name__ == "__main__":
    sys.exit(main())
