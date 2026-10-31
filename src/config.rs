//! `bot.toml`: the server, the bot's identity and the quiz rules. An unknown key or section is an
//! error, so a typo is reported at startup instead of silently ignored.

use std::collections::BTreeMap;
use std::path::PathBuf;

use anyhow::{Result, bail};
use serde::Deserialize;

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    #[serde(default)]
    pub server: Server,
    #[serde(default)]
    pub quiz: QuizRules,
    /// Message overrides; see `docs/strings.md`.
    #[serde(default)]
    pub strings: BTreeMap<String, String>,
    /// Channel passwords, one `[[token]]` block per channel: `channel` (name or path) and
    /// `password`. They are presented to the server at login as Mumble access tokens, so a channel
    /// that needs one can be entered. The passwords are never logged.
    #[serde(default, rename = "token", alias = "tokens")]
    pub tokens: Vec<Token>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Token {
    /// The channel the password belongs to: a name (`Quiz`) or a path (`Root/Quiz`).
    pub channel: String,
    pub password: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Server {
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default = "default_name")]
    pub name: String,
    /// Name of the persistent client certificate; defaults to `name` with anything outside
    /// `A-Z a-z 0-9 - _` replaced by `_`. Keep it stable to keep the bot's identity.
    pub cert_name: Option<String>,
    /// Connect password. Prefer `SERVER_PASSWORD` in the environment so the file holds no secret.
    pub password: Option<String>,
    /// Folder for the certificate and the pinned fingerprints; defaults to
    /// `$XDG_DATA_HOME/mumquiz`.
    /// Optional channel to join at login (a name like `Quiz`, or a path like `Root/Quiz`). Without
    /// it the bot stays in the channel the server puts it in at login.
    pub channel: Option<String>,
    pub data_dir: Option<PathBuf>,
    /// Log in as `SuperUser` with `SUPERUSER_PASSWORD` instead of a certificate. Local test
    /// servers only.
    #[serde(default)]
    pub superuser: bool,
    /// Register the certificate at login, so the server knows the bot as a user and the ACL rights
    /// (chat, whisper, channel description) apply. Needs `Self-Register`, or `Register` from an
    /// admin. Without it the bot stays an anonymous client.
    #[serde(default)]
    pub register: bool,
    /// Trust the server certificate through its pinned fingerprint (trust on first use). Only a
    /// local test server may turn this off, and the bot warns when it is off.
    #[serde(default = "default_true")]
    pub pin: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct QuizRules {
    /// Question bank (TOML, one `[[question]]` block per question).
    pub questions: PathBuf,
    /// High score database; created on first use.
    pub hiscores: PathBuf,
    /// Below this many players the bot waits.
    pub min_players: usize,
    /// Seconds a question may stay unanswered before the bot gives hints.
    pub hint_patience_secs: u64,
    /// Questions the bot answers before it gets hungry.
    pub stamina: u64,
    /// Share of the bank kept as "recently asked" questions (percent).
    pub recycle_percent: usize,
    /// Seconds between an answer and the next question.
    pub answer_gap_secs: u64,
    /// Seconds between two game ticks.
    pub tick_secs: u64,
    /// Seconds between two "waiting for players" messages.
    pub waiting_secs: u64,
    /// Command prefix, for example `!`.
    pub command_prefix: String,
    /// Names that may use the master commands. The bot itself is always a master.
    pub masters: Vec<String>,
    /// Keep a player's score when their name changes.
    pub keep_score_on_rename: bool,
    /// Send game messages as Mumcord message cards.
    pub cards: bool,
    /// Log every question the bot asks and every command it sees.
    pub verbose: bool,
}

fn default_host() -> String {
    "127.0.0.1".to_owned()
}

fn default_port() -> u16 {
    64738
}

fn default_name() -> String {
    "Mumquiz".to_owned()
}

fn default_true() -> bool {
    true
}

impl Default for Server {
    fn default() -> Self {
        Self {
            host: default_host(),
            port: default_port(),
            name: default_name(),
            cert_name: None,
            password: None,
            data_dir: None,
            channel: None,
            superuser: false,
            register: false,
            pin: true,
        }
    }
}

impl Default for QuizRules {
    fn default() -> Self {
        Self {
            // The bot needs a bank; this is the default next to `bot.toml`.
            questions: PathBuf::from("questions.toml"),
            hiscores: PathBuf::from("quiz-hiscores.sqlite"),
            min_players: 2,
            hint_patience_secs: 20,
            stamina: 12,
            recycle_percent: 80,
            answer_gap_secs: 5,
            tick_secs: 1,
            waiting_secs: 30,
            command_prefix: "!".to_owned(),
            masters: Vec::new(),
            keep_score_on_rename: true,
            cards: true,
            verbose: false,
        }
    }
}

impl Config {
    pub fn parse(text: &str) -> Result<Config> {
        let config: Config = toml::from_str(text)?;
        config.validate()?;
        Ok(config)
    }

    pub fn validate(&self) -> Result<()> {
        if self.server.name.is_empty() {
            bail!("server.name is empty");
        }
        if self.quiz.min_players == 0 {
            bail!("quiz.min_players must be above 0");
        }
        if self.quiz.hint_patience_secs < 5 {
            bail!("quiz.hint_patience_secs must be at least 5");
        }
        if self.quiz.stamina == 0 {
            bail!("quiz.stamina must be above 0");
        }
        if self.quiz.recycle_percent == 0 || self.quiz.recycle_percent > 100 {
            bail!("quiz.recycle_percent must be between 1 and 100");
        }
        if self.quiz.answer_gap_secs == 0 || self.quiz.tick_secs == 0 || self.quiz.waiting_secs == 0
        {
            bail!("quiz.answer_gap_secs, quiz.tick_secs and quiz.waiting_secs must be above 0");
        }
        if self.quiz.command_prefix.is_empty() {
            bail!("quiz.command_prefix is empty");
        }
        Ok(())
    }

    /// Masters, with the bot itself included as the upstream bot did.
    pub fn masters(&self) -> Vec<String> {
        let mut names: Vec<String> = self.quiz.masters.clone();
        names.push(self.server.name.clone());
        names.sort();
        names.dedup();
        names
    }
}
