//! Chat output: the card wire format Mumcord renders, and reading chat HTML back as text.

use mumquiz::cards::Renderer;
use mumquiz::html;
use mumquiz::quiz::Kind;

#[test]
fn a_card_carries_its_fields_as_a_encoded_query() {
    let render = Renderer::new(true);
    let html = render.render(
        Kind::Correct,
        "\"Merkur\" is right! congratulations, barbara!",
    );
    assert!(html.starts_with(
        "<div data-mumcord-card=\"v=1&amp;title=Correct&amp;color=success&amp;icon=check\">"
    ));
    assert!(html.contains("<div data-mumcord-card=\"body\">"));
    assert!(html.contains("[Success]"));
    // Quotes and ampersands are escaped as an attribute value.
    let escaped = render.render(Kind::Warning, "a & b \"c\"");
    assert!(escaped.contains("&amp;"));
    assert!(escaped.contains("&quot;"));
    assert!(html::form_encode("CPU high").contains('+'));
}

#[test]
fn plain_mode_sends_the_text_as_chat_text() {
    let render = Renderer::new(false);
    assert_eq!(render.render(Kind::Info, "a < b"), "a &lt; b");
}

#[test]
fn chat_html_reads_as_plain_text() {
    assert_eq!(
        html::to_text("<b>[Info] Quiz</b><br>the answer was &quot;Merkur&quot;"),
        "[Info] Quiz the answer was \"Merkur\""
    );
    assert_eq!(html::to_text("hello"), "hello");
    assert_eq!(
        html::to_text("<a href=\"https://example.org\">link</a>"),
        "link"
    );
}

#[test]
fn to_text_survives_multi_byte_chars_and_broken_html() {
    // The bot crashed once on a message that started with a two-byte character.
    assert_eq!(html::to_text("üller"), "üller");
    assert_eq!(html::to_text("<b>hölle</b>"), "hölle");
    assert_eq!(html::to_text("TÜREN"), "TÜREN");
    // An entity or a tag without its end stays text; it must not cut the string at a byte.
    assert_eq!(html::to_text("a &amp"), "a &amp");
    // A lone '<' is text, and a boundary does not add a space before punctuation.
    assert_eq!(html::to_text("a < b"), "a < b");
    assert_eq!(
        html::to_text("1. <b>barbara</b>: 1 points"),
        "1. barbara: 1 points"
    );
    assert_eq!(html::to_text("<b>a</b>.<b>b</b>"), "a.b");
    assert_eq!(html::to_text("&"), "&");
    // A tag without its end is text, not a swallowed rest of the message.
    assert_eq!(html::to_text("<b"), "<b");
}

#[test]
fn a_board_renders_as_one_card_with_its_lines_broken() {
    // One message, one card, the players as lines of that card.
    let render = Renderer::new(true);
    let card = render.render(Kind::Score, "1. barbara: 2 points\n2. zorg: 1 point");
    assert!(
        card.contains(
            "<div data-mumcord-card=\"body\">1. barbara: 2 points<br />2. zorg: 1 point</div>"
        ),
        "{card}"
    );
    // A raw newline in chat HTML is collapsed by the client, so the card uses a break.
    assert!(!card.contains("\n"), "{card}");
    // Plain mode: the same HTML, without the card wrapper.
    let plain = Renderer::new(false);
    assert_eq!(plain.render(Kind::Score, "1. a\n2. b"), "1. a<br />2. b");
    // A command in a line is marked as code.
    assert!(
        plain
            .render(Kind::Info, "`!botsnack`")
            .contains("<code>!botsnack</code>")
    );
}

#[test]
fn the_help_renders_its_markdown_as_chat_html() {
    let render = Renderer::new(true);
    let card = render.render(
        Kind::Info,
        "**Quiz** is a quiz channel.\n`!score` prints the board.",
    );
    assert!(card.contains("<b>Quiz</b>"), "{card}");
    assert!(card.contains("<code>!score</code>"), "{card}");
    assert!(card.contains("<br />"), "{card}");
    // A client that shows no cards still reads the same words.
    assert_eq!(
        html::to_text(&card),
        "[Info] Quiz Quiz is a quiz channel. !score prints the board."
    );
}

#[test]
fn the_styled_cards_read_as_chat_html() {
    let render = Renderer::new(true);
    // The question card: the category and the marker bold, the question as it is.
    let card = render.render(Kind::Question, "**<Geographie> Q:** Nenne einen Ozean!");
    assert!(card.contains("<b>&lt;Geographie&gt; Q:</b>"), "{card}");
    // A hint keeps its asterisks: they sit in a code span, so markdown does not eat them.
    let hint = render.render(Kind::Hint, "HINT: `*a*****`");
    assert!(hint.contains("<code>*a*****</code>"), "{hint}");
    // A board line with the player bold, and the answer bold on the correct line.
    assert!(
        render
            .render(Kind::Score, "1. **barbara**: 1 points")
            .contains("<b>barbara</b>")
    );
    assert!(
        render
            .render(Kind::Correct, "**Merkur** is right!")
            .contains("<b>Merkur</b>"),
        "the winner line"
    );
}
