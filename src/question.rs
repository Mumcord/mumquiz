//! Question bank: one `[[question]]` block per question, read from a TOML file and reloaded by
//! `!reload` (or SIGHUP).

use std::path::Path;

use anyhow::{Context, Result, bail};
use serde::Deserialize;

/// The upstream bot needs at least this many questions for the "recently asked" recycling to make
/// sense; a smaller bank is only a warning.
pub const MIN_QUESTIONS: usize = 11;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Question {
    pub category: String,
    pub text: String,
    /// Answers a player may type. The first one is the answer the bot shows when nobody wins.
    pub answers: Vec<String>,
    /// True when the answer is a number: hints then open from left to right instead of shuffled.
    pub numeric: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Bank {
    pub questions: Vec<Question>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Row {
    category: String,
    question: String,
    answer: String,
    #[serde(default)]
    answers: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct File {
    #[serde(default)]
    question: Vec<Row>,
}

impl Bank {
    pub fn load(path: &Path) -> Result<Bank> {
        let text = std::fs::read_to_string(path)
            .with_context(|| format!("reading question bank {}", path.display()))?;
        let bank = Bank::parse(&text).with_context(|| format!("parsing {}", path.display()))?;
        if bank.questions.is_empty() {
            bail!("question bank {} is empty", path.display());
        }
        Ok(bank)
    }

    pub fn parse(text: &str) -> Result<Bank> {
        let file: File = toml::from_str(text)?;
        let mut bank = Bank::default();
        for (n, row) in file.question.iter().enumerate() {
            if row.category.trim().is_empty() || row.question.trim().is_empty() {
                bail!("question {} needs a category and a question", n + 1);
            }
            let mut answers: Vec<String> = row
                .answer
                .split('|')
                .map(|a| a.trim().to_owned())
                .filter(|a| !a.is_empty())
                .collect();
            answers.extend(
                row.answers
                    .iter()
                    .map(|a| a.trim().to_owned())
                    .filter(|a| !a.is_empty()),
            );
            if answers.is_empty() {
                bail!("question {} has no answer", n + 1);
            }
            bank.questions.push(Question {
                category: row.category.trim().to_owned(),
                text: row.question.trim().to_owned(),
                numeric: answers[0].chars().all(|c| c.is_ascii_digit()),
                answers,
            });
        }
        Ok(bank)
    }

    pub fn len(&self) -> usize {
        self.questions.len()
    }

    pub fn is_empty(&self) -> bool {
        self.questions.is_empty()
    }
}
