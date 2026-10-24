//! Page text normalisation for evidence hashes, as specified in `registry/README.md`.

/// Elements whose content is never page text.
const SKIPPED: [&str; 6] = ["script", "style", "noscript", "template", "svg", "iframe"];

/// The main text of an HTML page: the first `<main>` element, else `<body>`, else the
/// whole document; comments, tags and skipped elements removed; entities decoded;
/// whitespace collapsed to single spaces.
pub fn main_text(html: &str) -> String {
    // ASCII lowercasing keeps every byte offset, so indexes into `lower` fit `html`.
    let lower = html.to_ascii_lowercase();
    let (start, end) = region(&lower, "main")
        .or_else(|| region(&lower, "body"))
        .unwrap_or((0, html.len()));
    let mut out = String::new();
    let mut i = start;
    while i < end {
        if lower[i..end].starts_with("<!--") {
            i = after(&lower, i, end, "-->");
        } else if lower.as_bytes()[i] == b'<' {
            let close = after(&lower, i, end, ">");
            let name: String = lower[i + 1..close]
                .chars()
                .take_while(char::is_ascii_alphanumeric)
                .collect();
            i = close;
            if SKIPPED.contains(&name.as_str()) {
                let closing = after(&lower, i, end, &format!("</{name}"));
                i = after(&lower, closing, end, ">");
            }
            out.push(' ');
        } else {
            let next = lower[i..end].find('<').map_or(end, |j| i + j);
            out.push_str(&html[i..next]);
            i = next;
        }
    }
    decode_entities(&out)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Byte range of the first `<name>` element, from its opening tag to its closing tag.
fn region(lower: &str, name: &str) -> Option<(usize, usize)> {
    let open = format!("<{name}");
    let start = lower.match_indices(&open).map(|(i, _)| i).find(|&i| {
        lower[i + open.len()..]
            .chars()
            .next()
            .is_some_and(|c| c == '>' || c.is_ascii_whitespace())
    })?;
    let end = lower[start..]
        .find(&format!("</{name}"))
        .map_or(lower.len(), |j| start + j);
    Some((start, end))
}

/// The index just past the next `needle` at or after `from`, or `end` if there is none.
fn after(lower: &str, from: usize, end: usize, needle: &str) -> usize {
    lower[from..end]
        .find(needle)
        .map_or(end, |j| from + j + needle.len())
}

fn decode_entities(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(amp) = rest.find('&') {
        out.push_str(&rest[..amp]);
        rest = &rest[amp..];
        let decoded = rest
            .find(';')
            .filter(|&semi| semi <= 10)
            .and_then(|semi| entity(&rest[1..semi]).map(|c| (c, semi + 1)));
        match decoded {
            Some((c, len)) => {
                out.push(c);
                rest = &rest[len..];
            }
            None => {
                out.push('&');
                rest = &rest[1..];
            }
        }
    }
    out + rest
}

fn entity(name: &str) -> Option<char> {
    let numeric = |digits: &str, radix| {
        u32::from_str_radix(digits, radix)
            .ok()
            .and_then(char::from_u32)
    };
    match name {
        "amp" => Some('&'),
        "lt" => Some('<'),
        "gt" => Some('>'),
        "quot" => Some('"'),
        "apos" => Some('\''),
        "nbsp" => Some('\u{a0}'),
        _ => match name.strip_prefix("#x").or_else(|| name.strip_prefix("#X")) {
            Some(hex) => numeric(hex, 16),
            None => name.strip_prefix('#').and_then(|dec| numeric(dec, 10)),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_main_text_only() {
        let html = "<html><head><title>T</title></head><body><nav>Menu</nav>\
            <main class=\"x\"><h1>Privacy</h1><p>Email <b>privacy@x.au</b></p></main>\
            <footer>2026</footer></body></html>";
        assert_eq!(main_text(html), "Privacy Email privacy@x.au");
    }

    #[test]
    fn falls_back_to_body_and_skips_scripts_and_comments() {
        let html = "<BODY><script>var a = '<p>';</script><!-- note -->Hello\n\n  <STYLE>p{}</STYLE>world</BODY>";
        assert_eq!(main_text(html), "Hello world");
    }

    #[test]
    fn does_not_take_a_tag_that_only_starts_with_main() {
        let html = "<body><mainmenu>Menu</mainmenu><p>Text</p></body>";
        assert_eq!(main_text(html), "Menu Text");
    }

    #[test]
    fn decodes_entities() {
        assert_eq!(
            main_text("A&amp;B &lt;x&gt; &#39;q&#x27; a&nbsp;b &bogus; & c"),
            "A&B <x> 'q' a b &bogus; & c"
        );
    }
}
