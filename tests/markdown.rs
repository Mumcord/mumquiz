//! The markdown subset the bot writes as chat HTML (src/markdown.rs).

use mumquiz::markdown;

#[test]
fn the_subset_renders_chat_html() {
    assert_eq!(
        markdown::render("**Quiz** is a quiz channel."),
        "<b>Quiz</b> is a quiz channel."
    );
    assert_eq!(
        markdown::render("`!help` shows this list."),
        "<code>!help</code> shows this list."
    );
    assert_eq!(
        markdown::render("like *all* the time"),
        "like <i>all</i> the time"
    );
    assert_eq!(markdown::render("a\nb"), "a<br />b");
    assert_eq!(
        markdown::render("> quoted"),
        "<blockquote>quoted</blockquote>"
    );
    assert_eq!(markdown::render("- item"), "<ul><li>item</li></ul>");
    assert_eq!(
        markdown::render("[mumcord](https://example.dev)"),
        "<a href=\"https://example.dev\">mumcord</a>"
    );
}

#[test]
fn anything_outside_the_subset_stays_text() {
    // An unterminated marker is plain text, it never cuts the string at a byte.
    assert_eq!(markdown::render("a * b"), "a * b");
    assert_eq!(markdown::render("a ` code"), "a ` code");
    assert_eq!(markdown::render("ü"), "ü");
    assert_eq!(
        markdown::render("<script>alert(1)</script>"),
        "&lt;script&gt;alert(1)&lt;/script&gt;"
    );
    // Only a web target becomes a link.
    assert_eq!(
        markdown::render("[x](javascript:alert(1))"),
        "[x](javascript:alert(1))"
    );
    assert_eq!(
        markdown::render("[x](https://example.dev x)"),
        "[x](https://example.dev x)"
    );
    assert_eq!(markdown::render("[no target]"), "[no target]");
}
