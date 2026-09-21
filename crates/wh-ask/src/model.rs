use crate::error::AskError;

/// Identity of the embedding GGUF cached in an index header.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EmbedIdentity {
    pub sha256: [u8; 32],
    pub len: u64,
    pub mtime_secs: u64,
}

/// Passage and question vectors. Tests supply a double; the app uses a GGUF.
pub trait Embedder {
    fn embed_passage(&mut self, node_id: &str, text: &str) -> Result<Vec<f32>, AskError>;
    fn embed_query(&mut self, question: &str) -> Result<Vec<f32>, AskError>;
    fn dimension(&self) -> u32;
}

/// Chat completion and the tokenizer used for the context budget.
pub trait ChatModel {
    fn context_tokens(&self) -> usize;
    fn count_tokens(&mut self, text: &str) -> Result<usize, AskError>;
    fn complete(&mut self, prompt: &str) -> Result<String, AskError>;
}

pub(crate) fn l2_normalize(values: &mut [f32]) {
    let norm = values.iter().map(|value| value * value).sum::<f32>().sqrt();
    if norm > 0.0 {
        for value in values {
            *value /= norm;
        }
    }
}
