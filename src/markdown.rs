//! The bot writes chat HTML. A message on the wire is HTML, so the markdown a user types in a client
//! is what the client builds from it; the bot builds the same HTML here from a small subset.
//!
//! Subset: `**bold**`, `*italic*`, `` `code` ``, `[text](https://url)`, a line starting with `> ` as
//! a quote and a line starting with `- ` as a list item. Between lines a break. Everything else is
//! escaped as text, and an unterminated marker stays plain text. No URL other than http(s) is
//! turned into a link, nothing is fetched.

use crate::html;

/// One message: its lines rendered and joined with a break.
pub fn render(text: &str) -> String {
    text.split('\n')
        .map(line)
        .collect::<Vec<_>>()
        .join("<br />")
}

fn line(raw: &str) -> String {
    let trimmed = raw.trim();
    if let Some(body) = trimmed.strip_prefix("> ") {
        return format!("<blockquote>{}</blockquote>", inline(body));
    }
    if let Some(body) = trimmed.strip_prefix("- ") {
        return format!("<ul><li>{}</li></ul>", inline(body));
    }
    inline(raw)
}

fn inline(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut rest = raw;
    while !rest.is_empty() {
        let c = match rest.chars().next() {
            Some(c) => c,
            None => break,
        };
        match c {
            '`' => {
                let body = &rest[1..];
                match body.find('`') {
                    Some(end) => {
                        out.push_str("<code>");
                        out.push_str(&html::escape(&body[..end]));
                        out.push_str("</code>");
                        rest = &body[end + 1..];
                    }
                    None => {
                        out.push_str(&html::escape("`"));
                        rest = body;
                    }
                }
            }
            '*' => {
                let marker = if rest.starts_with("**") { "**" } else { "*" };
                let body = &rest[marker.len()..];
                let tag = if marker.len() == 2 { "b" } else { "i" };
                match body.find(marker) {
                    Some(end) => {
                        out.push_str(&format!("<{tag}>"));
                        out.push_str(&html::escape(&body[..end]));
                        out.push_str(&format!("</{tag}>"));
                        rest = &body[end + marker.len()..];
                    }
                    None => {
                        out.push_str(&html::escape(marker));
                        rest = body;
                    }
                }
            }
            '[' => {
                // `[text](url)`: only a plain http(s) target becomes a link.
                let after = rest.find("](");
                match after {
                    Some(pos) => {
                        let text = &rest[1..pos];
                        let target = &rest[pos + 2..];
                        let end = target.find(')').unwrap_or(target.len());
                        let url = &target[..end];
                        if is_web_url(url) {
                            out.push_str(&format!(
                                "<a href=\"{}\">{}</a>",
                                html::escape(url),
                                html::escape(text)
                            ));
                            rest = &target[end + 1..];
                        } else {
                            // Not a web target: the whole thing stays text.
                            let stop = pos + 2 + end + 1;
                            out.push_str(&html::escape(&rest[..stop]));
                            rest = &rest[stop..];
                        }
                    }
                    None => {
                        out.push_str(&html::escape("["));
                        rest = &rest[1..];
                    }
                }
            }
            _ => {
                out.push_str(&html::escape(&rest[..c.len_utf8()]));
                rest = &rest[c.len_utf8()..];
            }
        }
    }
    out
}

fn is_web_url(url: &str) -> bool {
    if !(url.starts_with("http://") || url.starts_with("https://")) {
        return false;
    }
    !url.chars()
        .any(|c| c.is_whitespace() || c == '<' || c == '>' || c == '"')
}
