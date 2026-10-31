//! Mumquiz: the game logic of `zorgch/irc-quizbot` on the Mumble protocol.
//!
//! The library holds the parts that can be tested on their own (config, question bank, answer
//! matching, game state, high scores, message strings, chat HTML from a markdown subset). `src/bot.rs` connects those to a
//! live connection, `src/main.rs` is the binary.

pub mod answer;
pub mod bot;
pub mod cards;
pub mod cert;
pub mod config;
pub mod html;
pub mod markdown;
pub mod pins;
pub mod question;
pub mod quiz;
pub mod rng;
pub mod scores;
pub mod strings;

pub use config::Config;
pub use quiz::{Action, Kind, Line, Quiz};
