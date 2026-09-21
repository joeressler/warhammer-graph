"""Corpus v1 checks from the CLI spec. The suite does not call wahapedia.ru."""

from __future__ import annotations

import hashlib
import json
import threading
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

import httpx
import pytest
from typer.testing import CliRunner

from tests.fixture_catalog import (
    FETCHED_AT,
    INLINE_DESCRIPTION,
    LAST_UPDATE,
    fill,
    render,
    sample_rows,
    write_cache,
)
from wh_corpus.assemble import assemble
from wh_corpus.catalog import FETCH_EXAMPLE, SOURCE_URL
from wh_corpus.cli import app
from wh_corpus.errors import CacheError, HeaderDriftError
from wh_corpus.fetch import fetch, get_bytes
from wh_corpus.models import CacheMeta, dump_json

runner = CliRunner()


class CatalogServer:
    """In-process HTTP fixture for fetch reuse."""

    def __init__(self, files: dict[str, bytes]) -> None:
        self.files = files
        self.hits: dict[str, int] = {}
        self.agents: list[str | None] = []
        files_ref = self.files
        hits = self.hits
        agents = self.agents

        class Handler(BaseHTTPRequestHandler):
            def do_GET(self) -> None:  # noqa: N802
                name = self.path.split("?", 1)[0].rstrip("/").split("/")[-1]
                hits[name] = hits.get(name, 0) + 1
                agents.append(self.headers.get("User-Agent"))
                body = files_ref.get(name)
                if body is None:
                    self.send_response(404)
                    self.end_headers()
                    return
                self.send_response(200)
                self.send_header("Content-Length", str(len(body)))
                self.end_headers()
                self.wfile.write(body)

            def log_message(self, _format: str, *_args: object) -> None:
                return

        self._httpd = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
        self._thread = threading.Thread(target=self._httpd.serve_forever, daemon=True)
        self._thread.start()
        host, port = self._httpd.server_address
        self.base = f"http://{host}:{port}/"

    def stop(self) -> None:
        self._httpd.shutdown()
        self._httpd.server_close()


def _entities(corpus) -> list[dict]:
    text = (corpus / "entities.jsonl").read_text(encoding="utf-8")
    return [json.loads(line) for line in text.splitlines()]


def test_help_lists_commands_and_wahapedia_credit() -> None:
    result = runner.invoke(app, ["--help"])
    assert result.exit_code == 0
    assert "powered by Wahapedia" in result.stdout
    for name in ("fetch", "assemble", "validate", "export"):
        assert name in result.stdout
    for name in ("fetch", "assemble", "validate", "export"):
        command_help = runner.invoke(app, [name, "--help"])
        assert command_help.exit_code == 0
        assert "Examples" in command_help.stdout
    fetch_help = runner.invoke(app, ["fetch", "--help"])
    assert "wh-corpus fetch --edition 10ed --cache-dir ./cache" in fetch_help.stdout


def test_unknown_edition_exits_2() -> None:
    result = runner.invoke(app, ["fetch", "--edition", "11ed", "--cache-dir", "cache"])
    assert result.exit_code == 2
    assert "implemented edition: 10ed" in result.stderr
    assert "wh-corpus fetch --edition 10ed --cache-dir ./cache" in result.stderr


def test_fixture_assembles_schema_valid_and_stable(tmp_path) -> None:
    cache = tmp_path / "cache"
    out = tmp_path / "corpus"
    write_cache(cache, render(sample_rows()))
    first = assemble(
        edition="10ed",
        cache_dir=cache,
        out=out,
        fetch_command=FETCH_EXAMPLE,
    )
    manifest_bytes = (out / "manifest.json").read_bytes()
    entity_bytes = (out / "entities.jsonl").read_bytes()
    second = assemble(
        edition="10ed",
        cache_dir=cache,
        out=out,
        fetch_command=FETCH_EXAMPLE,
    )
    assert first.entities == second.entities
    assert (out / "manifest.json").read_bytes() == manifest_bytes
    assert (out / "entities.jsonl").read_bytes() == entity_bytes
    validated = runner.invoke(app, ["validate", "--corpus", str(out)])
    assert validated.exit_code == 0
    assert "valid: true" in validated.stdout
    manifest = json.loads(manifest_bytes)
    assert manifest["schema_version"] == 1
    assert manifest["fetched_at"] == FETCHED_AT
    assert manifest["source_url"] == SOURCE_URL
    assert manifest["warnings"] == []
    assert entity_bytes.endswith(b"\n")
    assert b"\n\n" not in entity_bytes


def test_description_keeps_newline_and_html(tmp_path) -> None:
    cache = tmp_path / "cache"
    out = tmp_path / "corpus"
    write_cache(cache, render(sample_rows()))
    assemble(edition="10ed", cache_dir=cache, out=out, fetch_command=FETCH_EXAMPLE)
    inline = next(
        entity
        for entity in _entities(out)
        if entity["table"] == "datasheet_ability" and entity["fields"]["line"] == "1"
    )
    assert inline["fields"]["description"] == INLINE_DESCRIPTION
    assert "\n" in inline["fields"]["description"]
    assert "<p>" in inline["fields"]["description"]
    abilities = [entity for entity in _entities(out) if entity["table"] == "datasheet_ability"]
    assert len(abilities) == 2


def test_header_drift_exits_4_and_preserves_out(tmp_path) -> None:
    cache = tmp_path / "cache"
    out = tmp_path / "corpus"
    write_cache(cache, render(sample_rows()))
    assemble(edition="10ed", cache_dir=cache, out=out, fetch_command=FETCH_EXAMPLE)
    before = (
        (out / "manifest.json").read_bytes(),
        (out / "entities.jsonl").read_bytes(),
    )
    drifted = b"id|name|link|extra|\nEX1|Example Faction|https://example.invalid|x|\n"
    faction = cache / "10ed" / "Factions.csv"
    faction.write_bytes(drifted)
    meta_path = cache / "10ed" / "cache-meta.json"
    meta = CacheMeta.model_validate_json(meta_path.read_text(encoding="utf-8"))
    meta.sha256["Factions.csv"] = hashlib.sha256(drifted).hexdigest()
    meta_path.write_text(dump_json(meta.model_dump()) + "\n", encoding="utf-8")
    result = runner.invoke(
        app,
        ["assemble", "--edition", "10ed", "--cache-dir", str(cache), "--out", str(out)],
    )
    assert result.exit_code == 4
    assert "Factions.csv" in result.stderr
    assert "expected:" in result.stderr
    assert "actual:" in result.stderr
    assert (
        (out / "manifest.json").read_bytes(),
        (out / "entities.jsonl").read_bytes(),
    ) == before


def test_unresolved_faction_id_warns_and_keeps_row(tmp_path) -> None:
    cache = tmp_path / "cache"
    out = tmp_path / "corpus"
    write_cache(cache, render(sample_rows(faction_id="MISSING")))
    result = runner.invoke(
        app,
        ["assemble", "--edition", "10ed", "--cache-dir", str(cache), "--out", str(out)],
    )
    assert result.exit_code == 0
    assert "warnings: 1" in result.stdout
    datasheet = next(entity for entity in _entities(out) if entity["table"] == "datasheet")
    assert datasheet["fields"]["faction_id"] == "MISSING"
    assert datasheet["refs"]["faction_id"] is None
    manifest = json.loads((out / "manifest.json").read_text(encoding="utf-8"))
    assert manifest["warnings"] == [
        {
            "code": "unresolved_foreign_key",
            "entity_id": datasheet["id"],
            "column": "faction_id",
            "value": "MISSING",
        }
    ]


def test_blank_ability_id_is_null_without_warning(tmp_path) -> None:
    cache = tmp_path / "cache"
    out = tmp_path / "corpus"
    write_cache(cache, render(sample_rows()))
    assemble(edition="10ed", cache_dir=cache, out=out, fetch_command=FETCH_EXAMPLE)
    inline = next(
        entity
        for entity in _entities(out)
        if entity["table"] == "datasheet_ability" and entity["fields"]["ability_id"] == ""
    )
    assert inline["refs"]["ability_id"] is None
    assert inline["refs"]["datasheet_id"] == "10ed:datasheet:EXDS"
    manifest = json.loads((out / "manifest.json").read_text(encoding="utf-8"))
    assert manifest["warnings"] == []


def test_duplicate_keyword_occurrence_segment(tmp_path) -> None:
    keyword = fill(
        "Datasheets_keywords.csv",
        datasheet_id="EXDS",
        keyword="Infantry",
        model="Example",
        is_faction_keyword="false",
    )
    cache = tmp_path / "cache"
    out = tmp_path / "corpus"
    write_cache(cache, render(sample_rows(keywords=[keyword, dict(keyword)])))
    assemble(edition="10ed", cache_dir=cache, out=out, fetch_command=FETCH_EXAMPLE)
    keywords = [entity for entity in _entities(out) if entity["table"] == "datasheet_keyword"]
    assert [entity["id"] for entity in keywords] == [
        "10ed:datasheet_keyword:EXDS:Infantry:Example",
        "10ed:datasheet_keyword:EXDS:Infantry:Example:2",
    ]


def test_get_bytes_retries_server_errors_and_not_404() -> None:
    calls = {"n": 0}

    def succeed_after_two(request: httpx.Request) -> httpx.Response:
        calls["n"] += 1
        if calls["n"] < 3:
            return httpx.Response(503)
        return httpx.Response(200, content=b"ok")

    with httpx.Client(transport=httpx.MockTransport(succeed_after_two)) as client:
        assert get_bytes(client, "https://example.invalid/Factions.csv", sleep=lambda _delay: None) == b"ok"
    assert calls["n"] == 3

    misses = {"n": 0}

    def missing(_request: httpx.Request) -> httpx.Response:
        misses["n"] += 1
        return httpx.Response(404)

    with httpx.Client(transport=httpx.MockTransport(missing)) as client:
        with pytest.raises(CacheError):
            get_bytes(client, "https://example.invalid/Factions.csv", sleep=lambda _delay: None)
    assert misses["n"] == 1


def test_fetch_reuses_then_redownloads(tmp_path) -> None:
    files = render(sample_rows())
    server = CatalogServer(files)
    cache = tmp_path / "cache"
    try:
        first = fetch(
            edition="10ed",
            cache_dir=cache,
            base_url=server.base,
            fetch_command=FETCH_EXAMPLE,
        )
        assert first.reused_cache is False
        assert first.last_update == LAST_UPDATE
        hits = dict(server.hits)
        assert hits["Factions.csv"] == 1
        assert server.agents[0] == "wh-corpus/1 (+https://wahapedia.ru; powered by Wahapedia)"
        second = fetch(
            edition="10ed",
            cache_dir=cache,
            base_url=server.base,
            fetch_command=FETCH_EXAMPLE,
        )
        assert second.reused_cache is True
        assert second.fetched_at == first.fetched_at
        assert server.hits["Last_update.csv"] == hits["Last_update.csv"] + 1
        assert server.hits["Factions.csv"] == hits["Factions.csv"]
        forced = fetch(
            edition="10ed",
            cache_dir=cache,
            base_url=server.base,
            force=True,
            fetch_command=FETCH_EXAMPLE,
        )
        assert forced.reused_cache is False
        assert server.hits["Factions.csv"] == hits["Factions.csv"] + 1
        files["Last_update.csv"] = render(sample_rows(last_update="2026-09-22 08:00:00"))[
            "Last_update.csv"
        ]
        third = fetch(
            edition="10ed",
            cache_dir=cache,
            base_url=server.base,
            fetch_command=FETCH_EXAMPLE,
        )
        assert third.reused_cache is False
        assert third.last_update == "2026-09-22 08:00:00"
        assert server.hits["Factions.csv"] == hits["Factions.csv"] + 2
        assert "wahapedia.ru" not in server.base
    finally:
        server.stop()


def test_validate_ok_and_rejects_schema_version(tmp_path) -> None:
    cache = tmp_path / "cache"
    out = tmp_path / "corpus"
    write_cache(cache, render(sample_rows()))
    assemble(edition="10ed", cache_dir=cache, out=out, fetch_command=FETCH_EXAMPLE)
    ok = runner.invoke(app, ["validate", "--corpus", str(out), "--output", "json"])
    assert ok.exit_code == 0
    payload = json.loads(ok.stdout)
    assert payload["valid"] is True
    assert payload["schema_version"] == 1
    manifest_path = out / "manifest.json"
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    manifest["schema_version"] = 2
    manifest_path.write_text(dump_json(manifest) + "\n", encoding="utf-8")
    rejected = runner.invoke(app, ["validate", "--corpus", str(out)])
    assert rejected.exit_code == 5
    assert "wh-corpus validate --corpus" in rejected.stderr


def test_fetch_header_drift_leaves_previous_cache(tmp_path) -> None:
    files = render(sample_rows())
    server = CatalogServer(files)
    cache = tmp_path / "cache"
    try:
        fetch(
            edition="10ed",
            cache_dir=cache,
            base_url=server.base,
            fetch_command=FETCH_EXAMPLE,
        )
        original = (cache / "10ed" / "Factions.csv").read_bytes()
        meta_before = (cache / "10ed" / "cache-meta.json").read_bytes()
        files["Last_update.csv"] = render(sample_rows(last_update="2026-09-22 08:00:00"))[
            "Last_update.csv"
        ]
        files["Factions.csv"] = b"id|title|link|\nEX1|Example Faction|https://example.invalid|\n"
        with pytest.raises(HeaderDriftError):
            fetch(
                edition="10ed",
                cache_dir=cache,
                base_url=server.base,
                fetch_command=FETCH_EXAMPLE,
            )
        assert (cache / "10ed" / "Factions.csv").read_bytes() == original
        assert (cache / "10ed" / "cache-meta.json").read_bytes() == meta_before
        assert not (cache / "10ed" / ".partial").exists()
    finally:
        server.stop()
