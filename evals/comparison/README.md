# Five models on two servers

The golden set (60 questions, `../golden.jsonl`) run once per model per server, temperature 0, the host's default settings, 16,384-token context, on 2026-10-07 (RTX 5000 Ada laptop GPU, 16 GB). Generated with `python -m evals.compare evals/results/*`.

| Model | Server | Answer | Tool | Abstain | Hallucination | Median s | Errors | Step limit |
|---|---|---|---|---|---|---|---|---|
| `gpt-oss:20b` | llamacpp | 98.0% | 98.2% | 44.4% | 1.7% | 4.6 | 0 | 4 |
| `gpt-oss:20b` | ollama | 94.1% | 98.2% | 55.6% | 1.7% | 5.3 | 2 | 1 |
| `granite4.1:3b` | llamacpp | 90.2% | 98.2% | 0.0% | 1.7% | 2.0 | 0 | 1 |
| `granite4.1:3b` | ollama | 86.3% | 96.4% | 11.1% | 6.7% | 1.9 | 0 | 0 |
| `granite4.1:8b` | llamacpp | 96.1% | 96.4% | 33.3% | 3.3% | 4.5 | 0 | 0 |
| `granite4.1:8b` | ollama | 98.0% | 98.2% | 33.3% | 1.7% | 5.5 | 0 | 0 |
| `lfm2.5:8b-a1b-q8_0` | llamacpp | 0.0% | 0.0% | 0.0% | 0.0% | 4.9 | 0 | 0 |
| `lfm2.5:8b-a1b-q8_0` | ollama | 94.1% | 94.6% | 11.1% | 1.7% | 4.3 | 0 | 6 |
| `qwen3:0.6b` | llamacpp | 76.5% | 83.9% | 33.3% | 3.3% | 3.8 | 0 | 0 |
| `qwen3:0.6b` | ollama | 56.9% | 64.3% | 66.7% | 8.3% | 2.8 | 0 | 0 |

Each row's graded summary is in this directory (`<model>_<server>/`), except `granite4.1:8b`, which is in [`../baseline/`](../baseline/) (Ollama, the control) and [`../baseline-llamacpp/`](../baseline-llamacpp/). Answers in the samples are cut to 60 characters, because full answers can quote published rules.

## How to read it

- **One run per cell.** Answer, tool, and hallucination are scored over 51 to 60 questions, so one question is about 2 points. Abstain is scored over only 9 questions, so one question is 11 points: differences there are not meaningful.
- **Read abstain with answer.** `qwen3:0.6b` on Ollama has the highest abstain rate (66.7%) and one of the lowest answer rates (56.9%). A model that often fails to use its tools declines a lot, which looks like good refusing and is not.
- **The servers do not always get the same model.** For every model except `gpt-oss:20b`, llama.cpp loaded the very file Ollama uses. For `gpt-oss:20b` it could not: Ollama's copy has an architecture name (`gptoss`) llama.cpp does not recognise, so llama.cpp ran the official `ggml-org/gpt-oss-20b-GGUF` file (MXFP4, Apache-2.0). Those two rows compare servers *and* files.

## What stands out

- **`lfm2.5` on llama.cpp scores 0%, and that is a serving failure, not the model.** It writes its tool call as text (`<|tool_call_start|>[get_unit(name='Angron')]<|tool_call_end|>`), and this llama-server build (b11476) did not turn that into a tool call, so no tool was ever used. Treat the row as "unsupported with this file and build".
- **`gpt-oss:20b` scores best on both servers** (94.1% on Ollama, 98.0% on llama.cpp) and has the best abstain rate of the models that also answer well (55.6% on Ollama, 44.4% on llama.cpp). On Ollama it had two errors, a malformed tool call the server rejected and a timeout. On llama.cpp it hit the step limit 4 times, all on questions it could not answer, where it kept searching instead of declining.
- **`granite4.1:8b` is within one question across servers** (98.0% and 96.1%).
- **The small models differ most between servers**: `qwen3:0.6b` gets 76.5% on llama.cpp and 56.9% on Ollama, and `granite4.1:3b` 90.2% and 86.3%. These are single runs and I did not look into why. A likely cause is that the two servers format the prompt differently (for example, whether a model "thinks" first), so do not read either as a ranking of the servers.
- **Speed is similar.** llama.cpp had the lower median for `granite4.1:8b` (4.5 s against 5.5 s) and `gpt-oss:20b` (4.6 s against 5.3 s), the same for `granite4.1:3b`, and was slower for `qwen3:0.6b` (3.8 s against 2.8 s).
- **Declining is the shared weak spot.** Apart from `gpt-oss:20b` and the `qwen3:0.6b` case above, every model declined 0 to 33% of the 9 refusal questions.
