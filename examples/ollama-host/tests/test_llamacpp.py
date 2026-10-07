"""The llama.cpp backend, offline.

A fake llama-server (an httpx mock transport) stands in for the model server. The
conversation still runs through the real `host.Host` and the real wh-mcp, so these
check that the OpenAI-shaped requests are right and that a tool call round-trips.
"""

from __future__ import annotations

import asyncio
import json
from pathlib import Path
from typing import Any, Callable

import httpx
import pytest
from mcp import Client

import host
import llamacpp
import serve_llamacpp
from llamacpp import LlamaServerChat, parse_reply, server_info, to_openai_messages


def completion(content: str = "", *calls: tuple[str, str]) -> dict[str, Any]:
    """A llama-server chat completion with optional tool calls given as (name, arguments-json)."""
    return {"choices": [{"message": {
        "role": "assistant", "content": content,
        "tool_calls": [{"type": "function", "id": f"srv{i}", "function": {"name": n, "arguments": a}} for i, (n, a) in enumerate(calls)],
    }}]}


def fake_server(*replies: Any) -> tuple[httpx.MockTransport, list[dict[str, Any]]]:
    """A transport that answers each request with the next reply and records the request bodies."""
    seen: list[dict[str, Any]] = []
    queue = list(replies)

    def handle(request: httpx.Request) -> httpx.Response:
        seen.append(json.loads(request.content))
        reply = queue.pop(0)
        if isinstance(reply, httpx.Response):
            return reply
        return httpx.Response(200, json=reply)

    return httpx.MockTransport(handle), seen


# ---------------------------------------------------------------- conversion

def test_a_tool_round_trip_becomes_openai_messages_with_matching_ids() -> None:
    messages = [
        {"role": "system", "content": "be brief"},
        {"role": "user", "content": "hi"},
        {"role": "assistant", "content": "", "tool_calls": [
            {"function": {"name": "get_unit", "arguments": {"unit": "Angron"}}},
            {"function": {"name": "get_roster", "arguments": {"subject": "Orks"}}},
        ]},
        {"role": "tool", "tool_name": "get_unit", "content": "{\"T\":\"11\"}"},
        {"role": "tool", "tool_name": "get_roster", "content": "{\"count\":87}"},
        {"role": "assistant", "content": "done"},
    ]
    out = to_openai_messages(messages)
    assistant = out[2]
    assert [c["type"] for c in assistant["tool_calls"]] == ["function", "function"]
    assert assistant["tool_calls"][0]["function"]["arguments"] == '{"unit": "Angron"}', "arguments are a JSON string"
    ids = [c["id"] for c in assistant["tool_calls"]]
    assert len(set(ids)) == 2
    assert [out[3]["tool_call_id"], out[4]["tool_call_id"]] == ids, "each result answers the call before it, in order"
    assert out[3]["name"] == "get_unit"
    assert out[5] == {"role": "assistant", "content": "done"}


def test_ids_are_the_same_on_every_request() -> None:
    """Stable ids keep the prompt identical across turns, so the server can reuse its cache."""
    messages = [
        {"role": "user", "content": "q"},
        {"role": "assistant", "content": "", "tool_calls": [{"function": {"name": "t", "arguments": {}}}]},
        {"role": "tool", "tool_name": "t", "content": "{}"},
    ]
    assert to_openai_messages(messages) == to_openai_messages(list(messages))


def test_a_reply_becomes_what_the_host_expects() -> None:
    reply = parse_reply(completion("", ("get_unit", '{"unit": "Angron", "sections": ["models"]}')))
    call = reply.message.tool_calls[0]
    assert call.function.name == "get_unit"
    assert call.function.arguments == {"unit": "Angron", "sections": ["models"]}
    assert reply.message.content == ""


@pytest.mark.parametrize("arguments", ["not json", "[1, 2]", "", "null"])
def test_unusable_arguments_become_an_empty_dict_so_the_tool_can_explain(arguments: str) -> None:
    reply = parse_reply(completion("", ("get_unit", arguments)))
    assert reply.message.tool_calls[0].function.arguments == {}


def test_a_null_content_reads_as_empty_text() -> None:
    body = {"choices": [{"message": {"role": "assistant", "content": None}}]}
    reply = parse_reply(body)
    assert reply.message.content == "" and reply.message.tool_calls == []


# ---------------------------------------------------------------- the chat function

def run(coro):
    return asyncio.run(coro)


def test_the_request_has_the_model_tools_and_temperature() -> None:
    transport, seen = fake_server(completion("hello"))
    chat = LlamaServerChat("granite4.1:8b", temperature=0.17, transport=transport)
    tools = [{"type": "function", "function": {"name": "t", "description": "d", "parameters": {"type": "object"}}}]
    reply = run(chat([{"role": "user", "content": "q"}], tools))
    assert reply.message.content == "hello"
    body = seen[0]
    assert body["model"] == "granite4.1:8b" and body["temperature"] == 0.17 and body["tools"] == tools
    assert body["messages"] == [{"role": "user", "content": "q"}]


def test_a_bad_request_is_a_failed_question_not_a_dead_server() -> None:
    transport, _ = fake_server(httpx.Response(500, json={"error": {"message": "Failed to parse messages"}}))
    with pytest.raises(RuntimeError, match="500.*Failed to parse messages"):
        run(LlamaServerChat("m", transport=transport)([{"role": "user", "content": "q"}], []))


def test_an_unreachable_server_raises_a_transport_error_the_eval_runner_treats_as_hard() -> None:
    def refuse(request: httpx.Request) -> httpx.Response:
        raise httpx.ConnectError("refused")

    with pytest.raises(httpx.TransportError):  # evals.run.is_hard ends a run on exactly this
        run(LlamaServerChat("m", transport=httpx.MockTransport(refuse))([{"role": "user", "content": "q"}], []))


def test_server_info_reads_health_and_props() -> None:
    def handle(request: httpx.Request) -> httpx.Response:
        if request.url.path == "/health":
            return httpx.Response(200, json={"status": "ok"})
        return httpx.Response(200, json={"default_generation_settings": {"n_ctx": 16384}, "model_path": "m.gguf", "build_info": "b1"})

    assert run(server_info(transport=httpx.MockTransport(handle))) == {"n_ctx": 16384, "model_path": "m.gguf", "build_info": "b1"}

    def loading(request: httpx.Request) -> httpx.Response:
        return httpx.Response(503, json={"error": "Loading model"})

    with pytest.raises(httpx.HTTPStatusError):
        run(server_info(transport=httpx.MockTransport(loading)))


# ---------------------------------------------------------------- through the real host and server

def test_a_tool_call_round_trips_through_the_real_host_and_wh_mcp(exe, bundle) -> None:
    transport, seen = fake_server(
        completion("", ("bundle_info", "{}")),
        completion("The data has 11 nodes."),
    )
    chat = LlamaServerChat("fake", transport=transport)

    async def go():
        async with Client(host.server_params(exe, bundle), mode="legacy") as client:
            tools = host.to_ollama_tools((await client.list_tools()).tools)
            conversation = host.Host(client, chat, tools, host.system_prompt(client.instructions))
            return await conversation.ask("How many nodes?")

    answer = run(go())
    assert answer.text == "The data has 11 nodes." and [s.tool for s in answer.steps] == ["bundle_info"]
    second = seen[1]["messages"]
    assistant = next(m for m in second if m["role"] == "assistant")
    tool = next(m for m in second if m["role"] == "tool")
    assert assistant["tool_calls"][0]["type"] == "function"
    assert tool["tool_call_id"] == assistant["tool_calls"][0]["id"], "the result is tied to the call that asked for it"
    assert '"node_count":11' in tool["content"], "the real server's reply is what the model sees"


def test_make_chat_chooses_the_backend() -> None:
    assert isinstance(host.make_chat("ollama", "m"), host.OllamaChat)
    chat = host.make_chat("llamacpp", "m", url="http://example.invalid:9")
    assert isinstance(chat, LlamaServerChat) and chat.base_url == "http://example.invalid:9"
    with pytest.raises(ValueError, match="unknown backend"):
        host.make_chat("nope", "m")


# ---------------------------------------------------------------- the launcher

def _store(tmp_path: Path, name: str = "granite4.1", tag: str = "8b", digest: str = "sha256:abc", with_blob: bool = True) -> Path:
    manifest = tmp_path / "manifests" / "registry.ollama.ai" / "library" / name / tag
    manifest.parent.mkdir(parents=True)
    manifest.write_text(json.dumps({"layers": [
        {"mediaType": "application/vnd.ollama.image.template", "digest": "sha256:t"},
        {"mediaType": "application/vnd.ollama.image.model", "digest": digest},
    ]}), encoding="utf-8")
    if with_blob:
        (tmp_path / "blobs").mkdir()
        (tmp_path / "blobs" / digest.replace(":", "-")).write_bytes(b"GGUF")
    return tmp_path


def test_an_ollama_name_resolves_to_its_gguf_file(tmp_path: Path) -> None:
    store = _store(tmp_path)
    assert serve_llamacpp.resolve_blob("granite4.1:8b", store) == store / "blobs" / "sha256-abc"
    assert serve_llamacpp.resolve_blob("library/granite4.1:8b", store) == store / "blobs" / "sha256-abc"


def test_a_missing_model_or_file_says_how_to_fix_it(tmp_path: Path) -> None:
    with pytest.raises(serve_llamacpp.ServeError, match="ollama pull nope:1b"):
        serve_llamacpp.resolve_blob("nope:1b", _store(tmp_path))
    with pytest.raises(serve_llamacpp.ServeError, match="missing"):
        serve_llamacpp.resolve_blob("granite4.1:8b", _store(tmp_path / "other", with_blob=False))


def test_the_server_command_always_enables_jinja_for_tool_calling() -> None:
    command = serve_llamacpp.build_command(Path("llama-server"), Path("m.gguf"), "alias", ctx=8192, port=9, gpu_layers=99, extra=["--no-webui"])
    assert "--jinja" in command and command[command.index("-c") + 1] == "8192" and command[-1] == "--no-webui"
    assert command[command.index("--alias") + 1] == "alias"


def test_finding_llama_server_prefers_an_explicit_path(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    binary = tmp_path / "llama-server"
    binary.write_bytes(b"")
    assert serve_llamacpp.find_server(str(binary)) == binary
    with pytest.raises(serve_llamacpp.ServeError, match="not found at"):
        serve_llamacpp.find_server(str(tmp_path / "missing"))
    monkeypatch.setenv("LLAMA_SERVER", str(binary))
    assert serve_llamacpp.find_server() == binary


def test_dry_run_prints_the_command_without_starting_anything(tmp_path: Path, capsys: pytest.CaptureFixture[str], monkeypatch: pytest.MonkeyPatch) -> None:
    store = _store(tmp_path)
    binary = tmp_path / "llama-server"
    binary.write_bytes(b"")
    monkeypatch.setenv("OLLAMA_MODELS", str(store))
    assert serve_llamacpp.main(["--ollama-model", "granite4.1:8b", "--llama-server", str(binary), "--dry-run"]) == 0
    assert "--jinja" in capsys.readouterr().err
    assert serve_llamacpp.main(["--ollama-model", "nope:1b", "--llama-server", str(binary), "--dry-run"]) == 2
