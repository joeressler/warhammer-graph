"""The grading rules behind the results table."""

from __future__ import annotations

from questions import BY_ID, QUESTIONS, passes, plain


def test_every_question_has_a_unique_id_a_tool_and_an_expected_fact() -> None:
    assert len({question.id for question in QUESTIONS}) == len(QUESTIONS) == len(BY_ID)
    for question in QUESTIONS:
        assert question.text.endswith(("?", ".")), question.id
        assert question.tools and question.contains, question.id


def test_a_run_needs_the_right_tool_and_every_fact() -> None:
    question = BY_ID["imotekh_leads"]
    full = "He can lead Immortals, Lychguard and Necron Warriors."
    assert passes(question, full, {"get_unit"}) == (True, True)
    assert passes(question, "He can lead Immortals and Lychguard.", {"get_unit"}) == (True, False)
    assert passes(question, full, {"search"}) == (False, True)
    assert passes(question, "", set()) == (False, False)


def test_matching_ignores_case_and_accepts_either_spelling() -> None:
    count = BY_ID["khorne_units"]
    assert passes(count, "There are 21 units.", {"get_roster"}) == (True, True)
    assert passes(count, "Twenty-One units.", {"get_roster"}) == (True, True)
    assert passes(count, "There are 22 units.", {"get_roster"}) == (True, False)


def test_any_of_the_listed_tools_counts() -> None:
    typo = BY_ID["typo_recovery"]
    for tool in ("get_unit", "find_units", "search"):
        assert passes(typo, "That is the Tactical Squad.", {tool})[0]
    assert not passes(typo, "That is the Tactical Squad.", {"get_roster"})[0]


def test_typographic_dashes_quotes_and_spaces_match_their_plain_forms() -> None:
    count = BY_ID["khorne_units"]
    # A non-breaking hyphen, as a model wrote it.
    assert passes(count, "Twenty\u2011one units.", {"get_roster"}) == (True, True)
    assert passes(count, "Twenty\u2013one units.", {"get_roster"}) == (True, True)
    weapons = BY_ID["angron_weapons"]
    assert passes(weapons, "Strike and sweep, S\u00a014.", {"get_unit"}) == (True, True)
    assert plain("Gork\u2019s Klaw \u2013 strike") == "gork's klaw - strike"
