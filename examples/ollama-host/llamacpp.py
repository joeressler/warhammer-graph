"""A llama.cpp backend for the host: the same agent loop, a different model server.

`host.Host` only needs a chat function: messages and tools in, a reply with
`.message.content` and `.message.tool_calls` out. `OllamaChat` is one. This is the
other, for `llama-server` (llama.cpp's HTTP server, started with `--jinja` so
that tool calling works):

    llama-server -m model.gguf --jinja -c 16384 -ngl 99 --port 8080
    python examples/ollama-host/host.py --backend llamacpp --ask "..."

llama-server speaks the OpenAI chat format, and the host keeps its messages in
Ollama's shape. The differences this module hides:

* an assistant tool call needs `"type": "function"` and its arguments as a JSON
  string, where Ollama takes a dict;
* a tool result needs the `tool_call_id` of the call it answers, where Ollama
  names the tool. The host's messages carry no ids, so they are made up here
  from each message's position, which keeps them the same on every request;
* the reply's arguments arrive as a JSON string and go back out as a dict.

The context window is set when the server starts (`-c`), not per request.
"""

from __future__ import annotations

import json
from types import SimpleNamespace
from typing import Any

import httpx

DEFAULT_BASE_URL = "http://localhost:8080"
# Generous: the host's own per-question timeout is what ends a slow answer.
REQUEST_TIMEOUT = 600.0


def to_openai_messages(messages: list[dict[str, Any]]) -> list[dict[str, Any]]:
    """The host's (Ollama-shaped) messages as OpenAI chat messages.

    Tool calls get an id `call_<message index>_<position>`, and each following
    tool message takes the next unanswered id, in order.
    """
    converted: list[dict[str, Any]] = []
    pending: list[str] = []
    for index, message in enumerate(messages):
        role = message["role"]
        if role == "assistant" and message.get("tool_calls"):
            calls = []
            pending = []
            for position, call in enumerate(message["tool_calls"]):
                call_id = f"call_{index}_{position}"
                pending.append(call_id)
                arguments = call["function"]["arguments"]
                calls.append({
                    "id": call_id,
                    "type": "function",
                    "function": {
                        "name": call["function"]["name"],
                        "arguments": arguments if isinstance(arguments, str) else json.dumps(arguments, ensure_ascii=False),
                    },
                })
            converted.append({"role": "assistant", "content": message.get("content") or "", "tool_calls": calls})
        elif role == "tool":
            entry = {
                "role": "tool",
                "tool_call_id": pending.pop(0) if pending else f"call_{index}_unmatched",
                "content": message.get("content", ""),
            }
            if message.get("tool_name"):
                entry["name"] = message["tool_name"]
            converted.append(entry)
        else:
            converted.append({"role": role, "content": message.get("content") or ""})
    return converted


def parse_reply(body: dict[str, Any]) -> SimpleNamespace:
    """An OpenAI chat completion as the reply the host expects.

    Arguments that are not valid JSON become an empty dict, so the tool's own
    error message ("missing field ...") is what the model sees and can correct.
    """
    message = body["choices"][0]["message"]
    calls = []
    for call in message.get("tool_calls") or []:
        raw = call["function"].get("arguments") or "{}"
        try:
            arguments = raw if isinstance(raw, dict) else json.loads(raw)
        except json.JSONDecodeError:
            arguments = {}
        calls.append(SimpleNamespace(function=SimpleNamespace(name=call["function"]["name"], arguments=arguments if isinstance(arguments, dict) else {})))
    return SimpleNamespace(message=SimpleNamespace(content=message.get("content") or "", tool_calls=calls))


class LlamaServerChat:
    """The real chat function for llama-server: one request per model turn."""

    def __init__(
        self,
        model: str,
        *,
        base_url: str | None = None,
        temperature: float = 0.0,
        transport: httpx.AsyncBaseTransport | None = None,
    ) -> None:
        self.model = model
        self.base_url = (base_url or DEFAULT_BASE_URL).rstrip("/")
        self._temperature = temperature
        self._client = httpx.AsyncClient(base_url=self.base_url, timeout=REQUEST_TIMEOUT, transport=transport)

    async def __call__(self, messages: list[dict[str, Any]], tools: list[dict[str, Any]]) -> Any:
        response = await self._client.post(
            "/v1/chat/completions",
            json={
                "model": self.model,
                "messages": to_openai_messages(messages),
                "tools": tools,
                "temperature": self._temperature,
            },
        )
        if response.status_code != 200:
            # Not an httpx error type: a bad request is one failed question, not a dead server.
            raise RuntimeError(f"llama-server returned {response.status_code}: {response.text[:300]}")
        return parse_reply(response.json())


async def server_info(base_url: str | None = None, *, transport: httpx.AsyncBaseTransport | None = None) -> dict[str, Any]:
    """What a running llama-server says about itself: build, context size, and model file.

    Raises httpx.HTTPError if the server cannot be reached or is not ready.
    """
    async with httpx.AsyncClient(base_url=(base_url or DEFAULT_BASE_URL).rstrip("/"), timeout=10.0, transport=transport) as client:
        health = await client.get("/health")
        health.raise_for_status()
        info: dict[str, Any] = {}
        props = await client.get("/props")
        if props.status_code == 200:
            body = props.json()
            info = {
                "n_ctx": body.get("default_generation_settings", {}).get("n_ctx"),
                "model_path": body.get("model_path"),
                "build_info": body.get("build_info"),
            }
        return info
