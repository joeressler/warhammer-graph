//! Name folding shared by search, name lookup, and rosters.

/// Case-fold and unify the apostrophe variants Wahapedia mixes.
pub(crate) fn fold(value: &str) -> String {
    caseless::default_case_fold_str(value).replace(['\u{2019}', '\u{2018}', '`'], "'")
}

/// `a`, `an`, and `the` may be missing from a name, so "Imotekh Stormlord"
/// still finds "Imotekh The Stormlord" instead of the shorter "Stormlord" sheet.
pub(crate) fn strip_articles(value: &str) -> String {
    value
        .split_whitespace()
        .filter(|word| !is_article(word))
        .collect::<Vec<_>>()
        .join(" ")
}

fn is_article(word: &str) -> bool {
    matches!(
        word.trim_matches(|c: char| !c.is_alphanumeric()),
        "a" | "an" | "the"
    )
}

/// Words of a folded name: articles dropped, edge punctuation trimmed, and
/// single characters (such as a lone dash) dropped.
pub(crate) fn tokens(value: &str) -> Vec<String> {
    strip_articles(value)
        .split_whitespace()
        .map(|word| {
            word.trim_matches(|c: char| !c.is_alphanumeric() && c != '\'')
                .to_string()
        })
        .filter(|word| word.chars().count() >= 2)
        .collect()
}

/// The identity of a label for exact lookup.
pub(crate) fn name_key(value: &str) -> String {
    tokens(&fold(value)).join(" ")
}

/// How many single-character edits turn `a` into `b`: an insertion, a deletion,
/// a substitution, or a swap of two neighboring characters. `taxtical` is one
/// edit from `tactical`, and `teh` is one from `the`.
pub(crate) fn edit_distance(a: &str, b: &str) -> usize {
    let a = a.chars().collect::<Vec<_>>();
    let b = b.chars().collect::<Vec<_>>();
    let (n, m) = (a.len(), b.len());
    if n == 0 || m == 0 {
        return n.max(m);
    }
    let mut older = vec![0usize; m + 1];
    let mut previous = (0..=m).collect::<Vec<_>>();
    let mut current = vec![0usize; m + 1];
    for i in 1..=n {
        current[0] = i;
        for j in 1..=m {
            let substitution = usize::from(a[i - 1] != b[j - 1]);
            let mut best = (previous[j] + 1)
                .min(current[j - 1] + 1)
                .min(previous[j - 1] + substitution);
            if i > 1 && j > 1 && a[i - 1] == b[j - 2] && a[i - 2] == b[j - 1] {
                best = best.min(older[j - 2] + 1);
            }
            current[j] = best;
        }
        std::mem::swap(&mut older, &mut previous);
        std::mem::swap(&mut previous, &mut current);
    }
    previous[m]
}

/// Whether the whole label occurs inside the (already folded) haystack.
pub(crate) fn label_in(haystack: &str, label: &str) -> bool {
    let needle = strip_articles(label);
    needle.chars().count() >= 2 && haystack.contains(&needle)
}

#[cfg(test)]
mod tests {
    use super::{edit_distance, fold, label_in, name_key, strip_articles};

    #[test]
    fn articles_in_a_name_are_optional() {
        assert_eq!(
            name_key("Imotekh Stormlord"),
            name_key("Imotekh The Stormlord")
        );
        let omitted = strip_articles(&fold("What units can Imotekh Stormlord lead"));
        assert!(label_in(&omitted, "imotekh the stormlord"));
        let tank = strip_articles(&fold("What units can Stormlord lead"));
        assert!(!label_in(&tank, "imotekh the stormlord"));
        assert!(label_in(&tank, "stormlord"));
    }

    #[test]
    fn edit_distance_counts_insertions_deletions_substitutions_and_swaps() {
        assert_eq!(edit_distance("tactical squad", "tactical squad"), 0);
        assert_eq!(edit_distance("taxtical squad", "tactical squad"), 1, "one substitution");
        assert_eq!(edit_distance("tacical squad", "tactical squad"), 1, "one missing letter");
        assert_eq!(edit_distance("tacttical squad", "tactical squad"), 1, "one extra letter");
        assert_eq!(edit_distance("teh", "the"), 1, "a swap is one edit, not two");
        assert_eq!(edit_distance("kitten", "sitting"), 3);
        assert_eq!(edit_distance("", "abc"), 3);
        assert_eq!(edit_distance("abc", ""), 3);
        assert_eq!(edit_distance("bike squad", "tactical squad"), 7, "bike to tactical, the squad matches");
    }

    #[test]
    fn apostrophes_and_case_are_unified() {
        assert_eq!(name_key("Emperor\u{2019}s Children"), name_key("emperor's children"));
        assert_eq!(name_key("Gork\u{2019}s Klaw - strike"), "gork's klaw strike");
    }
}
