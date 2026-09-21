"""Turn pipe-delimited export bytes into logical records."""

from __future__ import annotations

from wh_corpus.catalog import assert_header
from wh_corpus.errors import ParseError


def decode_export(data: bytes, filename: str) -> str:
    """UTF-8 text with one leading BOM removed and newlines normalized to LF."""
    try:
        text = data.decode("utf-8")
    except UnicodeDecodeError as exc:
        raise ParseError(filename, "UTF-8 decode failed") from exc
    if text.startswith("\ufeff"):
        text = text[1:]
    return text.replace("\r\n", "\n").replace("\r", "\n")


def _is_skippable(line: str) -> bool:
    return line == "" or line.strip("|") == ""


def header_names(data: bytes, filename: str) -> list[str]:
    """Names from the first physical line, with one trailing empty field removed."""
    text = decode_export(data, filename)
    if text == "":
        raise ParseError(filename, "empty file")
    first = text.split("\n", 1)[0]
    return _strip_header(first, filename)[0]


def _strip_header(first: str, filename: str) -> tuple[list[str], int, bool]:
    raw = first.split("|")
    drop_trailing = raw[-1] == ""
    names = raw[:-1] if drop_trailing else raw
    return names, len(raw), drop_trailing


def parse_csv(data: bytes, filename: str, expected: tuple[str, ...] | list[str]) -> list[dict[str, str]]:
    """Parse one catalog file into rows keyed by header name."""
    text = decode_export(data, filename)
    if text == "":
        raise ParseError(filename, "empty file")
    lines = text.split("\n")
    names, raw_width, drop_trailing = _strip_header(lines[0], filename)
    assert_header(filename, names, expected)
    records: list[dict[str, str]] = []
    buffer: str | None = None
    for line in lines[1:]:
        if buffer is None:
            if _is_skippable(line):
                continue
            buffer = line
        else:
            buffer = f"{buffer}\n{line}"
        fields = buffer.split("|")
        if len(fields) > raw_width:
            raise ParseError(filename, "record has more fields than the header")
        if len(fields) == raw_width:
            if drop_trailing:
                fields = fields[:-1]
            if len(fields) != len(names):
                raise ParseError(filename, "record field count does not match the header")
            records.append({names[index]: fields[index] for index in range(len(names))})
            buffer = None
    if buffer is not None:
        raise ParseError(filename, "unfinished record")
    return records
