//! The game itself: players, hunger, hints, points and wins. Pure logic with an injected clock and
//! random source, so the whole flow is testable without a server.
//!
//! Seconds are passed as `Now` instead of a real clock. `bot.rs` drives `tick` on a timer and sends
//! the lines this module returns.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use anyhow::Result;

use crate::answer;
use crate::config::{Config, QuizRules};
use crate::question::{Bank, Question};
use crate::rng::Rng;
use crate::scores::{Scores, TOP_LIMIT};
use crate::strings::Strings;

/// Game clock in seconds since the bot started.
pub type Now = u64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Info,
    Question,
    Hint,
    Correct,
    Winner,
    Warning,
    Score,
}

/// One line the bot wants to say.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    pub kind: Kind,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    Channel(Line),
    Private {
        quizzer: String,
        line: Line,
    },
    /// Channel description (the Mumble equivalent of an IRC channel topic).
    Topic(String),
    JoinChannel(String),
    /// Handled by the runner, which reads the bank again and calls `Quiz::set_bank`.
    ReloadQuestions,
}

pub struct Quiz {
    cfg: QuizRules,
    strings: Strings,
    bank: Bank,
    scores: Box<dyn Scores>,
    rng: Rng,
    self_name: String,
    channel: String,
    /// Player name to points. `None`: not answered yet this round, so `!score` skips them.
    quizzers: BTreeMap<String, Option<u64>>,
    masters: BTreeSet<String>,
    recently_asked: VecDeque<String>,
    pending: Option<Question>,
    /// Positions of the answer that are still hidden.
    hidden: BTreeSet<usize>,
    reveal_order: Vec<usize>,
    difficulty: usize,
    hint_num: usize,
    hunger: u64,
    complained: bool,
    target_score: u64,
    winner: Option<String>,
    /// `!stop` and `!start`: the bot stays connected and keeps its players, but it asks nothing and
    /// accepts no answer while it is stopped.
    playing: bool,
    ask_due_at: Now,
    hint_due_at: Now,
    wait_due_at: Now,
}

impl Quiz {
    pub fn new(
        config: &Config,
        strings: &Strings,
        bank: Bank,
        scores: Box<dyn Scores>,
        rng: Rng,
    ) -> Quiz {
        let mut quiz = Quiz {
            cfg: config.quiz.clone(),
            strings: strings.clone(),
            bank,
            scores,
            rng,
            self_name: config.server.name.clone(),
            channel: config.server.channel.clone().unwrap_or_default(),
            quizzers: BTreeMap::new(),
            masters: config.masters().into_iter().collect(),
            recently_asked: VecDeque::new(),
            pending: None,
            hidden: BTreeSet::new(),
            reveal_order: Vec::new(),
            difficulty: 1,
            hint_num: 0,
            hunger: 0,
            complained: false,
            target_score: 1,
            winner: None,
            playing: true,
            ask_due_at: 0,
            hint_due_at: 0,
            wait_due_at: 0,
        };
        quiz.target_score = quiz.target_for(quiz.quizzers.len());
        quiz
    }

    /// The channel the bot plays in, by name. Set at login from the server state, and by `!join`.
    pub fn channel(&self) -> &str {
        &self.channel
    }

    pub fn set_channel(&mut self, name: &str) {
        self.channel = name.to_owned();
    }

    /// Replaces the bank after `!reload`.
    pub fn set_bank(&mut self, bank: Bank) {
        self.bank = bank;
        // A smaller bank can now hold fewer recent questions than the threshold asked for.
        while self.recently_asked.len() > self.recycle_threshold() {
            self.recently_asked.pop_front();
        }
    }

    pub fn bank(&self) -> &Bank {
        &self.bank
    }

    pub fn quizzers(&self) -> &BTreeMap<String, Option<u64>> {
        &self.quizzers
    }

    pub fn pending(&self) -> Option<&Question> {
        self.pending.as_ref()
    }

    pub fn target_score(&self) -> u64 {
        self.target_score
    }

    pub fn hunger(&self) -> u64 {
        self.hunger
    }

    pub fn add(&mut self, name: &str) -> bool {
        if name == self.self_name {
            return false;
        }
        self.quizzers.entry(name.to_owned()).or_insert(None);
        self.target_score = self.target_for(self.quizzers.len());
        true
    }

    pub fn remove(&mut self, name: &str) -> bool {
        let removed = self.quizzers.remove(name).is_some();
        if removed {
            self.target_score = self.target_for(self.quizzers.len());
        }
        removed
    }

    pub fn rename(&mut self, old: &str, new: &str) -> Result<Vec<Action>> {
        let mut out = Vec::new();
        if !self.quizzers.contains_key(old) {
            return Ok(out);
        }
        let points = self.quizzers.remove(old).unwrap_or(None);
        self.quizzers.insert(new.to_owned(), points);
        self.target_score = self.target_for(self.quizzers.len());
        if self.cfg.keep_score_on_rename && self.scores.rename(old, new)? {
            out.push(self.channel_line(Kind::Info, format!("{old} is now {new}, score kept.")));
        }
        Ok(out)
    }

    /// Zero every score and write the channel description (Mumble has no IRC topic).
    pub fn reset(&mut self) -> Vec<Action> {
        for points in self.quizzers.values_mut() {
            *points = None;
        }
        self.target_score = self.target_for(self.quizzers.len());
        vec![Action::Topic(self.topic())]
    }

    fn target_for(&self, players: usize) -> u64 {
        1 + players as u64 / 2
    }

    fn topic(&self) -> String {
        let alltime = self.scores.alltime().unwrap_or(None);
        self.strings.channeltopic(
            self.target_score,
            self.winner.as_deref().unwrap_or("no one"),
            alltime
                .as_ref()
                .map(|(name, _)| name.as_str())
                .unwrap_or("no one"),
            alltime.as_ref().map(|(_, wins)| *wins),
        )
    }

    /// One game tick: the bot's `decide()`.
    pub fn tick(&mut self, now: Now) -> Vec<Action> {
        let mut out = Vec::new();
        if !self.playing {
            return out;
        }
        if self.quizzers.len() < self.cfg.min_players {
            if now >= self.wait_due_at {
                out.push(self.channel_line(Kind::Info, self.strings.waiting.clone()));
                self.wait_due_at = now + self.cfg.waiting_secs;
            }
            return out;
        }
        if self.pending.is_some() {
            if now >= self.hint_due_at {
                self.hint(now, &mut out);
            }
        } else if now >= self.ask_due_at {
            self.ask(now, &mut out);
        }
        out
    }

    /// A line from a player: an answer, or a command.
    pub fn message(&mut self, name: &str, text: &str, now: Now) -> Result<Vec<Action>> {
        let mut out = Vec::new();
        // Only a player in the channel can answer, as upstream required.
        if self.playing
            && self.quizzers.contains_key(name)
            && let Some(question) = self.pending.clone()
            && answer::matches(&question.answers, text)
        {
            self.award(name, now, &mut out);
        }

        let Some(rest) = text.trim().strip_prefix(&self.cfg.command_prefix) else {
            return Ok(out);
        };
        let mut words = rest.split_whitespace();
        let command = match words.next() {
            Some(word) => word.to_lowercase(),
            None => return Ok(out),
        };
        let arg = words.next().unwrap_or("").to_owned();

        match command.as_str() {
            "help" => {
                let target = if arg.is_empty() { name } else { arg.as_str() };
                if !self.quizzers.contains_key(target) {
                    // Keep quiet about people who are not playing, as upstream did.
                    return Ok(out);
                }
                // One card, one line per line: a card per sentence reads as spam.
                let mut lines = vec![
                    self.strings.help_channelinfo(&self.channel),
                    self.strings.help_botinfo(&self.self_name),
                ];
                lines.extend(self.strings.help.iter().cloned());
                out.push(self.private(target, Kind::Info, lines.join("\n")));
            }
            "score" => out.extend(self.print_score()),
            "hiscore" => out.extend(self.print_hiscore()?),
            "stop" => {
                if self.is_master(name, &mut out) {
                    // The players and the points stay, only the game pauses: the open question is
                    // dropped, so nobody wins a question that was asked before the stop.
                    self.playing = false;
                    self.pending = None;
                    out.push(self.channel_line(Kind::Info, self.strings.stopped.clone()));
                }
            }
            "start" => {
                if self.is_master(name, &mut out) {
                    self.playing = true;
                    out.push(self.channel_line(Kind::Info, self.strings.started.clone()));
                }
            }
            "botsnack" => {
                self.hunger = 0;
                self.complained = false;
                out.push(self.channel_line(Kind::Info, self.strings.thanks.clone()));
            }
            "reload" => {
                if self.is_master(name, &mut out) {
                    out.push(Action::ReloadQuestions);
                    out.push(self.channel_line(Kind::Info, self.strings.reloaded.clone()));
                }
            }
            "join" => {
                if self.is_master(name, &mut out) {
                    let target = if arg.is_empty() {
                        self.channel.clone()
                    } else {
                        arg.trim_start_matches('#').to_owned()
                    };
                    out.push(Action::JoinChannel(target));
                }
            }
            "op" => {
                if self.is_master(name, &mut out) {
                    self.masters.insert(arg.clone());
                    out.push(self.channel_line(Kind::Info, format!("{arg} is a master.")));
                }
            }
            "deop" => {
                if self.is_master(name, &mut out) {
                    // The bot itself stays a master: it is the one answering, and it cannot remove
                    // its own right to do so.
                    if arg == self.self_name {
                        out.push(self.channel_line(Kind::Warning, self.strings.invalidname(name)));
                    } else {
                        self.masters.remove(&arg);
                        out.push(
                            self.channel_line(Kind::Info, format!("{arg} is no longer a master.")),
                        );
                    }
                }
            }
            _ => out.push(self.channel_line(Kind::Warning, self.strings.unknowncmd.clone())),
        }
        Ok(out)
    }

    fn is_master(&mut self, name: &str, out: &mut Vec<Action>) -> bool {
        if self.masters.contains(name) || name == self.self_name {
            return true;
        }
        // Mumble has no channel operator and a bot may not kick: warn, and let the server admin act.
        out.push(self.channel_line(Kind::Warning, self.strings.invalidname(name)));
        false
    }

    fn award(&mut self, name: &str, now: Now, out: &mut Vec<Action>) {
        let answer = match self.pending.as_ref() {
            Some(question) => question.answers[0].clone(),
            None => return,
        };
        let points = self.quizzers.entry(name.to_owned()).or_insert(None);
        *points = Some(points.unwrap_or(0) + 1);
        let gained = *points;
        out.push(self.channel_line(Kind::Correct, self.strings.correctanswer(&answer, name)));
        if gained == Some(self.target_score) {
            self.win(name, out);
        }
        self.hunger = self.hunger.saturating_sub(1);
        self.pending = None;
        self.ask_due_at = now + self.cfg.answer_gap_secs;
    }

    fn win(&mut self, name: &str, out: &mut Vec<Action>) {
        if let Err(error) = self.scores.add_win(name) {
            tracing::warn!("saving the win of {name}: {error}");
        }
        self.winner = Some(name.to_owned());
        out.push(self.channel_line(Kind::Winner, self.strings.winner(name)));
        out.extend(self.reset());
    }

    fn ask(&mut self, now: Now, out: &mut Vec<Action>) {
        self.hunger += 1;
        if self.hunger > self.cfg.stamina {
            if !self.complained {
                out.push(self.channel_line(Kind::Warning, self.strings.botsnack.clone()));
                self.complained = true;
            }
            return;
        }
        let Some(question) = self.pick_question() else {
            out.push(self.channel_line(
                Kind::Warning,
                "no questions left, please feed me a bank.".to_owned(),
            ));
            return;
        };
        if self.recently_asked.len() >= self.recycle_threshold() {
            self.recently_asked.pop_front();
        }
        self.recently_asked.push_back(question.text.clone());
        let answer = &question.answers[0];
        // Gaps between the words stay visible from the start: a masked space would hide the word
        // break, and the player has to guess where one word ends.
        self.hidden = answer
            .chars()
            .enumerate()
            .filter(|(_, c)| *c != ' ')
            .map(|(index, _)| index)
            .collect();
        self.reveal_order = self.hidden.iter().copied().collect();
        // Numbers are read from left to right, words in random order.
        if !question.numeric {
            self.rng.shuffle(&mut self.reveal_order);
        }
        // At least one character per hint, so a short answer still shows something.
        self.difficulty = (answer.chars().count() / 6).max(1);
        self.hint_num = 0;
        out.push(self.channel_line(
            Kind::Question,
            self.strings.question(&question.category, &question.text),
        ));
        self.pending = Some(question);
        self.hint_due_at = now + self.cfg.hint_patience_secs;
    }

    fn pick_question(&mut self) -> Option<Question> {
        if self.bank.is_empty() {
            return None;
        }
        let mut order: Vec<usize> = (0..self.bank.len()).collect();
        self.rng.shuffle(&mut order);
        for index in &order {
            let question = &self.bank.questions[*index];
            if !self.recently_asked.contains(&question.text) {
                return Some(question.clone());
            }
        }
        // Every question is recent: start the bank again, as a small bank needs to.
        self.recently_asked.clear();
        Some(self.bank.questions[order[0]].clone())
    }

    fn recycle_threshold(&self) -> usize {
        (self.bank.len() * self.cfg.recycle_percent / 100).max(1)
    }

    fn hint(&mut self, now: Now, out: &mut Vec<Action>) {
        let answer = match self.pending.as_ref() {
            Some(question) => question.answers[0].clone(),
            None => return,
        };
        let chars: Vec<char> = answer.chars().collect();
        if chars.len() <= self.hint_num + 1 {
            if chars.len() == 1 && self.hint_num == 0 {
                out.push(self.channel_line(Kind::Hint, self.strings.hintone.clone()));
                self.hint_num += 1;
                self.hint_due_at = now + self.cfg.hint_patience_secs;
            } else {
                self.fail(now, out);
            }
            return;
        }
        if self.hint_num >= 5 {
            self.fail(now, out);
            return;
        }
        for _ in 0..self.difficulty {
            if let Some(index) = self.reveal_order.pop() {
                self.hidden.remove(&index);
            }
        }
        let shown: String = chars
            .iter()
            .enumerate()
            .map(|(index, c)| {
                if self.hidden.contains(&index) {
                    '*'
                } else {
                    *c
                }
            })
            .collect();
        out.push(self.channel_line(Kind::Hint, self.strings.hint(&shown)));
        self.hint_num += 1;
        self.hint_due_at = now + self.cfg.hint_patience_secs;
    }

    fn fail(&mut self, now: Now, out: &mut Vec<Action>) {
        if let Some(question) = self.pending.as_ref() {
            out.push(self.channel_line(
                Kind::Warning,
                self.strings.rightanswer(&question.answers[0]),
            ));
            out.push(self.channel_line(Kind::Info, self.strings.wishluck.clone()));
        }
        self.pending = None;
        self.ask_due_at = now + self.cfg.answer_gap_secs;
    }

    /// The board as one message, one player per line: five separate messages would look like spam
    /// and murmur would pace them anyway.
    fn print_score(&self) -> Vec<Action> {
        let mut rows: Vec<(String, u64)> = self
            .quizzers
            .iter()
            .filter_map(|(name, points)| points.map(|p| (name.clone(), p)))
            .collect();
        rows.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        let mut lines: Vec<String> = Vec::new();
        let mut rank = 0;
        let mut previous = 0;
        for (n, (name, points)) in rows.iter().take(TOP_LIMIT).enumerate() {
            if *points != previous {
                rank = n + 1;
                previous = *points;
            }
            lines.push(self.strings.score(rank, name, *points));
        }
        // An empty board still answers: a command that says nothing looks broken.
        if lines.is_empty() {
            return vec![self.channel_line(Kind::Score, self.strings.score_empty.clone())];
        }
        vec![self.channel_line(Kind::Score, lines.join("\n"))]
    }

    fn print_hiscore(&self) -> Result<Vec<Action>> {
        let rows = self.scores.top(TOP_LIMIT)?;
        let mut lines: Vec<String> = Vec::new();
        for (n, (name, wins)) in rows.iter().enumerate() {
            lines.push(self.strings.score(n + 1, name, *wins));
        }
        if lines.is_empty() {
            return Ok(vec![
                self.channel_line(Kind::Score, self.strings.hiscore_empty.clone()),
            ]);
        }
        Ok(vec![self.channel_line(Kind::Score, lines.join("\n"))])
    }

    fn channel_line(&self, kind: Kind, text: String) -> Action {
        Action::Channel(Line { kind, text })
    }

    fn private(&self, quizzer: &str, kind: Kind, text: String) -> Action {
        Action::Private {
            quizzer: quizzer.to_owned(),
            line: Line { kind, text },
        }
    }
}
