use std::num::NonZeroU32;
use std::path::Path;

use llama_cpp_2::context::params::LlamaContextParams;
use llama_cpp_2::llama_backend::LlamaBackend;
use llama_cpp_2::llama_batch::LlamaBatch;
use llama_cpp_2::model::params::LlamaModelParams;
use llama_cpp_2::model::{AddBos, LlamaModel, Special};
use llama_cpp_2::sampling::LlamaSampler;
use llama_cpp_2::token::LlamaToken;
use wh_ask::{
    load_bundle, model_identity, run_query, AskError, ChatModel, Embedder, QueryRequest,
    QueryResponse, DEFAULT_RESERVE,
};

/// llama.cpp's common sampler uses temperature 0.8 when the caller does not set one.
const DEFAULT_TEMPERATURE: f32 = 0.8;

struct Runtime {
    chat: LlamaModel,
    embed: Option<LlamaModel>,
    // Dropped after the models so llama_backend_free runs last.
    backend: LlamaBackend,
}

pub fn answer_with_files(
    bundle_dir: &Path,
    chat_path: &Path,
    embed_path: Option<&Path>,
    question: &str,
    top_k: usize,
    context_reserve: usize,
    config_dir: &Path,
) -> Result<QueryResponse, AskError> {
    // The bundle is checked before any GGUF so a bad format_version exits 3
    // even when the model path cannot be loaded.
    let _bundle = load_bundle(bundle_dir)?;
    if !chat_path.is_file() {
        return Err(AskError::model(format!(
            "{}: chat GGUF failed to load",
            chat_path.display()
        )));
    }
    if let Some(path) = embed_path {
        if !path.is_file() {
            return Err(AskError::model(format!(
                "{}: embedding GGUF failed to load",
                path.display()
            )));
        }
    }
    let identity = match embed_path {
        Some(path) => Some(model_identity(config_dir, bundle_dir, path)?),
        None => None,
    };
    let runtime = Runtime::load(chat_path, embed_path)?;
    let mut chat = LlamaChat {
        backend: &runtime.backend,
        model: &runtime.chat,
    };
    let mut embed_model = runtime.embed.as_ref().map(|model| LlamaEmbed {
        backend: &runtime.backend,
        model,
    });
    let request = QueryRequest {
        bundle_dir,
        question,
        top_k,
        context_reserve,
        config_dir,
    };
    let embed = match (identity.as_ref(), embed_model.as_mut()) {
        (Some(identity), Some(embedder)) => Some((identity, embedder as &mut dyn Embedder)),
        _ => None,
    };
    run_query(&request, &mut chat, embed)
}

pub fn query_reserve() -> usize {
    DEFAULT_RESERVE as usize
}

impl Runtime {
    fn load(chat_path: &Path, embed_path: Option<&Path>) -> Result<Self, AskError> {
        let backend = LlamaBackend::init()
            .map_err(|err| AskError::model(format!("llama backend failed to initialize: {err}")))?;
        let params = LlamaModelParams::default();
        let chat = LlamaModel::load_from_file(&backend, chat_path, &params).map_err(|err| {
            AskError::model(format!(
                "{}: chat GGUF failed to load: {err}",
                chat_path.display()
            ))
        })?;
        let embed = match embed_path {
            Some(path) => Some(LlamaModel::load_from_file(&backend, path, &params).map_err(
                |err| {
                    AskError::model(format!(
                        "{}: embedding GGUF failed to load: {err}",
                        path.display()
                    ))
                },
            )?),
            None => None,
        };
        Ok(Runtime {
            chat,
            embed,
            backend,
        })
    }
}

struct LlamaChat<'a> {
    backend: &'a LlamaBackend,
    model: &'a LlamaModel,
}

struct LlamaEmbed<'a> {
    backend: &'a LlamaBackend,
    model: &'a LlamaModel,
}

impl ChatModel for LlamaChat<'_> {
    fn context_tokens(&self) -> usize {
        self.model.n_ctx_train().max(512) as usize
    }

    fn count_tokens(&mut self, text: &str) -> Result<usize, AskError> {
        Ok(tokenize(self.model, text)?.len())
    }

    fn complete(&mut self, prompt: &str) -> Result<String, AskError> {
        let tokens = tokenize(self.model, prompt)?;
        let n_ctx = self.model.n_ctx_train().max(512);
        if tokens.len() >= n_ctx as usize {
            return Err(AskError::model("prompt does not fit the chat context"));
        }
        let params = LlamaContextParams::default()
            .with_n_ctx(NonZeroU32::new(n_ctx))
            .with_n_batch(n_ctx);
        let mut ctx = self
            .model
            .new_context(self.backend, params)
            .map_err(|err| AskError::model(format!("chat context failed to load: {err}")))?;
        let mut batch = LlamaBatch::new(n_ctx as usize, 1);
        batch
            .add_sequence(&tokens, 0, false)
            .map_err(|err| AskError::model(format!("chat batch failed: {err}")))?;
        ctx.decode(&mut batch)
            .map_err(|err| AskError::model(format!("chat decode failed: {err}")))?;

        let mut sampler = LlamaSampler::chain_simple([
            LlamaSampler::temp(DEFAULT_TEMPERATURE),
            LlamaSampler::dist(u32::MAX),
        ]);
        let mut output = Vec::new();
        let mut n_cur = tokens.len();
        while n_cur < n_ctx as usize {
            let token = sampler.sample(&ctx, -1);
            sampler.accept(token);
            if self.model.is_eog_token(token) {
                break;
            }
            output.extend(
                self.model
                    .token_to_bytes(token, Special::Plaintext)
                    .map_err(|err| AskError::model(format!("token decode failed: {err}")))?,
            );
            batch.clear();
            let pos = i32::try_from(n_cur)
                .map_err(|_| AskError::model("chat context position overflowed"))?;
            batch
                .add(token, pos, &[0], true)
                .map_err(|err| AskError::model(format!("chat batch failed: {err}")))?;
            ctx.decode(&mut batch)
                .map_err(|err| AskError::model(format!("chat decode failed: {err}")))?;
            n_cur += 1;
        }
        Ok(String::from_utf8_lossy(&output).into_owned())
    }
}

impl Embedder for LlamaEmbed<'_> {
    fn embed_passage(&mut self, _node_id: &str, text: &str) -> Result<Vec<f32>, AskError> {
        embed_text(self.backend, self.model, text)
    }

    fn embed_query(&mut self, question: &str) -> Result<Vec<f32>, AskError> {
        embed_text(self.backend, self.model, question)
    }

    fn dimension(&self) -> u32 {
        u32::try_from(self.model.n_embd()).unwrap_or(0)
    }
}

fn tokenize(model: &LlamaModel, text: &str) -> Result<Vec<LlamaToken>, AskError> {
    model
        .str_to_token(text, AddBos::Always)
        .map_err(|err| AskError::model(format!("tokenize failed: {err}")))
}

fn embed_text(
    backend: &LlamaBackend,
    model: &LlamaModel,
    text: &str,
) -> Result<Vec<f32>, AskError> {
    let dimension = usize::try_from(model.n_embd())
        .map_err(|_| AskError::model("embedding model reported no dimensions"))?;
    if dimension == 0 {
        return Err(AskError::model("embedding model reported no dimensions"));
    }
    let tokens = tokenize(model, text)?;
    if tokens.is_empty() {
        return Ok(vec![0.0; dimension]);
    }
    let n_ctx = model.n_ctx_train().max(512);
    if tokens.len() > n_ctx as usize {
        return Err(AskError::model(
            "passage does not fit the embedding context",
        ));
    }
    let params = LlamaContextParams::default()
        .with_embeddings(true)
        .with_n_ctx(NonZeroU32::new(n_ctx))
        .with_n_batch(n_ctx);
    let mut ctx = model
        .new_context(backend, params)
        .map_err(|err| AskError::model(format!("embedding context failed to load: {err}")))?;
    let mut batch = LlamaBatch::new(tokens.len().max(1), 1);
    batch
        .add_sequence(&tokens, 0, true)
        .map_err(|err| AskError::model(format!("embedding batch failed: {err}")))?;
    ctx.decode(&mut batch)
        .map_err(|err| AskError::model(format!("embedding decode failed: {err}")))?;
    if let Ok(values) = ctx.embeddings_seq_ith(0) {
        return Ok(values.to_vec());
    }
    let mut sum = vec![0.0f32; dimension];
    let mut counted = 0usize;
    for index in 0..tokens.len() as i32 {
        if let Ok(values) = ctx.embeddings_ith(index) {
            for (total, value) in sum.iter_mut().zip(values) {
                *total += value;
            }
            counted += 1;
        }
    }
    if counted == 0 {
        return Err(AskError::model("embedding model returned no vector"));
    }
    for value in &mut sum {
        *value /= counted as f32;
    }
    Ok(sum)
}
