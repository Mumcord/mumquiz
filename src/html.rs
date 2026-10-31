//! Chat HTML. Everything the server sends is untrusted input, so the bot reads it as plain text
//! before matching an answer; everything the bot writes is escaped as an attribute value or as text.

pub fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}

/// Chat HTML to plain text: tags dropped, the few entities Mumble uses decoded, line breaks turned
/// into spaces.
pub fn to_text(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut rest = html;
    // Only a block element reads as a space; an inline tag like <b> does not, and a lone '<' is text.
    let mut pending = false;
    while !rest.is_empty() {
        match marker(rest) {
            Marker::BlockTag => {
                if !out.is_empty() {
                    pending = true;
                }
                rest = after_tag(rest);
            }
            Marker::Tag => rest = after_tag(rest),
            Marker::Entity(name) => {
                // An entity the bot does not know stays as its own name.
                let decoded = entity(name).unwrap_or(name).to_owned();
                if needs_space(pending, &decoded) {
                    out.push(' ');
                }
                out.push_str(&decoded);
                pending = false;
                rest = &rest[name.len() + 2..];
            }
            Marker::Text(end) => {
                let part = &rest[..end];
                if needs_space(pending, part) {
                    out.push(' ');
                }
                out.push_str(part);
                pending = false;
                rest = &rest[end..];
            }
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

enum Marker<'a> {
    BlockTag,
    Tag,
    Entity(&'a str),
    Text(usize),
}

/// The first thing in `rest`: a tag, an entity, or plain text up to the next marker.
fn marker(rest: &str) -> Marker<'_> {
    let Some(c) = rest.chars().next() else {
        return Marker::Text(0);
    };
    if c == '<' {
        return match rest.find('>') {
            Some(end) => {
                let name = rest[1..end]
                    .trim_start_matches('/')
                    .split_whitespace()
                    .next()
                    .unwrap_or("");
                if is_block(name) {
                    Marker::BlockTag
                } else {
                    Marker::Tag
                }
            }
            // A tag without its end is just text.
            None => Marker::Text(1),
        };
    }
    if c == '&' {
        return match rest.find(';') {
            Some(end) => Marker::Entity(&rest[1..end]),
            None => Marker::Text(1),
        };
    }
    let end = rest
        .char_indices()
        .skip(1)
        .find(|(_, c)| *c == '<' || *c == '&')
        .map(|(i, _)| i)
        .unwrap_or(rest.len());
    Marker::Text(end)
}

fn after_tag(rest: &str) -> &str {
    let end = rest.find('>').unwrap_or(rest.len());
    &rest[end + 1..]
}

fn is_block(name: &str) -> bool {
    matches!(
        name,
        "br" | "p"
            | "div"
            | "li"
            | "ul"
            | "ol"
            | "blockquote"
            | "h1"
            | "h2"
            | "h3"
            | "h4"
            | "h5"
            | "h6"
            | "tr"
    )
}

fn entity(name: &str) -> Option<&'static str> {
    match name {
        "amp" => Some("&"),
        "lt" => Some("<"),
        "gt" => Some(">"),
        "quot" => Some("\""),
        "apos" | "#39" => Some("'"),
        "nbsp" => Some(" "),
        _ => None,
    }
}

/// A block boundary reads as a space, unless the text already has one or the next part is punctuation
/// that belongs to the word before it (`:` after a name, `)` after a number).
fn needs_space(pending: bool, part: &str) -> bool {
    if !pending || part.is_empty() {
        return false;
    }
    let first = part.chars().next().unwrap_or(' ');
    if first.is_whitespace() {
        return false;
    }
    // Only punctuation that closes a word attaches to it; an opening '!' or '?' starts the next one.
    !matches!(first, '.' | ',' | ':' | ';' | ')' | ']' | '}')
}

/// `application/x-www-form-urlencoded`, as the card marker in `docs/message-cards.md` uses it.
pub fn form_encode(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for c in value.chars() {
        match c {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' => out.push(c),
            ' ' => out.push('+'),
            _ => {
                let mut buffer = [0u8; 4];
                for byte in c.encode_utf8(&mut buffer).as_bytes() {
                    out.push_str(&format!("%{byte:02X}"));
                }
            }
        }
    }
    out
}
