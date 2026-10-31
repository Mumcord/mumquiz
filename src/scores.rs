//! High scores of all time. `Scores` is a trait so the game logic can run on memory in tests while
//! the bot uses the SQLite file the upstream bot already had.

use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{Context, Result};
use rusqlite::Connection;

pub const TOP_LIMIT: usize = 5;

pub trait Scores: Send {
    /// Wins of one quizzer; `None` when they never won.
    fn wins(&self, quizzer: &str) -> Option<u64>;
    fn add_win(&mut self, quizzer: &str) -> Result<()>;
    /// Best quizzers, most wins first.
    fn top(&self, limit: usize) -> Result<Vec<(String, u64)>>;
    /// Move a score to a renamed quizzer; true when there was a score to move.
    fn rename(&mut self, old: &str, new: &str) -> Result<bool>;
    /// All-time winner for the channel topic.
    fn alltime(&self) -> Result<Option<(String, u64)>> {
        Ok(self.top(1)?.first().cloned())
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct MemoryScores {
    wins: BTreeMap<String, u64>,
}

impl MemoryScores {
    pub fn new() -> MemoryScores {
        MemoryScores::default()
    }
}

impl Scores for MemoryScores {
    fn wins(&self, quizzer: &str) -> Option<u64> {
        self.wins.get(quizzer).copied()
    }

    fn add_win(&mut self, quizzer: &str) -> Result<()> {
        *self.wins.entry(quizzer.to_owned()).or_insert(0) += 1;
        Ok(())
    }

    fn top(&self, limit: usize) -> Result<Vec<(String, u64)>> {
        let mut rows: Vec<(String, u64)> = self.wins.clone().into_iter().collect();
        rows.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        rows.truncate(limit);
        Ok(rows)
    }

    fn rename(&mut self, old: &str, new: &str) -> Result<bool> {
        match self.wins.remove(old) {
            Some(wins) => {
                *self.wins.entry(new.to_owned()).or_insert(0) += wins;
                Ok(true)
            }
            None => Ok(false),
        }
    }
}

pub struct SqliteScores {
    conn: Connection,
}

impl SqliteScores {
    pub fn open(path: &Path) -> Result<SqliteScores> {
        if let Some(parent) = path.parent()
            && !parent.as_os_str().is_empty()
        {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating {}", parent.display()))?;
        }
        let conn = Connection::open(path).with_context(|| format!("opening {}", path.display()))?;
        // `isolation_level = None` in the upstream bot: every write commits at once.
        conn.execute(
            "CREATE TABLE IF NOT EXISTS hiscore (quizzer TEXT UNIQUE, wins INTEGER)",
            [],
        )
        .context("creating the hiscore table")?;
        Ok(SqliteScores { conn })
    }
}

impl Scores for SqliteScores {
    fn wins(&self, quizzer: &str) -> Option<u64> {
        self.conn
            .query_row(
                "SELECT wins FROM hiscore WHERE quizzer = ?1",
                [quizzer],
                |row| row.get::<_, i64>(0),
            )
            .ok()
            .map(|wins| wins as u64)
    }

    fn add_win(&mut self, quizzer: &str) -> Result<()> {
        match self.wins(quizzer) {
            Some(wins) => self.conn.execute(
                "UPDATE hiscore SET wins = ?1 WHERE quizzer = ?2",
                (wins as i64 + 1, quizzer),
            ),
            None => self.conn.execute(
                "INSERT INTO hiscore (quizzer, wins) VALUES (?1, ?2)",
                (quizzer, 1i64),
            ),
        }
        .context("saving the win")?;
        Ok(())
    }

    fn top(&self, limit: usize) -> Result<Vec<(String, u64)>> {
        let mut statement = self
            .conn
            .prepare("SELECT quizzer, wins FROM hiscore ORDER BY wins DESC, quizzer ASC LIMIT ?1")
            .context("preparing the hiscore query")?;
        let rows = statement
            .query_map([limit as i64], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)? as u64))
            })
            .context("reading the hiscore")?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row.context("reading a hiscore row")?);
        }
        Ok(out)
    }

    fn rename(&mut self, old: &str, new: &str) -> Result<bool> {
        let changed = self
            .conn
            .execute(
                "UPDATE hiscore SET quizzer = ?1 WHERE quizzer = ?2",
                (new, old),
            )
            .context("moving a hiscore entry")?;
        Ok(changed > 0)
    }
}
