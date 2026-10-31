//! mumquiz: the quiz bot for Mumble. Reads `bot.toml`, connects with its own certificate and plays
//! the game in the configured channel.

use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Parser;
use tokio::sync::mpsc;

use mumquiz::bot::{Bot, Command};
use mumquiz::config::Config;

#[derive(Parser)]
#[command(
    name = "mumquiz",
    version,
    about = "Quiz bot for standard Mumble servers: asks questions, awards points, keeps high scores"
)]
struct Cli {
    /// Config file. Keys and secrets: keep it private, it is never committed.
    #[arg(long, env = "MUMQUIZ_CONFIG", default_value = "bot.toml")]
    config: PathBuf,
    /// Question bank, overriding quiz.questions.
    #[arg(long, env = "MUMQUIZ_QUESTIONS")]
    questions: Option<PathBuf>,
    /// Certificate folder, overriding server.data_dir.
    #[arg(long, env = "MUMQUIZ_DATA")]
    data_dir: Option<PathBuf>,
    /// Name the players see, overriding server.name.
    #[arg(long, env = "MUMQUIZ_NAME")]
    name: Option<String>,
    /// Channel the bot joins, overriding server.channel.
    #[arg(long, env = "MUMQUIZ_CHANNEL")]
    channel: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let cli = Cli::parse();
    let text = std::fs::read_to_string(&cli.config)
        .with_context(|| format!("reading config {}", cli.config.display()))?;
    let mut config = Config::parse(&text)?;
    if let Some(questions) = &cli.questions {
        config.quiz.questions = questions.clone();
    }
    if let Some(data_dir) = &cli.data_dir {
        config.server.data_dir = Some(data_dir.clone());
    }
    if let Some(name) = &cli.name {
        config.server.name = name.clone();
    }
    if let Some(channel) = &cli.channel {
        config.server.channel = Some(channel.clone());
    }

    let mut bot = Bot::new(config)?;
    let (tx, mut rx) = mpsc::channel::<Command>(8);
    spawn_signals(tx.clone());
    bot.run(&mut rx).await
}

/// Ctrl-C and SIGTERM stop cleanly (the bot sends what is still queued before closing), SIGHUP reads
/// the question bank again.
fn spawn_signals(tx: mpsc::Sender<Command>) {
    let stop = tx.clone();
    tokio::spawn(async move {
        if tokio::signal::ctrl_c().await.is_ok() {
            let _ = stop.send(Command::Stop).await;
        }
    });
    #[cfg(unix)]
    {
        let stop = tx.clone();
        tokio::spawn(async move {
            match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
                Ok(mut signal) => {
                    while signal.recv().await.is_some() {
                        let _ = stop.send(Command::Stop).await;
                    }
                }
                Err(error) => tracing::warn!("cannot watch SIGTERM: {error}"),
            }
        });
        tokio::spawn(async move {
            match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::hangup()) {
                Ok(mut signal) => {
                    while signal.recv().await.is_some() {
                        let _ = tx.send(Command::Reload).await;
                    }
                }
                Err(error) => tracing::warn!("cannot watch SIGHUP: {error}"),
            }
        });
    }
}
