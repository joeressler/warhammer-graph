# Using wh-mcp with llama.cpp

This is the same host as the [Ollama host](ollama-host.md), with llama.cpp's `llama-server` serving the model instead of Ollama:

```text
you  ->  host.py  ->  llama-server (the model)  <->  wh-mcp  ->  bundle
```

Why bother: llama.cpp is the engine under most local-model tooling, it can load almost any GGUF file, it exposes more server settings, and it is the program that will serve a model you fine-tune yourself. Everything above the model server is unchanged. The agent loop is the one in `examples/ollama-host/host.py`, and the chat function for llama-server is `examples/ollama-host/llamacpp.py`.

## Install llama.cpp

Download a release from <https://github.com/ggml-org/llama.cpp/releases> and unzip it. On a machine with an NVIDIA GPU, take the `win-cuda-13` zip for your CUDA version, plus the matching `cudart` zip extracted into the same folder. (On this machine: build `b11476`, in `C:\Users\joear\llama.cpp\b11476`, outside OneDrive.) Check it:

```bash
llama-server --version
```

The launcher script looks for `llama-server` in this order: `--llama-server`, the `LLAMA_SERVER` environment variable, `PATH`, then the newest `~/llama.cpp/*/llama-server`.

## Start the server

If Ollama already has the model, serve that exact file. Ollama keeps each model as a plain GGUF file, and llama-server can load it without copying it:

```bash
python examples/ollama-host/serve_llamacpp.py --ollama-model granite4.1:8b
```

Or use any GGUF file:

```bash
python examples/ollama-host/serve_llamacpp.py --gguf path/to/model.gguf --alias my-model
```

The script prints the command it runs. By hand, that is:

```bash
llama-server -m model.gguf --jinja -c 16384 -ngl 99 --port 8080 --alias granite4.1:8b
```

| Flag | Why |
| --- | --- |
| `--jinja` | **Required.** It makes the server use the model's own chat template, which is how tool calls are formatted. Without it, tool calling does not work. |
| `-c 16384` | The context window. Ollama sets this per request, but llama-server fixes it at start-up, so the host's `--num-ctx` does nothing here. 16384 matches the Ollama runs. |
| `-ngl 99` | Put all layers on the GPU. |
| `--alias` | The name the server reports. The host only uses it as a label. |

Leave it running in its own terminal. `curl localhost:8080/health` returns `{"status":"ok"}` when it is ready. A model on the GPU needs room next to anything Ollama still has loaded, so stop one before loading a big model in the other.

## Ask a question

```bash
python examples/ollama-host/host.py --backend llamacpp --trace --ask "What invulnerable save does Angron have?"
```

`--base-url` points at a server that is not on `localhost:8080`. `evaluate.py` takes the same two flags.

## Run the evals

```bash
python -m evals.run --backend llamacpp --model granite4.1:8b
pytest evals -m eval --eval-backend llamacpp --eval-model granite4.1:8b -s
```

The run id gets `_llamacpp` added, and `run.json` records the backend and what the server reports about itself: the build, the context size, and the model file.

## How it differs from Ollama

`llamacpp.py` translates between the host's messages, which are in Ollama's shape, and the OpenAI chat format that llama-server speaks:

- An assistant tool call needs `"type": "function"` and its arguments as a JSON string. Ollama takes a dict. Without the type, llama-server answers HTTP 500.
- A tool result must carry the `tool_call_id` of the call it answers, where Ollama names the tool. The host's messages have no ids, so they are made up from each message's position. They are the same on every request, which keeps the prompt identical from turn to turn so the server can reuse what it has already processed.
- A reply's arguments arrive as a string and go back out as a dict. Arguments that are not valid JSON become an empty dict, so the tool's own "missing field" error is what the model sees and can correct.
- A server error (HTTP 4xx or 5xx) is a failed question. A server that cannot be reached ends an eval run and keeps the partial results.

## Same model, both servers

The same `granite4.1:8b` weights, the same 60 golden questions, temperature 0, and the host's default settings. Run on 2026-10-07 on an RTX 5000 Ada laptop GPU (16 GB):

| Metric | Ollama | llama.cpp (b11476) |
| --- | --- | --- |
| Answer exact match | 98.0% (50/51) | 96.1% (49/51) |
| Tool correct | 98.2% (55/56) | 96.4% (54/56) |
| Abstain correct (refuse) | 33.3% (3/9) | 33.3% (3/9) |
| Hallucination rate | 1.7% (1/60) | 3.3% (2/60) |
| Median seconds per question | 5.5 | 4.5 |

What to take from it:

- **The two servers are equivalent for this work.** The 2-point gap on answers is one question out of 51, which is noise for a single run at temperature 0. A change that small is not evidence either way.
- **The same weakness shows on both: refusing.** Both answered 3 of 9 refusals correctly. On llama.cpp the "price in dollars" question got an invented figure ("$34–$35 USD") as well as the points cost, which is a real hallucination.
- **Three rows differ**: a detachment count where the tool check failed (the agent searched the wrong faction first, then still answered correctly), a count where it settled on the wrong detachment and answered 4 instead of 2, and the invented price. None of these points at the server. Small differences in how the prompt is processed can send a model down a different path.
- **llama.cpp was a little faster**, about one second per question at the median.
- **Compare like with like.** The Ollama baseline stays the control for the evals. A model served by llama.cpp should be compared with the llama.cpp row, with the same build, context size, and quantization. The samples are in [`evals/baseline/`](../../evals/baseline/) and [`evals/baseline-llamacpp/`](../../evals/baseline-llamacpp/).

## Tests

```bash
python -m pytest examples/ollama-host/tests/test_llamacpp.py   # offline: a fake llama-server and the real wh-mcp
```

They cover the message conversion and ids, reply parsing, server errors, `make_chat`, the launcher script, and a full tool round trip through the real host and server. Nothing in them needs llama.cpp installed.
