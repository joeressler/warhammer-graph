"""The host loop, driven by a scripted fake model against the real wh-mcp server.

No Ollama is needed: the fake model returns the replies a real one might, so the
tests check what the host does with them.
"""

from __future__ import annotations

import asyncio
import json
from types import SimpleNamespace
from typing import Any, Callable

import pytest
from mcp import Client

import host


# ---------------------------------------------------------------- helpers


def reply(content: str = "", *calls: tuple[str, dict[str, Any]]) -> SimpleNamespace:
    """A model reply: some text, and any tool calls."""
    return SimpleNamespace(
        message=SimpleNamespace(
            content=content,
            tool_calls=[SimpleNamespace(function=SimpleNamespace(name=name, arguments=arguments)) for name, arguments in calls],
        )
    )


class Script:
    """A fake model. Each turn is a reply, or a function of the messages so far that returns one."""

    def __init__(self, *turns: Any) -> None:
        self.turns = list(turns)
        self.seen: list[list[dict[str, Any]]] = []
        self.tools_seen: list[list[dict[str, Any]]] = []

    async def __call__(self, messages: list[dict[str, Any]], tools: list[dict[str, Any]]) -> Any:
        self.seen.append([dict(message) for message in messages])
        self.tools_seen.append(tools)
        turn = self.turns.pop(0)
        return turn(messages) if callable(turn) else turn


def run_host(exe, bundle, script: Script, questions: list[str], *, allow=None, **options: Any):
    """Start the real server, ask each question, and return the answers and the host."""

    async def go():
        async with Client(host.server_params(exe, bundle), mode="legacy") as client:
            tools = host.to_ollama_tools((await client.list_tools()).tools, allow)
            conversation = host.Host(client, script, tools, host.system_prompt(client.instructions), **options)
            return [await conversation.ask(question) for question in questions], conversation

    return asyncio.run(go())


def last_tool_message(messages: list[dict[str, Any]]) -> str:
    assert messages[-1]["role"] == "tool", messages[-1]
    return messages[-1]["content"]


# ---------------------------------------------------------------- pure functions


def _tool(name: str, description: str = "does a thing") -> SimpleNamespace:
    return SimpleNamespace(name=name, description=description, input_schema={"type": "object", "properties": {"x": {"type": "string"}}})


def test_tools_convert_to_the_ollama_format_unchanged() -> None:
    converted = host.to_ollama_tools([_tool("get_unit", "Get a unit."), _tool("search")])
    assert converted[0] == {
        "type": "function",
        "function": {
            "name": "get_unit",
            "description": "Get a unit.",
            "parameters": {"type": "object", "properties": {"x": {"type": "string"}}},
        },
    }
    assert [tool["function"]["name"] for tool in converted] == ["get_unit", "search"]


def test_an_allowlist_keeps_only_those_tools_and_rejects_a_typo() -> None:
    tools = [_tool("get_unit"), _tool("search"), _tool("get_roster")]
    kept = host.to_ollama_tools(tools, ["search", "get_unit"])
    assert [tool["function"]["name"] for tool in kept] == ["get_unit", "search"]
    with pytest.raises(ValueError, match="unknown tool.*get_unitt"):
        host.to_ollama_tools(tools, ["get_unitt"])


def test_clip_leaves_short_text_and_says_how_much_it_cut() -> None:
    assert host.clip("short", 100) == "short"
    assert host.clip("x" * 500, 0) == "x" * 500, "0 means no limit"
    clipped = host.clip("x" * 500, 100)
    assert clipped.startswith("x" * 100)
    assert "[truncated 400 characters" in clipped


def test_the_system_prompt_has_the_servers_guidance_then_the_hosts() -> None:
    prompt = host.system_prompt("Start with get_unit.")
    assert prompt.startswith("Start with get_unit.")
    assert host.HOST_GUIDANCE in prompt
    assert host.system_prompt(None) == host.HOST_GUIDANCE


def test_the_command_line_defaults_and_flags() -> None:
    args = host.parse_args([])
    assert (args.model, args.num_ctx, args.max_steps, args.trace) == (host.DEFAULT_MODEL, host.DEFAULT_NUM_CTX, host.DEFAULT_MAX_STEPS, False)
    args = host.parse_args(["--model", "qwen3:0.6b", "--tools", "get_unit,search", "--trace", "--ask", "hi"])
    assert (args.model, args.tools, args.trace, args.ask) == ("qwen3:0.6b", "get_unit,search", True, "hi")


# ---------------------------------------------------------------- the loop, against the real server


def test_a_tool_call_runs_through_the_real_server_and_the_result_reaches_the_model(exe, bundle) -> None:
    def final(messages: list[dict[str, Any]]) -> Any:
        data = json.loads(last_tool_message(messages))
        assert data["models"][0]["invulnerable_save"] == "4+", data
        return reply("Example Unit has a 4+ invulnerable save.")

    script = Script(reply("", ("get_unit", {"unit": "Example Unit", "sections": ["models"]})), final)
    (answer,), _ = run_host(exe, bundle, script, ["What is Example Unit's invulnerable save?"])
    assert answer.text == "Example Unit has a 4+ invulnerable save."
    assert answer.stopped == "answered"
    assert answer.called("get_unit")
    assert [(step.tool, step.is_error) for step in answer.steps] == [("get_unit", False)]
    assert answer.steps[0].arguments == {"unit": "Example Unit", "sections": ["models"]}


def test_the_model_sees_every_tool_and_the_servers_instructions(exe, bundle) -> None:
    script = Script(reply("done"))
    run_host(exe, bundle, script, ["hello"])
    assert len(script.tools_seen[0]) == 15
    system = script.seen[0][0]
    assert system["role"] == "system"
    assert "get_unit" in system["content"], "the server's own instructions are included"
    assert host.HOST_GUIDANCE in system["content"]


def test_an_error_goes_back_to_the_model_so_it_can_retry(exe, bundle) -> None:
    def retry(messages: list[dict[str, Any]]) -> Any:
        error = json.loads(last_tool_message(messages))
        assert "Example Unit" in error["suggestions"], error
        return reply("", ("get_unit", {"unit": "Example Unit", "sections": ["points"]}))

    script = Script(reply("", ("get_unit", {"unit": "Exmple Unit"})), retry, reply("Recovered."))
    (answer,), _ = run_host(exe, bundle, script, ["Tell me about Exmple Unit"])
    assert [(step.tool, step.is_error) for step in answer.steps] == [("get_unit", True), ("get_unit", False)]
    assert answer.text == "Recovered."


def test_a_tool_the_server_does_not_have_is_reported_and_never_called(exe, bundle) -> None:
    def after(messages: list[dict[str, Any]]) -> Any:
        assert "no tool named make_coffee" in last_tool_message(messages)
        return reply("Sorry.")

    script = Script(reply("", ("make_coffee", {})), after)
    (answer,), _ = run_host(exe, bundle, script, ["coffee?"])
    assert [(step.tool, step.is_error) for step in answer.steps] == [("make_coffee", True)]


def test_a_tool_outside_the_allowlist_is_refused(exe, bundle) -> None:
    def after(messages: list[dict[str, Any]]) -> Any:
        text = last_tool_message(messages)
        assert "no tool named get_unit" in text and "find_units" in text
        return reply("Understood.")

    script = Script(reply("", ("get_unit", {"unit": "Example Unit"})), after)
    (answer,), _ = run_host(exe, bundle, script, ["q"], allow=["find_units"])
    assert answer.steps[0].is_error
    assert len(script.tools_seen[0]) == 1


def test_a_model_that_never_stops_calling_tools_hits_the_step_limit(exe, bundle) -> None:
    script = Script(*[reply("thinking", ("bundle_info", {})) for _ in range(3)])
    (answer,), _ = run_host(exe, bundle, script, ["loop"], max_steps=3)
    assert answer.stopped == "step_limit"
    assert len(answer.steps) == 3
    assert answer.text == "thinking", "the last thing the model said is kept"


def test_a_long_tool_result_is_cut_before_it_reaches_the_model(exe, bundle) -> None:
    def after(messages: list[dict[str, Any]]) -> Any:
        text = last_tool_message(messages)
        assert "[truncated" in text
        return reply("ok")

    script = Script(reply("", ("bundle_info", {})), after)
    (answer,), _ = run_host(exe, bundle, script, ["info"], max_tool_chars=60)
    assert answer.steps[0].result_chars < 200


def test_a_follow_up_keeps_the_earlier_conversation(exe, bundle) -> None:
    script = Script(reply("The first answer."), reply("The second answer."))
    answers, conversation = run_host(exe, bundle, script, ["first question", "second question"])
    assert [answer.text for answer in answers] == ["The first answer.", "The second answer."]
    second_turn = script.seen[1]
    contents = [message["content"] for message in second_turn]
    assert "first question" in contents and "The first answer." in contents
    assert contents[-1] == "second question"
    assert len(conversation.messages) == 5, "system, 2 questions, 2 answers"


def test_the_step_callback_sees_each_call_as_it_happens(exe, bundle) -> None:
    seen: list[str] = []
    script = Script(reply("", ("list_factions", {}), ("bundle_info", {})), reply("done"))
    run_host(exe, bundle, script, ["two tools at once"], on_step=lambda step: seen.append(step.tool))
    assert seen == ["list_factions", "bundle_info"]


def test_output_streams_survive_characters_cp1252_cannot_encode() -> None:
    import io

    raw = io.BytesIO()
    stream = io.TextIOWrapper(raw, encoding="cp1252", errors="strict")
    with pytest.raises(UnicodeEncodeError):
        stream.write("twenty\u2011one")
    stream = io.TextIOWrapper(io.BytesIO(), encoding="cp1252", errors="strict")
    host.use_utf8(stream)
    stream.write("twenty\u2011one \u2019")
    stream.flush()
    assert stream.buffer.getvalue().decode("utf-8") == "twenty\u2011one \u2019"


def test_thinking_text_is_removed_from_an_answer() -> None:
    assert host.strip_thinking("<think>reasoning</think>\nThe answer is 4+.") == "The answer is 4+."
    assert host.strip_thinking("Before <think>a</think> and <think>b</think>after") == "Before  and after"
    assert host.strip_thinking("<think>never closed, so all of it goes") == ""
    assert host.strip_thinking("  plain answer  ") == "plain answer"


def test_a_models_thinking_does_not_reach_the_final_answer(exe, bundle) -> None:
    script = Script(reply("<think>I should answer.</think>Done."))
    (answer,), _ = run_host(exe, bundle, script, ["q"])
    assert answer.text == "Done."


def test_a_garbage_tool_name_is_traced_short(capsys) -> None:
    long_name = "```\n\nBut we have not seen its result. " * 5
    host.trace_step(host.Step(long_name, {}, 10, True, 0.001))
    line = capsys.readouterr().err.splitlines()[0]
    assert "..." in line and len(line) < 200
