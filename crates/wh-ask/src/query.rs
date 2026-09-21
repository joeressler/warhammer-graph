use std::path::Path;

use serde::Serialize;

use crate::bundle::load_bundle;
use crate::cite::{citations, Citation};
use crate::error::AskError;
use crate::expand::expand;
use crate::index::load_or_build;
use crate::model::{ChatModel, EmbedIdentity, Embedder};
use crate::prompt::build;
use crate::retrieve::{embed_rank, label_rank};
use crate::QUERY_EXAMPLE;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Retrieval {
    Embedding,
    Label,
}

impl Retrieval {
    pub fn as_str(self) -> &'static str {
        match self {
            Retrieval::Embedding => "embedding",
            Retrieval::Label => "label",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct QueryResponse {
    pub answer: String,
    pub retrieval: Retrieval,
    pub citations: Vec<Citation>,
}

pub struct QueryRequest<'a> {
    pub bundle_dir: &'a Path,
    pub question: &'a str,
    pub top_k: usize,
    pub context_reserve: usize,
    pub config_dir: &'a Path,
}

pub fn run_query(
    request: &QueryRequest<'_>,
    chat: &mut dyn ChatModel,
    embed: Option<(&EmbedIdentity, &mut dyn Embedder)>,
) -> Result<QueryResponse, AskError> {
    if !(1..=32).contains(&request.top_k) {
        return Err(AskError::usage(format!(
            "--top-k must be from 1 to 32\n{QUERY_EXAMPLE}"
        )));
    }
    let bundle = load_bundle(request.bundle_dir)?;
    let (seeds, retrieval) = if let Some((identity, embedder)) = embed {
        let vectors = load_or_build(request.config_dir, &bundle, identity, embedder)?;
        let query = embedder.embed_query(request.question)?;
        (
            embed_rank(&vectors, &query, request.top_k),
            Retrieval::Embedding,
        )
    } else {
        (
            label_rank(&bundle, request.question, request.top_k),
            Retrieval::Label,
        )
    };
    let context = expand(&bundle, &seeds);
    let prompt = build(
        &bundle,
        &context,
        request.question,
        request.context_reserve,
        chat,
    )?;
    let answer = chat.complete(&prompt)?;
    let citations = citations(&bundle, &answer, &context);
    Ok(QueryResponse {
        answer,
        retrieval,
        citations,
    })
}

pub fn render_output(response: &QueryResponse, output: &str) -> Result<String, AskError> {
    match output {
        "text" => Ok(render_text(response)),
        "json" => {
            #[derive(Serialize)]
            struct Json<'a> {
                answer: &'a str,
                retrieval: &'a str,
                citations: &'a [Citation],
            }
            let payload = Json {
                answer: &response.answer,
                retrieval: response.retrieval.as_str(),
                citations: &response.citations,
            };
            let mut text = serde_json::to_string_pretty(&payload)
                .map_err(|err| AskError::model(format!("could not render JSON output: {err}")))?;
            text.push('\n');
            Ok(text)
        }
        _ => Err(AskError::usage(format!(
            "bad flag: --output\n{QUERY_EXAMPLE}"
        ))),
    }
}

fn render_text(response: &QueryResponse) -> String {
    let mut text = response.answer.clone();
    if !text.ends_with('\n') {
        text.push('\n');
    }
    text.push_str("Citations:\n");
    for citation in &response.citations {
        let link = citation.wahapedia_link.as_deref().unwrap_or("");
        text.push_str(&citation.node_id);
        text.push('\t');
        text.push_str(&citation.title);
        text.push('\t');
        text.push_str(link);
        text.push('\n');
    }
    text
}
