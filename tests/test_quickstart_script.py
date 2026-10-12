"""The quickstart script, with downloads and subprocesses faked. No network, no model, no server."""

import hashlib
import importlib.util
import io
import subprocess
import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parent.parent
spec = importlib.util.spec_from_file_location("quickstart_script", ROOT / "scripts" / "quickstart.py")
quickstart = importlib.util.module_from_spec(spec)
sys.modules["quickstart_script"] = quickstart  # dataclasses look the module up by name
spec.loader.exec_module(quickstart)


class FakeResponse(io.BytesIO):
    def __init__(self, data, status=200):
        super().__init__(data)
        self.status = status

    def __enter__(self):
        return self

    def __exit__(self, *exc):
        return False


def test_download_writes_the_file_and_checks_its_size(tmp_path):
    data = b"x" * 5000
    dest = tmp_path / "m.gguf"
    quickstart.download("http://example/m", dest, expected_bytes=len(data), opener=lambda request: FakeResponse(data), out=io.StringIO())
    assert dest.read_bytes() == data and not (tmp_path / "m.gguf.part").exists()


def test_download_resumes_a_partial_file_with_a_range_request(tmp_path):
    data = bytes(range(256)) * 20
    dest = tmp_path / "m.gguf"
    (tmp_path / "m.gguf.part").write_bytes(data[:1000])
    seen = {}

    def opener(request):
        seen["range"] = request.headers.get("Range")
        return FakeResponse(data[1000:], status=206)

    quickstart.download("http://example/m", dest, expected_bytes=len(data), opener=opener, out=io.StringIO())
    assert seen["range"] == "bytes=1000-" and dest.read_bytes() == data


def test_download_starts_over_when_the_server_ignores_the_range(tmp_path):
    data = b"abc" * 1000
    dest = tmp_path / "m.gguf"
    (tmp_path / "m.gguf.part").write_bytes(b"junk")
    quickstart.download("http://example/m", dest, expected_bytes=len(data), opener=lambda request: FakeResponse(data, status=200), out=io.StringIO())
    assert dest.read_bytes() == data


def test_a_short_download_is_an_error_and_keeps_the_partial_file_for_resuming(tmp_path):
    dest = tmp_path / "m.gguf"
    with pytest.raises(quickstart.QuickstartError, match="resume"):
        quickstart.download("http://example/m", dest, expected_bytes=5000, opener=lambda request: FakeResponse(b"x" * 100), out=io.StringIO())
    assert (tmp_path / "m.gguf.part").exists() and not dest.exists()


def test_ensure_model_deletes_a_file_that_fails_its_hash(tmp_path):
    dest = tmp_path / "m.gguf"

    def fetch(url, path, expected_bytes):
        path.write_bytes(b"not the model")

    with pytest.raises(quickstart.QuickstartError, match="SHA-256"):
        quickstart.ensure_model(dest, fetch=fetch)
    assert not dest.exists()


def test_ensure_model_skips_the_download_when_the_file_is_already_right(tmp_path, monkeypatch):
    dest = tmp_path / "m.gguf"
    dest.write_bytes(b"model bytes")
    monkeypatch.setattr(quickstart, "MODEL_BYTES", len(b"model bytes"))
    monkeypatch.setattr(quickstart, "MODEL_SHA256", hashlib.sha256(b"model bytes").hexdigest())
    quickstart.ensure_model(dest, fetch=lambda *a, **k: pytest.fail("should not download"))


def test_the_tools_step_is_skipped_when_built_and_run_otherwise(tmp_path):
    calls = []
    run = lambda command, cwd=None: calls.append(command) or subprocess.CompletedProcess(command, 0)  # noqa: E731
    quickstart.ensure_tools(tmp_path, runner=run)
    assert len(calls) == 1 and calls[0][1].endswith("setup.py")
    (tmp_path / "bundle").mkdir()
    (tmp_path / "bundle" / "graph.db").write_text("x")
    (tmp_path / "target" / "release").mkdir(parents=True)
    (tmp_path / "target" / "release" / f"wh-mcp{quickstart.EXE}").write_text("x")
    quickstart.ensure_tools(tmp_path, runner=run)
    assert len(calls) == 1


def test_a_failed_tools_build_stops_with_a_message(tmp_path):
    with pytest.raises(quickstart.QuickstartError, match="Building the tools failed"):
        quickstart.ensure_tools(tmp_path, runner=lambda command, cwd=None: subprocess.CompletedProcess(command, 1))


def test_host_packages_are_installed_only_when_missing():
    calls = []
    run = lambda command: calls.append(command) or subprocess.CompletedProcess(command, 0)  # noqa: E731
    quickstart.ensure_host_packages(runner=run, find_spec=lambda name: object())
    assert calls == []
    quickstart.ensure_host_packages(runner=run, find_spec=lambda name: None if name == "ollama" else object())
    assert calls[0][-1] == "ollama"


def test_the_pinned_model_matches_the_published_checksum_file_format():
    assert len(quickstart.MODEL_SHA256) == 64 and quickstart.MODEL_URL.endswith(quickstart.MODEL_FILE)


def sample_paths(tmp_path, with_server=True):
    return quickstart.Paths(
        model=tmp_path / "m.gguf", bundle=tmp_path / "bundle", mcp=tmp_path / "target" / "release" / "wh-mcp",
        system_prompt=tmp_path / "system-prompt.txt", llama_server=(tmp_path / "llama-server") if with_server else None,
        port=8123, ngl=0,
    )


def test_the_sheet_gives_every_path_and_a_ready_to_paste_command_for_each_step(tmp_path):
    paths = sample_paths(tmp_path)
    sheet = quickstart.connection_sheet(paths)
    for item in (paths.model, paths.bundle, paths.mcp, paths.system_prompt, paths.llama_server):
        assert str(item) in sheet
    assert "--jinja" in sheet and "--port 8123" in sheet and "-ngl 0" in sheet
    assert f'"{paths.mcp}" --bundle "{paths.bundle}"' in sheet
    assert "claude mcp add" in sheet and "http://localhost:8123/v1" in sheet
    assert "READY" in sheet and "NOT FOUND" not in sheet


def test_the_mcp_json_in_the_sheet_is_valid_json_with_the_right_command(tmp_path):
    import json

    paths = sample_paths(tmp_path)
    sheet = quickstart.connection_sheet(paths)
    marker = "`mcpServers` config (many do):\n\n"
    block = sheet[sheet.index(marker) + len(marker):sheet.index("\n\n  Claude Code")]
    block = "\n".join(line[4:] for line in block.splitlines())
    config = json.loads(block)["mcpServers"]["warhammer"]
    assert config["command"] == str(paths.mcp) and config["args"] == ["--bundle", str(paths.bundle)]


def test_a_missing_llama_server_is_called_out_with_where_to_get_it(tmp_path):
    sheet = quickstart.connection_sheet(sample_paths(tmp_path, with_server=False))
    assert "NOT FOUND" in sheet and "github.com/ggml-org/llama.cpp/releases" in sheet


def test_the_server_command_pins_the_alias_and_context(tmp_path):
    command = quickstart.server_command(sample_paths(tmp_path))
    assert f"--alias {quickstart.MODEL_ALIAS}" in command and f"-c {quickstart.CONTEXT}" in command


def test_info_refuses_to_run_before_anything_is_set_up(tmp_path, capsys):
    code = quickstart.main(["--info", "--models-dir", str(tmp_path)])
    assert code == 1 and "Nothing is set up yet" in capsys.readouterr().err
