/// Plain text for retrieval. The steps are the graph spec's `html_to_text`.
pub fn html_to_text(input: &str) -> String {
    let without_blocks = remove_element(&remove_element(input, "script"), "style");
    let with_breaks = replace_break_tags(&without_blocks);
    let without_tags = remove_tags(&with_breaks);
    let decoded = decode_entities(&without_tags);
    collapse_whitespace(&decoded)
}

fn remove_element(input: &str, tag: &str) -> String {
    let open = format!("<{tag}");
    let close = format!("</{tag}");
    let bytes = input.as_bytes();
    let mut out = String::new();
    let mut index = 0;
    while index < bytes.len() {
        if starts_with_ignore_ascii(bytes, index, open.as_bytes())
            && tag_name_ends(bytes, index + open.len())
        {
            if let Some(open_end) = find_byte(bytes, index, b'>') {
                let after_open = open_end + 1;
                if let Some(close_at) = find_ignore_ascii(bytes, after_open, close.as_bytes()) {
                    if tag_name_ends(bytes, close_at + close.len()) {
                        if let Some(close_end) = find_byte(bytes, close_at, b'>') {
                            index = close_end + 1;
                            continue;
                        }
                    }
                }
            }
        }
        let ch = input[index..]
            .chars()
            .next()
            .expect("index is on a char boundary");
        out.push(ch);
        index += ch.len_utf8();
    }
    out
}

fn tag_name_ends(bytes: &[u8], index: usize) -> bool {
    if index >= bytes.len() {
        return true;
    }
    matches!(bytes[index], b'>' | b'/' | b' ' | b'\t' | b'\n' | b'\r')
}

fn replace_break_tags(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = String::new();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'<' {
            if let Some(end) = find_byte(bytes, index, b'>') {
                let inside = &input[index + 1..end];
                if is_break_tag(inside) {
                    out.push('\n');
                    index = end + 1;
                    continue;
                }
            }
        }
        let ch = input[index..]
            .chars()
            .next()
            .expect("index is on a char boundary");
        out.push(ch);
        index += ch.len_utf8();
    }
    out
}

fn is_break_tag(inside: &str) -> bool {
    let compact: String = inside
        .chars()
        .filter(|ch| !ch.is_whitespace())
        .collect::<String>()
        .to_ascii_lowercase();
    matches!(
        compact.as_str(),
        "br" | "br/"
            | "/p"
            | "/div"
            | "/li"
            | "/tr"
            | "/h1"
            | "/h2"
            | "/h3"
            | "/h4"
            | "/h5"
            | "/h6"
    )
}

fn remove_tags(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = String::new();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'<' {
            if let Some(end) = find_byte(bytes, index, b'>') {
                index = end + 1;
                continue;
            }
        }
        let ch = input[index..]
            .chars()
            .next()
            .expect("index is on a char boundary");
        out.push(ch);
        index += ch.len_utf8();
    }
    out
}

fn decode_entities(input: &str) -> String {
    let mut out = String::new();
    let mut rest = input;
    while let Some(start) = rest.find('&') {
        out.push_str(&rest[..start]);
        rest = &rest[start..];
        if let Some(end) = rest.find(';') {
            let token = &rest[..=end];
            if let Some(decoded) = decode_one(token) {
                out.push_str(&decoded);
                rest = &rest[end + 1..];
                continue;
            }
        }
        out.push('&');
        rest = &rest[1..];
    }
    out.push_str(rest);
    out
}

fn decode_one(token: &str) -> Option<String> {
    match token {
        "&amp;" => Some("&".to_string()),
        "&lt;" => Some("<".to_string()),
        "&gt;" => Some(">".to_string()),
        "&quot;" => Some("\"".to_string()),
        "&apos;" => Some("'".to_string()),
        "&nbsp;" => Some("\u{00A0}".to_string()),
        _ => decode_numeric(token),
    }
}

fn decode_numeric(token: &str) -> Option<String> {
    let body = token.strip_prefix("&#")?.strip_suffix(';')?;
    let code = if let Some(hex) = body.strip_prefix(['x', 'X']) {
        if hex.is_empty() || !hex.chars().all(|ch| ch.is_ascii_hexdigit()) {
            return None;
        }
        u32::from_str_radix(hex, 16).ok()?
    } else {
        if body.is_empty() || !body.chars().all(|ch| ch.is_ascii_digit()) {
            return None;
        }
        body.parse::<u32>().ok()?
    };
    char::from_u32(code).map(|ch| ch.to_string())
}

fn collapse_whitespace(input: &str) -> String {
    let mut lines = Vec::new();
    for line in input.split('\n') {
        let mut collapsed = String::new();
        let mut in_space = false;
        for ch in line.chars() {
            if ch.is_whitespace() {
                if !in_space {
                    collapsed.push(' ');
                    in_space = true;
                }
            } else {
                in_space = false;
                collapsed.push(ch);
            }
        }
        lines.push(collapsed.trim().to_string());
    }
    let mut joined = lines.join("\n");
    while joined.contains("\n\n\n") {
        joined = joined.replace("\n\n\n", "\n\n");
    }
    joined.trim().to_string()
}

fn starts_with_ignore_ascii(hay: &[u8], index: usize, needle: &[u8]) -> bool {
    let end = index + needle.len();
    end <= hay.len() && hay[index..end].eq_ignore_ascii_case(needle)
}

fn find_byte(hay: &[u8], from: usize, byte: u8) -> Option<usize> {
    hay[from..]
        .iter()
        .position(|item| *item == byte)
        .map(|pos| from + pos)
}

fn find_ignore_ascii(hay: &[u8], from: usize, needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || hay.len() < from + needle.len() {
        return None;
    }
    let last = hay.len() - needle.len();
    (from..=last).find(|&index| hay[index..index + needle.len()].eq_ignore_ascii_case(needle))
}

#[cfg(test)]
mod tests {
    use super::html_to_text;

    #[test]
    fn strips_scripts_and_keeps_markup_text() {
        let text = html_to_text("<script>alert(1)</script><p>Example rule.</p>\ncontinued");
        assert_eq!(text, "Example rule.\n\ncontinued");
    }

    #[test]
    fn decodes_entities_after_tags_are_gone() {
        assert_eq!(html_to_text("&lt;b&gt;x&amp;y&#39;"), "<b>x&y'");
        assert_eq!(html_to_text("&#x41;"), "A");
        assert_eq!(html_to_text("&#xD800;"), "&#xD800;");
    }

    #[test]
    fn collapses_runs_of_newlines_to_two() {
        assert_eq!(html_to_text("a<br><br/><br/>b"), "a\n\nb");
        assert_eq!(html_to_text("  spaced   out  "), "spaced out");
    }
}
