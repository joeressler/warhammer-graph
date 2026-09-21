use crate::bundle::LoadedBundle;
use crate::model::l2_normalize;

#[derive(Clone, Copy)]
struct LabelScore {
    exact: bool,
    tokens: u32,
}

/// Rank passage indexes. Label search drops scores that are not above zero.
pub fn label_rank(bundle: &LoadedBundle, question: &str, top_k: usize) -> Vec<usize> {
    let folded = caseless::default_case_fold_str(question.trim());
    let tokens = folded
        .split_whitespace()
        .filter(|token| token.chars().count() >= 2)
        .map(str::to_string)
        .collect::<Vec<_>>();
    let mut ranked = Vec::new();
    for (index, passage) in bundle.passages.iter().enumerate() {
        let title = caseless::default_case_fold_str(&passage.title);
        let text = caseless::default_case_fold_str(&passage.text);
        let exact = !folded.is_empty() && title.contains(&folded);
        let mut hits = 0u32;
        for token in &tokens {
            if title.contains(token) {
                hits += 1;
            }
            if text.contains(token) {
                hits += 1;
            }
        }
        if exact || hits > 0 {
            ranked.push((
                index,
                LabelScore {
                    exact,
                    tokens: hits,
                },
            ));
        }
    }
    ranked.sort_by(|left, right| {
        right
            .1
            .exact
            .cmp(&left.1.exact)
            .then(right.1.tokens.cmp(&left.1.tokens))
            .then(left.0.cmp(&right.0))
    });
    ranked.truncate(top_k);
    ranked.into_iter().map(|(index, _)| index).collect()
}

/// Highest dot product first. Ties keep `passages.jsonl` order.
pub fn embed_rank(vectors: &[Vec<f32>], query: &[f32], top_k: usize) -> Vec<usize> {
    let mut query = query.to_vec();
    l2_normalize(&mut query);
    let mut ranked = vectors
        .iter()
        .enumerate()
        .map(|(index, vector)| {
            let score = vector
                .iter()
                .zip(query.iter())
                .map(|(left, right)| left * right)
                .sum::<f32>();
            (index, score)
        })
        .collect::<Vec<_>>();
    ranked.sort_by(|left, right| {
        right
            .1
            .partial_cmp(&left.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(left.0.cmp(&right.0))
    });
    ranked.truncate(top_k);
    ranked.into_iter().map(|(index, _)| index).collect()
}
