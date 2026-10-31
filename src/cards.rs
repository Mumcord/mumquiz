//! How the bot's lines reach chat: plain text, or a Mumcord message card (see
//! `docs/message-cards.md` in the Mumcord repository). Cards are the Mumcord half of the port; the
//! fallback HTML inside the card is what an official Mumble client shows.

use crate::html;
use crate::markdown;
use crate::quiz::Kind;

struct CardStyle {
    title: &'static str,
    color: &'static str,
    icon: &'static str,
}

fn style(kind: Kind) -> CardStyle {
    match kind {
        Kind::Info => CardStyle {
            title: "Quiz",
            color: "info",
            icon: "bot",
        },
        Kind::Question => CardStyle {
            title: "Question",
            color: "info",
            icon: "bot",
        },
        Kind::Hint => CardStyle {
            title: "Hint",
            color: "info",
            icon: "bell",
        },
        Kind::Correct => CardStyle {
            title: "Correct",
            color: "success",
            icon: "check",
        },
        Kind::Winner => CardStyle {
            title: "Winner",
            color: "success",
            icon: "check",
        },
        Kind::Warning => CardStyle {
            title: "Notice",
            color: "warning",
            icon: "warning",
        },
        Kind::Score => CardStyle {
            title: "Score",
            color: "info",
            icon: "chart",
        },
    }
}

#[derive(Debug, Clone)]
pub struct Renderer {
    cards: bool,
}

impl Renderer {
    pub fn new(cards: bool) -> Renderer {
        Renderer { cards }
    }

    pub fn render(&self, kind: Kind, text: &str) -> String {
        // The text becomes chat HTML through the markdown subset; a line without markup reads the
        // same, and the lines of a board are joined with a break.
        let body = markdown::render(text);
        if !self.cards {
            return body;
        }
        let card = style(kind);
        let fields = format!(
            "v=1&title={}&color={}&icon={}",
            html::form_encode(card.title),
            card.color,
            card.icon,
        );
        // The status and the title as text, so a client without cards still reads the line.
        format!(
            "<div data-mumcord-card=\"{}\"><b>[{}] {}</b><div data-mumcord-card=\"body\">{}</div></div>",
            html::escape(&fields),
            status(card.color),
            html::escape(card.title),
            body,
        )
    }
}

fn status(color: &str) -> &'static str {
    match color {
        "success" => "Success",
        "warning" => "Warning",
        "error" => "Error",
        _ => "Info",
    }
}
