use std::cmp::Ordering;

use crate::html::html_to_text;

const HTML_FIELDS: &[&str] = &[
    "description",
    "legend",
    "loadout",
    "transport",
    "leader_head",
    "leader_footer",
    "damaged_description",
    "inv_sv_descr",
    "base_size_descr",
];

pub fn part(field_name: &str, value: &str) -> String {
    if HTML_FIELDS.contains(&field_name) {
        html_to_text(value)
    } else {
        value.to_string()
    }
}

pub fn join_parts(parts: impl IntoIterator<Item = String>) -> String {
    parts
        .into_iter()
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

/// Display label after NBSP replacement, trim, and whitespace collapse, plus
/// the Unicode default casefold used as the keyword identity.
pub fn normalize_keyword(raw: &str) -> (String, String) {
    let replaced = raw.replace('\u{00A0}', " ");
    let trimmed = replaced.trim();
    let mut label = String::new();
    let mut in_space = false;
    for ch in trimmed.chars() {
        if ch.is_whitespace() {
            if !in_space {
                label.push(' ');
                in_space = true;
            }
        } else {
            in_space = false;
            label.push(ch);
        }
    }
    let folded = caseless::default_case_fold_str(&label);
    (label, folded)
}

pub fn quote_segment(value: &str) -> String {
    let mut out = String::new();
    for byte in value.as_bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                out.push(*byte as char);
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

pub fn with_line(prefix: &str, line: &str) -> String {
    if line.is_empty() {
        prefix.to_string()
    } else {
        format!("{prefix} {line}")
    }
}

pub fn integer_token(value: &str) -> Option<(bool, String)> {
    let (negative, digits) = if let Some(rest) = value.strip_prefix('-') {
        (true, rest)
    } else if let Some(rest) = value.strip_prefix('+') {
        (false, rest)
    } else {
        (false, value)
    };
    if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    let trimmed = digits.trim_start_matches('0');
    let magnitude = if trimmed.is_empty() {
        "0".to_string()
    } else {
        trimmed.to_string()
    };
    Some((negative, magnitude))
}

pub fn cmp_integer_strings(left: &str, right: &str) -> Ordering {
    let (left_neg, left_mag) = integer_token(left).expect("caller checks every value");
    let (right_neg, right_mag) = integer_token(right).expect("caller checks every value");
    match (left_neg, right_neg) {
        (true, false) => Ordering::Less,
        (false, true) => Ordering::Greater,
        (false, false) => left_mag
            .len()
            .cmp(&right_mag.len())
            .then(left_mag.cmp(&right_mag)),
        (true, true) => right_mag
            .len()
            .cmp(&left_mag.len())
            .then(right_mag.cmp(&left_mag)),
    }
}
