"""Download the edition catalog into a local cache."""

from __future__ import annotations

import hashlib
import time
from collections.abc import Callable
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path

import httpx

from wh_corpus.catalog import CATALOG, USER_AGENT, CatalogFile, assert_header
from wh_corpus.errors import CacheError, ParseError
from wh_corpus.models import CacheMeta, dump_json
from wh_corpus.parse import header_names, parse_csv

RETRY_DELAYS = (1, 2, 4)
TIMEOUT_SECONDS = 60.0


@dataclass(frozen=True)
class FetchResult:
    edition: str
    cache: str
    last_update: str
    fetched_at: str | None
    reused_cache: bool
    urls: tuple[str, ...]
    dry_run: bool


def utc_now_rfc3339(now: datetime | None = None) -> str:
    current = now or datetime.now(timezone.utc)
    current = current.astimezone(timezone.utc).replace(microsecond=0)
    return current.strftime("%Y-%m-%dT%H:%M:%SZ")


def edition_dir(cache_dir: Path, edition: str) -> Path:
    return cache_dir / edition


def file_url(base_url: str, file_name: str) -> str:
    return f"{base_url.rstrip('/')}/{file_name}"


def get_bytes(
    client: httpx.Client,
    url: str,
    *,
    sleep: Callable[[float], None] = time.sleep,
) -> bytes:
    """GET one file, retrying network failures and HTTP 429 or 5xx three times."""
    attempt = 0
    while True:
        try:
            response = client.get(url)
        except httpx.HTTPError as exc:
            if attempt >= len(RETRY_DELAYS):
                raise CacheError(f"network failure: {url}") from exc
            sleep(RETRY_DELAYS[attempt])
            attempt += 1
            continue
        if response.status_code == 429 or response.status_code >= 500:
            if attempt >= len(RETRY_DELAYS):
                raise CacheError(f"network failure: {url}")
            sleep(RETRY_DELAYS[attempt])
            attempt += 1
            continue
        if response.status_code != 200:
            raise CacheError(f"network failure: {url}")
        return response.content


def _cache_complete(directory: Path, meta: CacheMeta | None) -> bool:
    if meta is None:
        return False
    return all((directory / item.file_name).is_file() for item in CATALOG)


def _read_meta(directory: Path) -> CacheMeta | None:
    path = directory / "cache-meta.json"
    if not path.is_file():
        return None
    try:
        return CacheMeta.model_validate_json(path.read_text(encoding="utf-8"))
    except (OSError, ValueError) as exc:
        raise CacheError(f"cache incomplete: {path}") from exc


def _last_update_value(data: bytes, spec: CatalogFile) -> str:
    rows = parse_csv(data, spec.file_name, spec.columns)
    if len(rows) != 1 or rows[0].get("last_update", "") == "":
        raise ParseError(spec.file_name, "expected one last_update value")
    return rows[0]["last_update"]


def _check_header(data: bytes, spec: CatalogFile) -> None:
    assert_header(spec.file_name, header_names(data, spec.file_name), spec.columns)


def fetch(
    *,
    edition: str,
    cache_dir: Path,
    base_url: str,
    force: bool = False,
    dry_run: bool = False,
    sleep: Callable[[float], None] = time.sleep,
    now: Callable[[], datetime] | None = None,
    fetch_command: str,
) -> FetchResult:
    """Download the catalog, or reuse it when last_update is unchanged."""
    directory = edition_dir(cache_dir, edition)
    urls = tuple(file_url(base_url, item.file_name) for item in CATALOG)
    cache_display = str(directory)
    with httpx.Client(
        headers={"User-Agent": USER_AGENT},
        timeout=TIMEOUT_SECONDS,
        trust_env=False,
        follow_redirects=True,
    ) as client:
        try:
            last_bytes = get_bytes(client, urls[0], sleep=sleep)
        except CacheError as exc:
            raise CacheError(f"{exc}\n{fetch_command}") from exc
        last_spec = CATALOG[0]
        _check_header(last_bytes, last_spec)
        remote_update = _last_update_value(last_bytes, last_spec)
        meta = _read_meta(directory)
        reused = (
            not force
            and meta is not None
            and meta.last_update == remote_update
            and _cache_complete(directory, meta)
        )
        if dry_run:
            return FetchResult(
                edition=edition,
                cache=cache_display,
                last_update=remote_update,
                fetched_at=meta.fetched_at if reused and meta is not None else None,
                reused_cache=reused,
                urls=urls,
                dry_run=True,
            )
        if reused and meta is not None:
            return FetchResult(
                edition=edition,
                cache=cache_display,
                last_update=meta.last_update,
                fetched_at=meta.fetched_at,
                reused_cache=True,
                urls=urls,
                dry_run=False,
            )
        fetched_at = utc_now_rfc3339(now() if now else None)
        partial = directory / ".partial"
        if partial.exists():
            _remove_tree(partial)
        partial.mkdir(parents=True, exist_ok=True)
        payloads: dict[str, bytes] = {last_spec.file_name: last_bytes}
        try:
            (partial / last_spec.file_name).write_bytes(last_bytes)
            for spec, url in zip(CATALOG[1:], urls[1:], strict=True):
                try:
                    payload = get_bytes(client, url, sleep=sleep)
                except CacheError as exc:
                    raise CacheError(f"{exc}\n{fetch_command}") from exc
                target = partial / spec.file_name
                target.write_bytes(payload)
                try:
                    _check_header(payload, spec)
                except ParseError:
                    target.unlink(missing_ok=True)
                    raise
                payloads[spec.file_name] = payload
            directory.mkdir(parents=True, exist_ok=True)
            hashes: dict[str, str] = {}
            for spec in CATALOG:
                blob = payloads[spec.file_name]
                hashes[spec.file_name] = hashlib.sha256(blob).hexdigest()
                (directory / spec.file_name).write_bytes(blob)
            written = CacheMeta(
                edition=edition,
                last_update=remote_update,
                fetched_at=fetched_at,
                sha256=hashes,
            )
            meta_path = directory / "cache-meta.json"
            temporary = directory / "cache-meta.json.partial"
            temporary.write_text(dump_json(written.model_dump()) + "\n", encoding="utf-8")
            temporary.replace(meta_path)
        finally:
            _remove_tree(partial)
    return FetchResult(
        edition=edition,
        cache=cache_display,
        last_update=remote_update,
        fetched_at=fetched_at,
        reused_cache=False,
        urls=urls,
        dry_run=False,
    )


def _remove_tree(path: Path) -> None:
    if not path.exists():
        return
    for child in sorted(path.rglob("*"), reverse=True):
        if child.is_file() or child.is_symlink():
            child.unlink()
        else:
            child.rmdir()
    path.rmdir()
