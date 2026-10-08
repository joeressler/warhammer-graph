"""Connect a local Ollama model to the wh-mcp server.

The host starts wh-mcp, shows its tools to an Ollama model, runs the tool calls
the model asks for through MCP, and feeds the results back until the model
answers in plain text:

    you  ->  host  ->  Ollama model  <->  wh-mcp (started by the host)  ->  bundle

Run it:

    python examples/ollama-host/host.py --ask "What invulnerable save does Angron have?"
    python examples/ollama-host/host.py            # interactive

The model must support tool calling (`ollama show <model>` lists `tools`).

To use llama.cpp instead of Ollama, start `llama-server` with `--jinja` (see
`serve_llamacpp.py`) and add `--backend llamacpp`.
"""

from __future__ import annotations

import argparse
import asyncio
import json
import os
import re
import sys
import time
from collections.abc import Awaitable, Callable, Sequence
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

import ollama
from mcp import Client, StdioServerParameters

ROOT = Path(__file__).resolve().parents[2]

DEFAULT_MODEL = "granite4.1:8b"
# Ollama's own default context is small, and a unit card is about 9 KB of JSON.
DEFAULT_NUM_CTX = 16384
DEFAULT_MAX_STEPS = 8
# A tool result longer than this is cut, with a note, so one big roster cannot
# fill a small model's context. 0 turns the limit off.
DEFAULT_MAX_TOOL_CHARS = 12000

HOST_GUIDANCE = (
    "Answer only from what the tools return, not from memory. "
    "Call a tool for every fact you state. "
    "If a tool returns an error, read it and call again with a corrected name or an id it offers. "
    "Give the final answer in plain language."
)

ChatFn = Callable[[list[dict[str, Any]], list[dict[str, Any]]], Awaitable[Any]]
"""Sends messages and tools to a model and returns a reply with `.message`."""


def find_executable() -> Path:
    """The built wh-mcp binary: release first, then debug."""
    name = "wh-mcp.exe" if os.name == "nt" else "wh-mcp"
    for profile in ("release", "debug"):
        candidate = ROOT / "target" / profile / name
        if candidate.exists():
            return candidate
    return ROOT / "target" / "release" / name


def server_params(exe: Path | str, bundle: Path | str) -> StdioServerParameters:
    """How to launch wh-mcp against a bundle."""
    return StdioServerParameters(command=str(exe), args=["--bundle", str(bundle)])


def to_ollama_tools(tools: Sequence[Any], allow: Sequence[str] | None = None) -> list[dict[str, Any]]:
    """Convert MCP tools to Ollama's tool format, optionally keeping only `allow`.

    A name in `allow` that the server does not have is an error, so a typo is
    not read as "no tools".
    """
    known = {tool.name for tool in tools}
    if allow:
        missing = [name for name in allow if name not in known]
        if missing:
            raise ValueError(f"unknown tool(s): {', '.join(missing)}; the server has: {', '.join(sorted(known))}")
    return [
        {
            "type": "function",
            "function": {
                "name": tool.name,
                "description": tool.description or "",
                "parameters": tool.input_schema,
            },
        }
        for tool in tools
        if not allow or tool.name in allow
    ]


def system_prompt(instructions: str | None) -> str:
    """The server's own usage guidance, then this host's."""
    return "\n\n".join(part for part in (instructions, HOST_GUIDANCE) if part)


def clip(text: str, limit: int) -> str:
    """Cut `text` to `limit` characters and say how much was left out."""
    if limit <= 0 or len(text) <= limit:
        return text
    cut = len(text) - limit
    return (
        text[:limit]
        + f"\n[truncated {cut} characters; ask for less, for example with sections or a smaller limit]"
    )


def result_text(result: Any) -> str:
    """The text blocks of a tool result, joined."""
    return "\n".join(block.text for block in result.content if getattr(block, "type", "") == "text")


_THINKING = re.compile(r"<think>.*?(?:</think>|\Z)", re.DOTALL)


def strip_thinking(text: str) -> str:
    """Drop `<think>...</think>` reasoning a model wrote into its answer.

    A block that never closes is dropped to the end. Some models write their
    reasoning into the answer text instead of Ollama's separate `thinking` field.
    """
    return _THINKING.sub("", text).strip()


@dataclass
class Step:
    """One tool call the model made."""

    tool: str
    arguments: dict[str, Any]
    result_chars: int
    is_error: bool
    seconds: float


@dataclass
class Answer:
    """The outcome of one question."""

    text: str
    steps: list[Step] = field(default_factory=list)
    # "answered", or "step_limit" when the model kept calling tools.
    stopped: str = "answered"

    def called(self, tool: str) -> bool:
        return any(step.tool == tool for step in self.steps)


class OllamaChat:
    """The real chat function: one call to a local Ollama model."""

    def __init__(
        self,
        model: str,
        *,
        host: str | None = None,
        num_ctx: int = DEFAULT_NUM_CTX,
        temperature: float = 0.0,
    ) -> None:
        self.model = model
        self._client = ollama.AsyncClient(host=host)
        self._options = {"num_ctx": num_ctx, "temperature": temperature}

    async def __call__(self, messages: list[dict[str, Any]], tools: list[dict[str, Any]]) -> Any:
        return await self._client.chat(
            model=self.model, messages=messages, tools=tools, options=self._options
        )


BACKENDS = ("ollama", "llamacpp")


def make_chat(backend: str, model: str, *, url: str | None = None, num_ctx: int = DEFAULT_NUM_CTX, temperature: float = 0.0) -> ChatFn:
    """The chat function for a backend.

    `url` is the Ollama host or the llama-server base URL, each defaulting to the
    local one. `num_ctx` sets Ollama's context window; llama-server fixes its own
    when it starts (`-c`), so it is ignored there.
    """
    if backend == "ollama":
        return OllamaChat(model, host=url, num_ctx=num_ctx, temperature=temperature)
    if backend == "llamacpp":
        from llamacpp import LlamaServerChat

        return LlamaServerChat(model, base_url=url, temperature=temperature)
    raise ValueError(f"unknown backend {backend!r}; choose one of: {', '.join(BACKENDS)}")


class Host:
    """A conversation between a model and the wh-mcp tools."""

    def __init__(
        self,
        client: Client,
        chat: ChatFn,
        tools: list[dict[str, Any]],
        system: str,
        *,
        max_steps: int = DEFAULT_MAX_STEPS,
        max_tool_chars: int = DEFAULT_MAX_TOOL_CHARS,
        on_step: Callable[[Step], None] | None = None,
    ) -> None:
        self._client = client
        self._chat = chat
        self._tools = tools
        self._allowed = {tool["function"]["name"] for tool in tools}
        self._max_steps = max_steps
        self._max_tool_chars = max_tool_chars
        self._on_step = on_step
        self.messages: list[dict[str, Any]] = [{"role": "system", "content": system}]

    async def ask(self, question: str) -> Answer:
        """Ask a question. Earlier questions stay in the conversation."""
        self.messages.append({"role": "user", "content": question})
        steps: list[Step] = []
        last_text = ""
        for _ in range(self._max_steps):
            reply = await self._chat(self.messages, self._tools)
            message = reply.message
            last_text = message.content or ""
            calls = list(message.tool_calls or [])
            self.messages.append(_assistant(last_text, calls))
            if not calls:
                return Answer(text=strip_thinking(last_text), steps=steps)
            for call in calls:
                step, text = await self._run(call.function.name, dict(call.function.arguments or {}))
                steps.append(step)
                if self._on_step:
                    self._on_step(step)
                self.messages.append({"role": "tool", "tool_name": step.tool, "content": text})
        return Answer(text=strip_thinking(last_text), steps=steps, stopped="step_limit")

    async def _run(self, name: str, arguments: dict[str, Any]) -> tuple[Step, str]:
        """Run one tool call through MCP. A failure becomes text the model can read."""
        started = time.monotonic()
        if name not in self._allowed:
            text = json.dumps({"error": f"no tool named {name}; the tools are: {', '.join(sorted(self._allowed))}"})
            is_error = True
        else:
            try:
                result = await self._client.call_tool(name, arguments)
                text, is_error = result_text(result), bool(result.is_error)
            except Exception as error:  # a protocol failure must not end the conversation
                text, is_error = json.dumps({"error": f"{type(error).__name__}: {error}"}), True
        text = clip(text, self._max_tool_chars)
        step = Step(name, arguments, len(text), is_error, time.monotonic() - started)
        return step, text


def _assistant(content: str, calls: list[Any]) -> dict[str, Any]:
    """The assistant's turn as a plain message, ready to send back to the model."""
    message: dict[str, Any] = {"role": "assistant", "content": content}
    if calls:
        message["tool_calls"] = [
            {"function": {"name": call.function.name, "arguments": dict(call.function.arguments or {})}}
            for call in calls
        ]
    return message


def trace_step(step: Step) -> None:
    """Print one tool call to stderr."""
    flag = "ERROR" if step.is_error else "ok"
    name = " ".join(step.tool.split())
    name = name if len(name) <= 60 else name[:57] + "..."
    arguments = json.dumps(step.arguments, ensure_ascii=False)
    print(f"  [{flag}] {name}({arguments[:200]}) -> {step.result_chars} chars in {step.seconds * 1000:.0f} ms", file=sys.stderr)


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Ask a local Ollama model questions through the wh-mcp server.")
    parser.add_argument("--ask", help="One question to answer, then exit. Omit for an interactive prompt.")
    parser.add_argument("--backend", choices=BACKENDS, default="ollama", help="Model server: Ollama, or llama.cpp's llama-server (started with --jinja).")
    parser.add_argument("--model", default=DEFAULT_MODEL, help=f"Model name (default {DEFAULT_MODEL}). For llama-server it is only a label.")
    parser.add_argument("--bundle", type=Path, default=ROOT / "bundle", help="Graph bundle directory.")
    parser.add_argument("--exe", type=Path, default=None, help="Path to wh-mcp (default: target/release or target/debug).")
    parser.add_argument("--ollama-host", default=None, help="Ollama server URL (default: the local one).")
    parser.add_argument("--base-url", default=None, help="llama-server URL for --backend llamacpp (default http://localhost:8080).")
    parser.add_argument("--num-ctx", type=int, default=DEFAULT_NUM_CTX, help="Model context window in tokens.")
    parser.add_argument("--temperature", type=float, default=0.0)
    parser.add_argument("--max-steps", type=int, default=DEFAULT_MAX_STEPS, help="Most tool-calling rounds per question.")
    parser.add_argument("--max-tool-chars", type=int, default=DEFAULT_MAX_TOOL_CHARS, help="Cut tool results to this length (0 = no cut).")
    parser.add_argument("--tools", default=None, help="Comma-separated tool names to offer. Default: all.")
    parser.add_argument("--trace", action="store_true", help="Print each tool call to stderr.")
    return parser.parse_args(argv)


async def run(args: argparse.Namespace) -> int:
    exe = args.exe or find_executable()
    if not exe.exists():
        print(f"host: {exe} not found. Build it with: cargo build -p wh-mcp --release", file=sys.stderr)
        return 2
    allow = [name.strip() for name in args.tools.split(",") if name.strip()] if args.tools else None
    async with Client(server_params(exe, args.bundle), mode="legacy") as client:
        listing = await client.list_tools()
        try:
            tools = to_ollama_tools(listing.tools, allow)
        except ValueError as error:
            print(f"host: {error}", file=sys.stderr)
            return 2
        url = args.base_url if args.backend == "llamacpp" else args.ollama_host
        chat = make_chat(args.backend, args.model, url=url, num_ctx=args.num_ctx, temperature=args.temperature)
        host = Host(
            client,
            chat,
            tools,
            system_prompt(client.instructions),
            max_steps=args.max_steps,
            max_tool_chars=args.max_tool_chars,
            on_step=trace_step if args.trace else None,
        )
        print(f"host: {args.model} ({args.backend}) with {len(tools)} tools", file=sys.stderr)
        if args.ask:
            print((await host.ask(args.ask)).text)
            return 0
        while True:
            try:
                question = (await asyncio.to_thread(input, "> ")).strip()
            except EOFError:
                return 0
            if not question:
                return 0
            answer = await host.ask(question)
            print(answer.text + ("\n(stopped: step limit)" if answer.stopped == "step_limit" else ""))


def use_utf8(*streams: Any) -> None:
    """Make output streams UTF-8, replacing anything unencodable.

    Models write typographic characters such as a non-breaking hyphen. On
    Windows a piped stdout is cp1252 and would raise on them, which would end the
    conversation after the model had already answered.
    """
    for stream in streams:
        if hasattr(stream, "reconfigure"):
            stream.reconfigure(encoding="utf-8", errors="replace")


def main() -> int:
    use_utf8(sys.stdout, sys.stderr)
    return asyncio.run(run(parse_args()))


if __name__ == "__main__":
    raise SystemExit(main())
