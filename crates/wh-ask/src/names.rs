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

/// Whether the whole label occurs inside the (already folded) haystack.
pub(crate) fn label_in(haystack: &str, label: &str) -> bool {
    let needle = strip_articles(label);
    needle.chars().count() >= 2 && haystack.contains(&needle)
}

#[cfg(test)]
mod tests {
    use super::{fold, label_in, name_key, strip_articles};

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
    fn apostrophes_and_case_are_unified() {
        assert_eq!(name_key("Emperor\u{2019}s Children"), name_key("emperor's children"));
        assert_eq!(name_key("Gork\u{2019}s Klaw - strike"), "gork's klaw strike");
    }
}
