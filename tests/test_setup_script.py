"""The one-step setup script, with every command faked. No network, no cargo."""

import importlib.util
import subprocess
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parent.parent
spec = importlib.util.spec_from_file_location("setup_script", ROOT / "scripts" / "setup.py")
setup_script = importlib.util.module_from_spec(spec)
spec.loader.exec_module(setup_script)


class Recorder:
    """Stands in for subprocess.run: records commands, fails on a chosen one."""

    def __init__(self, fail_on=None):
        self.commands = []
        self.fail_on = fail_on

    def __call__(self, command, cwd=None):
        self.commands.append(command)
        code = 1 if self.fail_on and self.fail_on in " ".join(map(str, command)) else 0
        return subprocess.CompletedProcess(command, code)


def everything_installed(name):
    return f"/bin/{name}"


def test_steps_run_in_order_export_then_build_then_validate():
    run = Recorder()
    code = setup_script.main(["--no-install"], runner=run, which=everything_installed, root=ROOT)
    assert code == 0
    words = [" ".join(map(str, c)) for c in run.commands]
    assert "wh_corpus export" in words[0]
    assert words[1].startswith("cargo build --release -p wh-graph -p wh-mcp")
    assert " build --corpus " in words[2]
    assert " validate --bundle " in words[3]
    assert len(words) == 4, "nothing is registered unless asked"


def test_skip_export_reuses_the_corpus():
    run = Recorder()
    setup_script.main(["--no-install", "--skip-export"], runner=run, which=everything_installed, root=ROOT)
    assert not any("wh_corpus" in " ".join(map(str, c)) for c in run.commands)


def test_the_first_failing_step_stops_the_rest(capsys):
    run = Recorder(fail_on="cargo build")
    code = setup_script.main(["--no-install"], runner=run, which=everything_installed, root=ROOT)
    assert code == 1
    assert len(run.commands) == 2, "export ran, cargo failed, the graph build never started"
    assert "Step failed" in capsys.readouterr().err


def test_missing_rust_is_reported_before_anything_runs(capsys):
    run = Recorder()
    code = setup_script.main([], runner=run, which=lambda name: None, root=ROOT)
    assert code == 1
    assert run.commands == []
    assert "rustup" in capsys.readouterr().err


def test_old_python_is_reported():
    with pytest.raises(setup_script.SetupError, match="3.12"):
        setup_script.check_tools(which=everything_installed, version=(3, 11, 0))


def test_register_adds_one_user_scope_command_and_needs_claude(capsys):
    run = Recorder()
    setup_script.main(["--no-install", "--register"], runner=run, which=everything_installed, root=ROOT)
    last = run.commands[-1]
    assert last[:5] == ["claude", "mcp", "add", "--scope", "user"]
    assert "--bundle" in last

    no_claude = lambda name: None if name == "claude" else f"/bin/{name}"
    assert setup_script.main(["--register"], runner=Recorder(), which=no_claude, root=ROOT) == 1
    assert "--register" in capsys.readouterr().err


def test_a_program_that_cannot_start_is_a_clear_error():
    def missing(command, cwd=None):
        raise FileNotFoundError

    with pytest.raises(setup_script.SetupError, match="Could not start"):
        setup_script.run_step("x", ["nope"], ROOT, runner=missing)
