"""The questions used to compare local models, with checkable expectations.

Each expectation comes from the real 10th-edition bundle. A run passes when the
model called one of the expected tools AND its answer contains every expected
fact (case-insensitive; "a|b" means either spelling).
"""

from __future__ import annotations

from dataclasses import dataclass


@dataclass(frozen=True)
class Question:
    id: str
    text: str
    # At least one of these tools must have been called.
    tools: tuple[str, ...]
    # Every entry must appear in the answer. "a|b" accepts either.
    contains: tuple[str, ...]
    note: str = ""


QUESTIONS: tuple[Question, ...] = (
    Question(
        "angron_invuln",
        "What invulnerable save does Angron have?",
        ("get_unit",),
        ("4+",),
        "One unit lookup; the save is a typed field.",
    ),
    Question(
        "angron_weapons",
        "What are Angron's weapon profiles? Include strength and damage.",
        ("get_unit",),
        ("strike", "sweep", "14"),
        "Strike and sweep are two separate profiles.",
    ),
    Question(
        "warboss_points",
        "How many points does a Warboss cost?",
        ("get_unit",),
        ("75",),
        "Several Warboss variants exist; the plain one is 75.",
    ),
    Question(
        "dropship_transport",
        "How many models can the Orion Assault Dropship transport?",
        ("get_unit",),
        ("12",),
        "Transport capacity is in the datasheet text, not a field.",
    ),
    Question(
        "imotekh_leads",
        "What units can Imotekh the Stormlord lead?",
        ("get_unit",),
        ("Immortals", "Lychguard", "Necron Warriors"),
        "Needs the leads section.",
    ),
    Question(
        "khorne_units",
        "How many units can a Khorne daemon army take?",
        ("get_roster",),
        ("21|twenty-one",),
        "A roster the model must count.",
    ),
    Question(
        "idols_of_khorne",
        "Which units get the Idols of Khorne detachment rule?",
        ("units_for_rule",),
        ("Angron", "Daemon Prince of Khorne"),
        "Needs the rule-to-units tool, not a unit lookup.",
    ),
    Question(
        "fnp_count",
        "How many units have the Feel No Pain ability?",
        ("units_with_ability",),
        ("116",),
        "The reply carries a total, so the model need not count rows.",
    ),
    Question(
        "auric_stratagems",
        "How many stratagems does the Auric Champions detachment have?",
        ("get_detachment",),
        ("6|six",),
        "Count a list of stratagems.",
    ),
    Question(
        "typo_recovery",
        "Tell me about the Taxtical Squad.",
        ("get_unit", "find_units", "search"),
        ("Tactical Squad",),
        "The error's suggestions miss it, so the model must correct the spelling itself.",
    ),
    Question(
        "daemonic_incursion",
        "What are the rules of the Daemonic Incursion detachment?",
        ("get_detachment",),
        ("Boarding Actions",),
        "Two detachments share this name; the model must notice and use an id.",
    ),
)

BY_ID = {question.id: question for question in QUESTIONS}

# Models write typographic dashes, quotes, and spaces. Treat them as their plain
# forms so "twenty‑one" and "Gork’s" match what the question expects.
_PLAIN = str.maketrans(
    {
        **{ord(dash): "-" for dash in "‐‑‒–—―−"},
        **{ord(quote): "'" for quote in "‘’ʼ`"},
        **{ord(quote): '"' for quote in "“”"},
        ord(" "): " ",
        ord(" "): " ",
    }
)


def plain(text: str) -> str:
    """Lowercase `text` with typographic punctuation replaced by its plain form."""
    return text.translate(_PLAIN).lower()


def passes(question: Question, text: str, called: set[str]) -> tuple[bool, bool]:
    """Whether the tools used and the facts stated meet the expectation: (tools_ok, text_ok)."""
    tools_ok = any(tool in called for tool in question.tools)
    answer = plain(text)
    text_ok = all(any(plain(option) in answer for option in item.split("|")) for item in question.contains)
    return tools_ok, text_ok
