use crate::bundle::Bundle;
use crate::names::{fold, tokens};
use crate::types::SearchHit;

const MAX_SEARCH: usize = 100;

const EXACT: u32 = 1000;
const CONTAINS: u32 = 500;
const TITLE_WORD: u32 = 10;
const TEXT_WORD: u32 = 1;

struct Scored {
    score: u32,
    /// The whole phrase is the title, or sits inside it, or every word appears somewhere.
    strong: bool,
    title_len: usize,
    index: usize,
}

impl Bundle {
    /// Label search over passage titles and text. `kinds` limits the node kinds
    /// searched (empty means all). Results are best first and capped at 100.
    ///
    /// An exact title ranks first, then a title containing the phrase, then
    /// passages with every word. When nothing has every word, passages with
    /// any word are returned instead.
    pub fn search(&self, query: &str, kinds: &[&str], limit: usize) -> Vec<SearchHit> {
        let limit = limit.clamp(1, MAX_SEARCH);
        let mut words = tokens(&fold(query.trim()));
        if words.is_empty() {
            return Vec::new();
        }
        let phrase = words.join(" ");
        words.sort();
        words.dedup();

        let scan: Box<dyn Iterator<Item = usize> + '_> = if kinds.is_empty() {
            Box::new(0..self.passages.len())
        } else {
            Box::new(
                kinds
                    .iter()
                    .flat_map(|kind| self.passage_indexes(kind).iter().copied()),
            )
        };

        let mut scored = Vec::new();
        for index in scan {
            let folded = &self.folded[index];
            let exact = folded.key == phrase;
            let contains = !exact && folded.key.contains(&phrase);
            let mut title_hits = 0u32;
            let mut text_hits = 0u32;
            let mut covered = 0usize;
            for word in &words {
                let in_title = folded.title.contains(word.as_str());
                let in_text = folded.text.contains(word.as_str());
                title_hits += u32::from(in_title);
                text_hits += u32::from(in_text);
                covered += usize::from(in_title || in_text);
            }
            if !exact && !contains && covered == 0 {
                continue;
            }
            let score = if exact { EXACT } else { 0 }
                + if contains { CONTAINS } else { 0 }
                + title_hits * TITLE_WORD
                + text_hits * TEXT_WORD;
            scored.push(Scored {
                score,
                strong: exact || contains || covered == words.len(),
                title_len: folded.title.chars().count(),
                index,
            });
        }
        if scored.iter().any(|item| item.strong) {
            scored.retain(|item| item.strong);
        }
        scored.sort_by(|left, right| {
            right
                .score
                .cmp(&left.score)
                .then(left.title_len.cmp(&right.title_len))
                .then(left.index.cmp(&right.index))
        });
        scored.truncate(limit);
        scored
            .into_iter()
            .filter_map(|item| {
                let node = self.node(&self.passages[item.index].node_id)?;
                Some(SearchHit {
                    node: self.node_ref(node),
                    score: item.score,
                })
            })
            .collect()
    }
}
