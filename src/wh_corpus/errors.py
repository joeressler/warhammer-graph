"""Exit-code errors for the corpus CLI."""


class CorpusError(Exception):
    """Failure that maps to a documented process exit code."""

    exit_code = 1


class UsageError(CorpusError):
    """Unknown edition, missing flag, or bad --output."""

    exit_code = 2


class CacheError(CorpusError):
    """Network failure, or a cache that is incomplete or hashed incorrectly."""

    exit_code = 3


class ParseError(CorpusError):
    """CSV bytes could not be turned into logical records."""

    exit_code = 4

    def __init__(self, filename: str, reason: str) -> None:
        self.filename = filename
        self.reason = reason
        super().__init__(f"{filename}: {reason}")


class HeaderDriftError(ParseError):
    """A catalog file's header names do not match the frozen set."""

    def __init__(self, filename: str, expected: list[str], actual: list[str]) -> None:
        self.expected = expected
        self.actual = actual
        expected_text = ", ".join(expected)
        actual_text = ", ".join(actual)
        super().__init__(
            filename,
            f"header drift\nexpected: {expected_text}\nactual: {actual_text}",
        )


class ValidationFailed(CorpusError):
    """Schema or extra corpus checks failed."""

    exit_code = 5

    def __init__(self, details: str, corpus: str) -> None:
        self.details = details
        self.corpus = corpus
        command = f"wh-corpus validate --corpus {corpus}"
        super().__init__(f"{details}\n{command}")
