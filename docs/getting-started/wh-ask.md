# Getting started with wh-ask

`wh-ask` answers questions from a graph bundle built by [wh-graph](wh-graph.md). It is a local desktop window and a headless `query` command. Both call the same retrieval path. The app does not download Wahapedia, does not parse CSV, and does not download GGUF models. After the bundle and the model files are on disk, a query reads only the local filesystem.

The frozen contract is [03-rust-frontend.md](../specs/03-rust-frontend.md). This page is the install and command guide.

## Install

You need Rust 1.83 or newer, plus a C++ toolchain, CMake, and libclang. `wh-ask` links `llama-cpp-2` 0.1.102 with `llama-cpp-sys-2` 0.1.100. That crate compiles the llama.cpp sources it vendors. The first `cargo build` of the desktop feature is the slow step.

On Debian or Ubuntu, install the compilers and the libraries the window needs:

```bash
sudo apt-get install \
  build-essential cmake clang libclang-dev pkg-config \
  libssl-dev \
  libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev \
  libxkbcommon-dev libxkbcommon-x11-dev libwayland-dev
```

The X11 and Wayland packages come from the eframe 0.31 Linux setup. The window uses eframe's default glow renderer.

On macOS, install the Xcode command line tools and CMake (`xcode-select --install` and `brew install cmake`). On Windows, install Visual Studio Build Tools with the C++ workload, CMake, and LLVM so bindgen can find libclang.

From the repository root:

```bash
cargo build -p wh-ask
cargo run -p wh-ask -- --help
```

The binary is `target/debug/wh-ask`. Add `--release` when you want `target/release/wh-ask` for real models. The `desktop` feature is on by default. It pulls in eframe and llama.cpp. Library tests can skip that native build:

```bash
cargo test -p wh-ask --no-default-features
```

## Models

Put GGUF files somewhere on disk, for example `./models/`. `wh-ask` loads whatever paths you pass.

- `--chat-model` is required for `query` and for a useful window session. It is a chat GGUF.
- `--embed-model` is optional. When you pass it, retrieval embeds every passage and ranks them by similarity. When you omit it, retrieval is label search over passage titles and text.
- If you pass an embedding path and that file fails to load, the command exits 4. Label search is not a fallback in that case.

The app never downloads a model. Any GGUF that this llama.cpp build can load is acceptable. The graph builder does not bake vectors, because a vector belongs to one embedding model.

## Query from the terminal

```bash
cargo run -p wh-ask -- query \
  --bundle ./bundle \
  --chat-model ./models/chat.gguf \
  --question "Which example units are in Example Faction?"
```

With embeddings, and eight passages (the default):

```bash
cargo run -p wh-ask -- query \
  --bundle ./bundle \
  --chat-model ./models/chat.gguf \
  --embed-model ./models/embed.gguf \
  --question "Which example units are in Example Faction?" \
  --top-k 8
```

`--top-k` is an integer from 1 to 32 and defaults to 8. `--output text` prints the answer, then a `Citations:` block with `node_id`, title, and link on each line. `--output json` prints one pretty-printed object:

```json
{
  "answer": "...",
  "retrieval": "embedding",
  "citations": [
    {
      "node_id": "10ed:datasheet:EXDS",
      "title": "Example Unit",
      "wahapedia_link": "https://example.invalid/example-unit",
      "neighbors": ["Example", "Example Faction"]
    }
  ]
}
```

`retrieval` is `embedding` or `label`. A citation with no Wahapedia page has `wahapedia_link` set to `null`. Citations are node ids the model used from the prompt. If the model cites nothing, the seed passages are listed instead. Each citation includes up to eight neighbor labels from the graph.

The first query that uses an embedding model builds an index of every passage. A later query with the same bundle bytes and the same model file reuses it. The index lives under the config directory, not inside the bundle:

```text
indexes/{bundle passages sha256}/{model sha256}/index.bin
```

That directory is inside the `wh-ask` config folder described below.

The prompt tells the model to answer only from the retrieved passages and to cite them as `[node_id]`. Generation uses temperature 0.8, which is llama.cpp's default when the caller does not set one. `query` reserves 1024 tokens for the answer.

## Open the window

```bash
cargo run -p wh-ask
```

Flags prefill the settings fields. They do not start a query:

```bash
cargo run -p wh-ask -- \
  --bundle ./bundle \
  --chat-model ./models/chat.gguf \
  --embed-model ./models/embed.gguf
```

The window, from top to bottom:

1. Bundle path, chat model path, embedding model path, and top-k. An empty embedding path is valid and shows `Label search`.
2. Question box and Submit. Submit stays disabled while a query runs.
3. Answer text, or the error string if the bundle or a GGUF cannot be loaded.
4. Citations. Select one to see that node's neighbor labels.
5. Footer: `powered by Wahapedia`.

A bad path stays on screen. The process keeps running. The window exits with a failure code only when it cannot create a window at all.

Settings are written to `settings.json` in the platform config directory:

| Platform | Directory |
| --- | --- |
| Linux | `$XDG_CONFIG_HOME/wh-ask` or `~/.config/wh-ask` |
| macOS | `~/Library/Application Support/wh-ask` |
| Windows | `%APPDATA%\wh-ask` |

The file stores the bundle directory, chat GGUF path, optional embedding GGUF path, `top_k` (default 8), and `context_reserve` (default 1024). The window edits the paths and top-k. Paths are stored absolute after you enter them. `query` takes its paths and `--top-k` from the command line. It uses the config directory for the embedding index.

## Exit codes for query

| Code | When |
| --- | --- |
| 0 | An answer was produced. |
| 2 | A required flag is missing, `--output` is not `text` or `json`, or `--top-k` is outside 1..32. Stderr includes one working command. |
| 3 | The bundle is unreadable or `format_version` is not 1. |
| 4 | The chat GGUF failed to load, or an embedding GGUF was passed and failed to load. |

`query` requires `--bundle`, `--chat-model`, and `--question`.

## Tests

```bash
cargo test -p wh-ask --no-default-features
```

Those tests use a synthetic bundle, a fake embedder, and a fake chat model. They do not load a GGUF and they do not open a network socket. `cargo test -p wh-ask` runs the same tests and also compiles the desktop feature.
