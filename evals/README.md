# Evals: measuring the tool-using agent before changing it

This directory measures one thing: **does the agent answer numeric and count questions correctly by calling the MCP tools, and does it decline when the graph cannot answer?** It is built before any prompt tuning, verifier, or post-training, because you cannot tell whether a change helped without a fixed test and a recorded starting point.

> **The baseline is the control.** Any later work (a better prompt, a verifier step, fine-tuning) has to beat the numbers in the baseline summary on the same `golden.jsonl`, with the same grader, before it counts as an improvement.

## What runs what

```text
golden.jsonl ──► run.py ──► examples/ollama-host/host.py (the production agent) ──► wh-mcp ──► bundle
                   │                      │
                   │                      └─ Ollama only serves the model
                   ▼
        results/<run_id>/<id>.json  (one full trace per question)
                   │
                   ▼
              grade.py ──► summary.json, summary.md
```

- **Production agent.** `host.Host` in `examples/ollama-host/host.py` is the loop that sends tools to the model, runs its tool calls through MCP, and feeds results back. The evals import it (`evals/agent.py`) and add no chat loop of their own.
- **Ollama** serves the model and nothing else. Tests and the runner orchestrate, and the agent code executes.
- Each question gets a **fresh conversation** over one running `wh-mcp` server. The server is read-only, so sharing it carries no state between questions.
- **No model grades anything.** Grading is plain string and number checks, so the same traces always get the same scores.

| File | Role |
| --- | --- |
| `golden.jsonl` | The 60 questions with their expected tools, arguments, and answers. A static fixture. |
| `author.py` | Computes the expected answers from the bundle and writes `golden.jsonl`. Run it only when the questions or the game data change. |
| `run.py` | The runner: `python -m evals.run`. |
| `grade.py` | The grader: `python -m evals.grade <results dir>`. |
| `test_golden_run.py` | The runner as a pytest test, marked `eval`. |
| `test_eval_harness.py` | Offline tests of the golden file, the grader, and the runner (scripted fake model, real `wh-mcp`). |
| `baseline/` | The control: one graded summary from a real run on this machine (Ollama). |
| `baseline-llamacpp/` | The same model and questions served by llama.cpp, for comparing against a llama.cpp-served model. |
| `comparison/` | Five models on both servers, one graded sample each, with a generated table (`python -m evals.compare evals/results/*`) and notes. |
| `results/` | Your runs. Gitignored. |

**Keep `golden.jsonl` out of any training data.** If a model is ever fine-tuned, these questions and answers must not be in what it trains on, or the score stops measuring anything.

## The golden set

60 questions, one JSON object per line:

| Field | Meaning |
| --- | --- |
| `id` | A stable name. Never reuse or renumber one: a result is only comparable across runs by id. |
| `question` | Exactly what the agent is asked. |
| `expected_tools` | Tools the agent should call. Every entry must be called. `a\|b` means either tool satisfies that entry. An empty list means no tool is required (a question the graph cannot answer, so there is nothing to look up). |
| `expected_args` | Constraints per tool, as `{"get_unit": {"unit": "Angron"}}`. A string is matched as a normalized name (case, `a/an/the`, and punctuation ignored; either side may contain the other), and `a\|b` accepts either. A **list** means each item must appear in some call of that tool, which is how a comparison requires both units to be looked up. An id the agent got from an earlier lookup of the name also counts as the name. |
| `expected_answer` | A number, a string, or `"yes"`/`"no"`. `null` for `refuse`. |
| `type` | `lookup`, `count`, `compare`, or `refuse`. |
| `notes` | Optional. |

| Type | Count | What it tests |
| --- | --- | --- |
| `lookup` | 22 | One typed fact: a unit's Toughness, Wounds, save, points, a stratagem's command point cost, an enhancement's cost. |
| `count` | 19 | "How many…": a faction's roster, units with a keyword or ability, a detachment's stratagems, rules, or enhancements, the factions. |
| `compare` | 10 | Yes/no comparisons of two units' stats or points, the higher of two values, and which keyword has more units. They end with "Answer yes or no first." |
| `refuse` | 9 | Questions the graph cannot answer: invented units, an invented stratagem, faction, or keyword, a characteristic that does not exist, another edition, real-world prices. |

**Every expected answer is computed from the live bundle** by counting its `nodes.jsonl` and `edges.jsonl` in `author.py`, not through `wh-ask`, so the answer key does not depend on the code being tested. `python -m evals.author --check` then calls the expected tools directly, with no model, and confirms every expected number appears in what they return. That check is why a *correct* count is never counted as a hallucination (see below): count questions only use numbers the tools state, such as the `count` on a roster or the `total` on a keyword list. A question a model could answer only by counting rows would be unfair under the hallucination definition, so there are none.

If the game data changes: rebuild the bundle, run `python -m evals.author --write --check`, read the diff to `golden.jsonl`, and treat the result as a **new golden set** with a new baseline. Scores from before are no longer comparable.

## Scoring contract

For each question the grader records four verdicts.

**Answer exact match (primary).** Over `lookup`, `count`, and `compare` questions, the fraction whose final answer states the expected fact:

- A **number** answer must appear as a standalone number in the final text. Number words count (`twenty-one` is 21), commas and a trailing `.0` are ignored, and the answer is read after typographic dashes and quotes are made plain. `D6`, `2D6`, `S5`, and `10th` are not numbers. Numbered list markers (`1.`) are ignored.
- A `count` answer is judged on the **first number it states**, the headline, which must be the expected one. "It has 6 stratagems. 1. A costs 1 CP." passes a question whose answer is 6; "It has 4 enhancements and 6 stratagems" does not. Numbers the question itself contains are skipped, and so is the word "one" when it is not the first number, since in prose it is rarely a quantity.
- A **string** answer must appear in the text, case-insensitively. `a|b` accepts either.
- A **yes/no** answer is the first standalone `yes` or `no` in the text.
- An empty answer, an error, or a timeout is a failure.

**Tool correct (secondary).** Over questions that have `expected_tools`, the fraction where every expected tool was called with the expected arguments. This is scored separately, so an agent that guessed the right number without calling a tool is visible.

**Abstain correct (secondary).** Over `refuse` questions, the fraction where the agent declined. The answer must contain a refusal phrase ("could not find", "no such", "not in the", and similar, listed in `grade.py` as `REFUSAL`) and state **no number that is not grounded** in a tool result or the question. A refusal that adds an invented figure fails.

**Hallucination rate (secondary).** Over *all* questions, the fraction of final answers containing a number that **never appears in any tool result payload for that trace**. Details:

- The word "one" in an answer is not treated as a number here ("up to one enhancement"), though other number words are.
- The numbers in a tool result are every digit sequence in its text, after removing node ids (`10ed:datasheet:000000882`) so an id fragment cannot ground a number.
- A number the user wrote in the question is exempt: repeating it is not a claim about the data.
- The game's name (`Warhammer 40,000`) is exempt.
- It is scored **independently of tool correct**. A right number from memory, with no tool call, is flagged as a hallucination even though the answer matches. That is deliberate: the aim is an agent whose numbers come from tools.
- Known limit: a number the model computes correctly from tool data, such as a sum or a count it made by counting rows, is flagged, because it is in no payload. That is why the golden counts all use numbers the tools print.
- Known limit: a wrong answer built from a tool result for the *wrong thing* (for example a fuzzy match to a different unit) is not a hallucination by this definition, because the number did come from a tool. It shows up as a failed answer or abstain instead.

`summary.json` also has per-type aggregates, the number of runs that errored or hit the step limit, and the median seconds per question.

## Running a baseline

You need Ollama running with the model pulled, the real bundle built, and `wh-mcp` built. If you have not built them, `python scripts/setup.py` does all three.

```bash
ollama pull granite4.1:8b
cargo build -p wh-mcp --release
```

Run the whole golden set through the real agent, either way:

```bash
python -m evals.run --model granite4.1:8b
pytest evals -m eval --eval-model granite4.1:8b -s
```

To use llama.cpp's `llama-server` instead of Ollama, start it with `python examples/ollama-host/serve_llamacpp.py --ollama-model granite4.1:8b` and add `--backend llamacpp` (CLI) or `--eval-backend llamacpp` (pytest). See the [llama.cpp host guide](../docs/getting-started/llamacpp-host.md). The backend is recorded in `run.json` and in the run id.

The first is the CLI. The second is the same run as a pytest test, which fails if the pipeline breaks and prints the aggregate. Plain `pytest` runs only the offline tests and skips the live one.

Useful options (CLI names; the pytest ones are `--eval-model`, `--eval-ids`, `--eval-run-id`, `--eval-temperature`, `--eval-results-dir`):

| Option | Meaning |
| --- | --- |
| `--run-id NAME` | Name the run. The default is `<UTC timestamp>_<model>`. |
| `--ids a,b` / `--limit N` | A subset, for a quick check. A subset is not a baseline. |
| `--temperature 0.0` | The baseline uses 0, so a re-run is as close to repeatable as a local model gets. Say so when you use anything else. |
| `--num-ctx`, `--max-steps`, `--max-tool-chars` | The host's own settings. Leave them at their defaults for a baseline. |
| `--no-grade` | Save traces only. Grade later with `python -m evals.grade evals/results/<run_id>`. |

A run saves `evals/results/<run_id>/<id>.json` for every question, `run.json` (the model, settings, git commit, bundle fingerprint, and a hash of `golden.jsonl`), and then `summary.json` and `summary.md`.

**What a trace holds.** `final_text`, `error`, `stopped` (`answered`, `step_limit`, or `error`), `tool_calls` (each with the tool, arguments, the full result text the model saw, and timing), and `messages`, the whole conversation including the system prompt.

**When the run fails.** A timeout, a step limit, or one bad reply is recorded as a failed question and the run continues. If the agent path itself breaks (Ollama or a model is unreachable), the run stops, keeps every trace it has, marks `run.json` as `aborted`, and the CLI exits 1. Exit 2 means it could not start (no `wh-mcp`, no bundle, no model).

## Reading the results

Open `summary.md`:

1. **Aggregate.** Answer exact match is the headline. Read hallucination next to it: a high answer score with a high hallucination rate means the agent is right from memory, not from the tools.
2. **By type.** `lookup` is the easy floor. `count` shows whether the agent reads a stated total instead of counting. `refuse` shows whether it invents things when the graph is silent.
3. **Per question.** Each row gives the three verdicts, any hallucinated numbers, the expected answer, and what the agent said. To see why a row failed, open `results/<run_id>/<id>.json` and read `tool_calls` and `final_text`.

Compare two runs by their aggregate and by id. One run of one model at temperature 0 is one sample, so a difference of a question or two is noise. Run more than once, or at a nonzero temperature, before claiming a small gain.

## Baseline

The checked-in sample is [`baseline/`](baseline/): the graded summary of a full run on this machine (RTX 5000 Ada laptop GPU, Ollama), with `granite4.1:8b` at temperature 0 and the host's default settings. It exists so you can see the format, and it is the control. The full traces stay in the gitignored `results/`. The answers in the sample are cut to 60 characters, because the full text can quote published rules, which this repository does not commit.

Run 2026-10-07, `granite4.1:8b`, temperature 0, 60 questions, bundle fingerprint `ebdb143faa6578e4`:

| Metric | Baseline |
| --- | --- |
| **Answer exact match** (lookup, count, compare) | **98.0%** (50/51) |
| Tool correct | 98.2% (55/56) |
| **Abstain correct** (refuse) | **33.3%** (3/9) |
| Hallucination rate | 1.7% (1/60) |
| Median seconds per question | 5.5 |

By type, answer exact match is 95.5% on lookup (21/22), 100% on count (19/19), and 100% on compare (10/10).

What the baseline says, so later work knows where the room is:

- **The numbers are no longer the problem.** Given a typed field or a stated total, this model reads it correctly. The one lookup miss is the Orion Assault Dropship's transport capacity, which is free text in the datasheet ("1 model" instead of 12).
- **Declining is.** It declined 3 of 9 refusals. For an invented unit it matched the nearest real one ("Lord Zarthus the Undying" became a Necron Lord with Toughness 5). For "Toughness in 9th edition" it answered with the 10th edition number, and for "price in dollars" it answered with the points cost. Those numbers *came from a tool*, so the hallucination rate (1.7%) does not catch them; the abstain rate does.
- **A right answer from the wrong place.** On `count_black_ship_guardians_enhancements` the answer matched (2) but the tool check failed: the agent never found that detachment and answered from a different one. Tool correct and answer correct are separate on purpose.
- It is one run at temperature 0, so treat a one-or-two question change as noise.

The first live attempt at this run was graded before two grader fixes (a count is judged on its first number, and the word "one" in prose is not a quantity). Grading is deterministic from the saved traces, so the traces were graded again with the final grader; the model was not re-run.

Later prompt, verifier, or fine-tuning work must beat these numbers on the same golden set, same grader, same settings. A new golden set (see above) needs a new baseline.

The same model served by llama.cpp (`baseline-llamacpp/`, build b11476, same weights from the same file) scored 96.1% answer exact match, 96.4% tool correct, 33.3% abstain correct, and 3.3% hallucination, with a median of 4.5 s per question. That is within one run's noise of the Ollama control. Compare a model with a baseline from the same server, build, and context size.

Two grader fixes were made after the first runs and the traces were re-graded each time without re-running the model: a count is judged on its first number and the word "one" is not a quantity; a node id the agent cites in its answer (`10ed:datasheet:000000614`) is not a number. Re-grading left the Ollama control's numbers unchanged.

## Why this exists

An agent that calls tools can fail in ways a plain model cannot, such as choosing the wrong tool, reading the reply wrong, or stating a number it never retrieved, and each needs a different fix. A fixed question set with automatic grading turns "it seems better" into a number. This is the measurement step that comes before post-training: build the test, record the baseline, then try to move it.
