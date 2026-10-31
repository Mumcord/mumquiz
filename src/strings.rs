//! The bot's messages. The defaults follow the upstream Python bot; `[strings]` in `bot.toml`
//! replaces single lines. Every line may use `{placeholders}` and the markdown subset of
//! `src/markdown.rs`.

use std::collections::BTreeMap;

use anyhow::{Result, bail};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Strings {
    pub question: String,
    pub waiting: String,
    pub botsnack: String,
    pub hint: String,
    pub hintone: String,
    pub rightanswer: String,
    pub wishluck: String,
    pub correctanswer: String,
    pub winner: String,
    pub thanks: String,
    pub score: String,
    pub channeltopic: String,
    pub invalidname: String,
    pub unknowncmd: String,
    pub help_channelinfo: String,
    pub help_botinfo: String,
    pub reloaded: String,
    pub score_empty: String,
    pub hiscore_empty: String,
    pub stopped: String,
    pub started: String,
    pub help: Vec<String>,
}

impl Default for Strings {
    fn default() -> Self {
        Self {
            question: "**<{category}> Q:** {question}".to_owned(),
            waiting: "Playing alone is no fun. Let's wait for others...".to_owned(),
            botsnack: "I'm hungry. Please feed me with `!botsnack`.".to_owned(),
            hint: "HINT: `{hint}`".to_owned(),
            hintone: "HINT: only one character!".to_owned(),
            rightanswer: "the answer was: **{answer}**".to_owned(),
            wishluck: "better luck with the next question!".to_owned(),
            correctanswer: "**{answer}** is right! congratulations, {name}!".to_owned(),
            winner: "congratulations to **{name}**, you're winner!!!".to_owned(),
            thanks: "ta. :-)".to_owned(),
            score: "{rank}. **{name}**: {points} points".to_owned(),
            channeltopic: "**happy quizzing.** target score: {target}. previous winner: {winner}. \
                            all-time winner: {alltime} ({alltime_wins})."
                .to_owned(),
            invalidname: "not on my watch, **{name}**!".to_owned(),
            unknowncmd: "... wat.".to_owned(),
            help_channelinfo: "**{channel}** is a quiz channel.".to_owned(),
            help_botinfo: "I am **{bot}**, and *I* ask the questions around here! :->".to_owned(),
            reloaded: "reloaded questions.".to_owned(),
            score_empty: "no points yet this round.".to_owned(),
            hiscore_empty: "no wins yet, the board is empty.".to_owned(),
            stopped: "quiz stopped. I stay here, but I ask nothing until `!start`.".to_owned(),
            started: "quiz started.".to_owned(),
            help: vec![
                "`!help` shows this list.".to_owned(),
                "`!score` prints the current top 5 quizzers.".to_owned(),
                "`!hiscore` prints the all time top 5 quizzers.".to_owned(),
                "happy quizzing!".to_owned(),
                "(o, and BTW, I'm hungry, like *all* the freaking time.".to_owned(),
                "you can feed me with `!botsnack`. please do. often.)".to_owned(),
            ],
        }
    }
}

impl Strings {
    /// Defaults with the `[strings]` overrides applied. An unknown key is an error.
    pub fn with_overrides(overrides: &BTreeMap<String, String>) -> Result<Strings> {
        let mut out = Strings::default();
        for (key, value) in overrides {
            match key.as_str() {
                "question" => out.question = value.clone(),
                "waiting" => out.waiting = value.clone(),
                "botsnack" => out.botsnack = value.clone(),
                "hint" => out.hint = value.clone(),
                "hintone" => out.hintone = value.clone(),
                "rightanswer" => out.rightanswer = value.clone(),
                "wishluck" => out.wishluck = value.clone(),
                "correctanswer" => out.correctanswer = value.clone(),
                "winner" => out.winner = value.clone(),
                "thanks" => out.thanks = value.clone(),
                "score" => out.score = value.clone(),
                "channeltopic" => out.channeltopic = value.clone(),
                "invalidname" => out.invalidname = value.clone(),
                "unknowncmd" => out.unknowncmd = value.clone(),
                "help_channelinfo" => out.help_channelinfo = value.clone(),
                "help_botinfo" => out.help_botinfo = value.clone(),
                "reloaded" => out.reloaded = value.clone(),
                "score_empty" => out.score_empty = value.clone(),
                "hiscore_empty" => out.hiscore_empty = value.clone(),
                "stopped" => out.stopped = value.clone(),
                "started" => out.started = value.clone(),
                "help" => out.help = value.lines().map(str::to_owned).collect(),
                other => bail!("unknown strings key {other:?}"),
            }
        }
        Ok(out)
    }

    pub fn question(&self, category: &str, question: &str) -> String {
        fill(
            &self.question,
            &[("category", category), ("question", question)],
        )
    }

    pub fn hint(&self, hint: &str) -> String {
        fill(&self.hint, &[("hint", hint)])
    }

    pub fn rightanswer(&self, answer: &str) -> String {
        fill(&self.rightanswer, &[("answer", answer)])
    }

    pub fn correctanswer(&self, answer: &str, name: &str) -> String {
        fill(&self.correctanswer, &[("answer", answer), ("name", name)])
    }

    pub fn winner(&self, name: &str) -> String {
        fill(&self.winner, &[("name", name)])
    }

    pub fn score(&self, rank: usize, name: &str, points: u64) -> String {
        fill(
            &self.score,
            &[
                ("rank", &rank.to_string()),
                ("name", name),
                ("points", &points.to_string()),
            ],
        )
    }

    pub fn invalidname(&self, name: &str) -> String {
        fill(&self.invalidname, &[("name", name)])
    }

    pub fn help_channelinfo(&self, channel: &str) -> String {
        fill(&self.help_channelinfo, &[("channel", channel)])
    }

    pub fn help_botinfo(&self, bot: &str) -> String {
        fill(&self.help_botinfo, &[("bot", bot)])
    }

    pub fn channeltopic(
        &self,
        target: u64,
        winner: &str,
        alltime: &str,
        alltime_wins: Option<u64>,
    ) -> String {
        fill(
            &self.channeltopic,
            &[
                ("target", &target.to_string()),
                ("winner", winner),
                ("alltime", alltime),
                ("alltime_wins", &alltime_wins.unwrap_or(0).to_string()),
            ],
        )
    }
}

/// Replaces `{key}` with the value; an unknown placeholder stays as it is so a typo in a custom
/// string shows up in the chat instead of dropping text.
pub fn fill(template: &str, args: &[(&str, &str)]) -> String {
    let mut out = String::with_capacity(template.len());
    let mut rest = template;
    while let Some(open) = rest.find('{') {
        out.push_str(&rest[..open]);
        let Some(close) = rest[open..].find('}') else {
            out.push_str(&rest[open..]);
            return out;
        };
        let key = &rest[open + 1..open + close];
        match args.iter().find(|(k, _)| *k == key) {
            Some((_, value)) => out.push_str(value),
            None => out.push_str(&rest[open..open + close + 1]),
        }
        rest = &rest[open + close + 1..];
    }
    out.push_str(rest);
    out
}
