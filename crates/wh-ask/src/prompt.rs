use std::collections::HashSet;

use crate::bundle::LoadedBundle;
use crate::error::AskError;
use crate::expand::ContextNodes;
use crate::model::ChatModel;

const INSTRUCTION: &str = "\
You answer questions about Warhammer 40,000 using only the passages below.
When you use a passage, cite it as [node_id]. If that passage has a link, write the link on the same line as the citation.
If the passages do not contain the answer, say that they do not.";

struct Piece {
    node_id: String,
    title: String,
    link: Option<String>,
    text: String,
}

/// Build the only prompt the chat model sees, shrinking expanded nodes and
/// then seed text until it fits in the context minus the reserved answer tokens.
pub fn build(
    bundle: &LoadedBundle,
    context: &ContextNodes,
    question: &str,
    reserve: usize,
    chat: &mut dyn ChatModel,
) -> Result<String, AskError> {
    let budget = chat.context_tokens().saturating_sub(reserve);
    let mut pieces = context
        .nodes
        .iter()
        .map(|id| piece(bundle, id))
        .collect::<Vec<_>>();
    let seed_count = context.seeds.len().min(pieces.len());
    let originals = pieces
        .iter()
        .map(|piece| piece.text.clone())
        .collect::<Vec<_>>();

    let mut keep = pieces.len();
    while keep > seed_count {
        let prompt = render(
            &pieces[..keep],
            bundle,
            question,
            context.seeds.is_empty() && keep == 0,
        );
        if chat.count_tokens(&prompt)? <= budget {
            return Ok(prompt);
        }
        keep -= 1;
    }

    let prompt = render(&pieces[..keep], bundle, question, context.seeds.is_empty());
    if chat.count_tokens(&prompt)? <= budget {
        return Ok(prompt);
    }

    for index in (0..seed_count).rev() {
        let mut low = 0usize;
        let mut high = originals[index].chars().count();
        while low < high {
            let mid = (low + high + 1) / 2;
            pieces[index].text = char_prefix(&originals[index], mid);
            let prompt = render(&pieces[..keep], bundle, question, false);
            if chat.count_tokens(&prompt)? <= budget {
                low = mid;
            } else {
                high = mid - 1;
            }
        }
        pieces[index].text = char_prefix(&originals[index], low);
        let prompt = render(&pieces[..keep], bundle, question, false);
        if chat.count_tokens(&prompt)? <= budget {
            return Ok(prompt);
        }
    }
    Ok(render(
        &pieces[..keep],
        bundle,
        question,
        context.seeds.is_empty(),
    ))
}

fn piece(bundle: &LoadedBundle, id: &str) -> Piece {
    if let Some(passage) = bundle
        .passages
        .iter()
        .find(|passage| passage.node_id == *id)
    {
        return Piece {
            node_id: id.to_string(),
            title: passage.title.clone(),
            link: passage.wahapedia_link.clone(),
            text: passage.text.clone(),
        };
    }
    let node = bundle.node(id);
    Piece {
        node_id: id.to_string(),
        title: node.map(|node| node.label.clone()).unwrap_or_default(),
        link: node.and_then(|node| node.source_url.clone()),
        text: node.map(|node| node.text.clone()).unwrap_or_default(),
    }
}

fn render(pieces: &[Piece], bundle: &LoadedBundle, question: &str, no_match: bool) -> String {
    let mut prompt = String::new();
    prompt.push_str(INSTRUCTION);
    prompt.push_str("\n\nEdges:\n");
    let ids = pieces
        .iter()
        .map(|piece| piece.node_id.as_str())
        .collect::<HashSet<_>>();
    for edge in &bundle.edges {
        if ids.contains(edge.from.as_str()) && ids.contains(edge.to.as_str()) {
            prompt.push_str(&edge_line(bundle, edge));
            prompt.push('\n');
        }
    }
    prompt.push_str("\nPassages:\n");
    if no_match || pieces.is_empty() {
        prompt.push_str("The bundle had no matching passage.\n");
    } else {
        for piece in pieces {
            prompt.push_str("---\n");
            prompt.push_str("title: ");
            prompt.push_str(&piece.title);
            prompt.push_str("\nnode_id: ");
            prompt.push_str(&piece.node_id);
            prompt.push_str("\nlink: ");
            prompt.push_str(piece.link.as_deref().unwrap_or("none"));
            prompt.push('\n');
            prompt.push_str(&piece.text);
            if !piece.text.ends_with('\n') {
                prompt.push('\n');
            }
            prompt.push_str("---\n");
        }
    }
    prompt.push_str("\nQuestion: ");
    prompt.push_str(question);
    prompt.push('\n');
    prompt
}

fn edge_line(bundle: &LoadedBundle, edge: &wh_graph::EdgeRecord) -> String {
    let from = label(bundle, &edge.from);
    let to = label(bundle, &edge.to);
    let mut line = format!("{from} -{}-> {to}", edge.kind);
    let attrs = serde_json::to_string(&edge.attrs).unwrap_or_else(|_| "{}".to_string());
    if attrs != "{}" {
        line.push(' ');
        line.push_str(&attrs);
    }
    line
}

fn label(bundle: &LoadedBundle, id: &str) -> String {
    bundle
        .node(id)
        .map(|node| node.label.clone())
        .unwrap_or_else(|| id.to_string())
}

fn char_prefix(text: &str, chars: usize) -> String {
    text.chars().take(chars).collect()
}
