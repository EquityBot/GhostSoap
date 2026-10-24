//! Page text normalisation for evidence hashes, as specified in `registry/README.md`.

/// Elements whose content is never page text.
const SKIPPED: [&str; 6] = ["script", "style", "noscript", "template", "svg", "iframe"];

/// Whether a response is HTML: its media type names HTML in any letter case or, with no
/// `Content-Type`, its body starts like an HTML document.
pub fn is_html(content_type: Option<&str>, body: &[u8]) -> bool {
    match content_type {
        Some(media) => media.to_ascii_lowercase().contains("html"),
        None => {
            let body = body.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(body);
            let start = body.trim_ascii_start();
            let head = start.get(..14).unwrap_or(start).to_ascii_lowercase();
            head.starts_with(b"<!doctype html") || head.starts_with(b"<html")
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Seen {
    No,
    Inside,
    Done,
}

/// The text of the first element called `name`.
struct Region {
    name: &'static str,
    seen: Seen,
    text: String,
}

impl Region {
    fn new(name: &'static str) -> Self {
        Self {
            name,
            seen: Seen::No,
            text: String::new(),
        }
    }

    fn tag(&mut self, name: &str, closing: bool) {
        match (self.seen, closing) {
            (Seen::No, false) if name == self.name => self.seen = Seen::Inside,
            (Seen::Inside, true) if name == self.name => self.seen = Seen::Done,
            _ => {}
        }
    }

    fn push(&mut self, piece: &str) {
        if self.seen == Seen::Inside {
            self.text.push_str(piece);
        }
    }
}

/// The main text of an HTML page: the first `<main>` element, else `<body>`, else the
/// whole document, ignoring comments and skipped elements; tags removed; entities
/// decoded; whitespace collapsed to single spaces.
pub fn main_text(html: &str) -> String {
    // ASCII lowercasing keeps every byte offset, so indexes into `lower` fit `html`.
    let lower = html.to_ascii_lowercase();
    let (mut main, mut body, mut all) = (Region::new("main"), Region::new("body"), String::new());
    let mut i = 0;
    while i < html.len() {
        let piece = if lower[i..].starts_with("<!--") {
            i = after(&lower, i, "-->");
            continue;
        } else if lower.as_bytes()[i] == b'<' {
            let close = tag_end(&lower, i);
            let closing = lower[i + 1..].starts_with('/');
            let name: String = lower[i + 1 + usize::from(closing)..close]
                .chars()
                .take_while(|&c| !c.is_ascii_whitespace() && c != '/' && c != '>')
                .collect();
            i = close;
            if !closing && SKIPPED.contains(&name.as_str()) {
                let closing_tag = after(&lower, i, &format!("</{name}"));
                i = after(&lower, closing_tag, ">");
            } else {
                main.tag(&name, closing);
                body.tag(&name, closing);
            }
            " "
        } else {
            let next = lower[i..].find('<').map_or(html.len(), |j| i + j);
            let text = &html[i..next];
            i = next;
            text
        };
        main.push(piece);
        body.push(piece);
        all.push_str(piece);
    }
    let text = if main.seen != Seen::No {
        main.text
    } else if body.seen != Seen::No {
        body.text
    } else {
        all
    };
    decode_entities(&text)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// The index just past the `>` that ends the tag starting at `from`, skipping quoted
/// attribute values, or the end of the document.
fn tag_end(lower: &str, from: usize) -> usize {
    let bytes = lower.as_bytes();
    let mut i = from + 1;
    while i < bytes.len() {
        match bytes[i] {
            b'>' => return i + 1,
            b'=' => {
                i += 1;
                while bytes.get(i).is_some_and(u8::is_ascii_whitespace) {
                    i += 1;
                }
                if let Some(&quote @ (b'"' | b'\'')) = bytes.get(i) {
                    i = bytes[i + 1..]
                        .iter()
                        .position(|&b| b == quote)
                        .map_or(bytes.len(), |j| i + 1 + j + 1);
                }
            }
            _ => i += 1,
        }
    }
    bytes.len()
}

/// The index just past the next `needle` at or after `from`, or the end of the document.
fn after(lower: &str, from: usize, needle: &str) -> usize {
    lower[from..]
        .find(needle)
        .map_or(lower.len(), |j| from + j + needle.len())
}

fn decode_entities(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(amp) = rest.find('&') {
        out.push_str(&rest[..amp]);
        rest = &rest[amp..];
        // An entity is at most 10 bytes before its `;`, so look no further.
        let decoded = rest
            .bytes()
            .take(11)
            .position(|b| b == b';')
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
        let html = "<body><mainmenu>Menu</mainmenu><main-nav>Nav</main-nav><p>Text</p></body>";
        assert_eq!(main_text(html), "Menu Nav Text");
    }

    #[test]
    fn ignores_main_tags_in_comments_scripts_and_templates() {
        let html = "<!-- <main>Old placeholder</main> --><html><body>\
            <template><main>Template</main></template>\
            <script>document.write('<main>Fake</main>')</script>\
            <main>Current <script>'</main>'</script>policy</main></body></html>";
        assert_eq!(main_text(html), "Current policy");
    }

    #[test]
    fn a_quoted_angle_bracket_does_not_end_a_tag() {
        let html = "<main><a title=\"a > b\" data-x = 'c>d'>Link</a> text</main>";
        assert_eq!(main_text(html), "Link text");
    }

    #[test]
    fn decodes_entities() {
        assert_eq!(
            main_text("A&amp;B &lt;x&gt; &#39;q&#x27; a&nbsp;b &bogus; & c"),
            "A&B <x> 'q' a b &bogus; & c"
        );
    }

    #[test]
    fn many_unterminated_ampersands_decode_in_linear_time() {
        // Quadratic scanning would take minutes on this input.
        let text = "&".repeat(500_000);
        assert_eq!(decode_entities(&text), text);
    }

    #[test]
    fn detects_html_by_media_type_or_markup() {
        let page = b"\xEF\xBB\xBF \n<!DOCTYPE HTML><html></html>";
        assert!(is_html(Some("text/html; charset=utf-8"), b""));
        assert!(is_html(Some("TEXT/HTML"), b""));
        assert!(is_html(None, page));
        assert!(is_html(None, b"<HTML lang=\"en\">"));
        assert!(!is_html(Some("application/pdf"), page));
        assert!(!is_html(None, b"%PDF-1.7"));
    }
}
