//! Live suite: the bot against the local test server from `testenv/`, with scripted players.
//! Nothing here runs by default; CI starts the server and runs the suite through
//! `scripts/smoke/smoke.sh`. Locally:
//!
//!     testenv/up.sh
//!     cargo test --test live -- --ignored --test-threads=1
//!
//! The host is checked so a test can never reach a public server (scripts/ci/check-loopback.sh).

use std::collections::BTreeMap;
use std::env;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use mumcord_protocol::{ConnectConfig, Event, TextTarget, connect};
use mumquiz::bot::{Bot, Command};
use mumquiz::config::{Config, QuizRules, Server};
use mumquiz::html;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

const WAIT: Duration = Duration::from_secs(20);

const QUESTION: &str = "Wie heisst unser Stern?";
const ANSWER: &str = "Sonne";
const BANK: &str = "tests/live-bank.toml";
/// A quiet moment, used for the checks that expect nothing.
const QUIET: Duration = Duration::from_secs(4);

fn host() -> String {
    let value = env::var("MUMQUIZ_TEST_HOST").unwrap_or_else(|_| "127.0.0.1".to_owned());
    assert!(
        matches!(value.as_str(), "127.0.0.1" | "localhost" | "::1"),
        "the live tests may only talk to the local test server, got {value}"
    );
    value
}

fn port() -> u16 {
    env::var("MUMQUIZ_TEST_PORT")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(64738)
}

/// Password from the environment, else from the test environment's `.env` (the mumbot convention).
fn testenv_var(name: &str) -> Option<String> {
    if let Some(value) = env::var(name).ok().filter(|value| !value.is_empty()) {
        return Some(value);
    }
    let dir = env::var("MUMQUIZ_TESTENV_DIR").unwrap_or_else(|_| "testenv".to_owned());
    let text = std::fs::read_to_string(Path::new(&dir).join(".env")).ok()?;
    for line in text.lines() {
        if let Some((key, value)) = line.split_once('=')
            && key == name
            && !value.is_empty()
        {
            return Some(value.to_owned());
        }
    }
    None
}

struct Game {
    data: PathBuf,
    config: Config,
    bot: mpsc::Sender<Command>,
    runner: JoinHandle<Result<()>>,
    player: mumcord_protocol::Connection,
    channel: u32,
    /// Chat the player saw while waiting for the bot to appear. `message` looks here first, so a
    /// line the bot wrote during the wait is not lost.
    seen: Vec<String>,
}

fn client(config: &Config, username: &str) -> ConnectConfig {
    ConnectConfig {
        host: config.server.host.clone(),
        port: config.server.port,
        username: username.to_owned(),
        password: None,
        tokens: Vec::new(),
        identity: None,
        // The local test server is self-signed.
        verify_cert: Arc::new(|_| true),
        login_timeout: WAIT,
    }
}

impl Game {
    fn say(&self, text: &str) {
        self.player
            .handle
            .send_text(&TextTarget::channel(self.channel), text)
            .expect("send");
    }

    /// The next event matching `pred`, skipping everything else.
    async fn event(&mut self, pred: impl Fn(&Event) -> bool, what: &str) -> Event {
        let deadline = tokio::time::Instant::now() + WAIT;
        loop {
            let event = tokio::time::timeout_at(deadline, self.player.events.recv())
                .await
                .unwrap_or_else(|_| panic!("no {what} within {WAIT:?}"));
            match event {
                Some(event) if pred(&event) => return event,
                Some(_) => {}
                None => panic!("the player lost the connection while waiting for {what}"),
            }
        }
    }

    /// Nothing matching `pred` may arrive in the quiet window.
    async fn quiet(&mut self, pred: impl Fn(&Event) -> bool, what: &str) {
        let deadline = tokio::time::Instant::now() + QUIET;
        loop {
            match tokio::time::timeout_at(deadline, self.player.events.recv()).await {
                Ok(Some(event)) => {
                    if pred(&event) {
                        panic!("{what}: unexpected {event:?}");
                    }
                }
                Ok(None) => panic!("the player lost the connection"),
                Err(_) => return,
            }
        }
    }

    async fn message(&mut self, pred: impl Fn(&str) -> bool, what: &str) -> String {
        if let Some(text) = self.seen.iter().find(|text| pred(text)) {
            return text.clone();
        }
        let event = self.event(|event| text_matches(event, &pred), what).await;
        let Event::TextMessage(chat) = event else {
            panic!("expected a chat message");
        };
        html::to_text(&chat.message)
    }

    /// A message written to the player, not to the channel.
    async fn whisper(&mut self, pred: impl Fn(&str) -> bool, what: &str) -> String {
        let own = self.player.handle.with_state(|state| state.session);
        let event = self
            .event(
                |event| {
                    matches!(event, Event::TextMessage(chat)
                        if chat.target.sessions.iter().any(|session| Some(*session) == own)
                        && pred(&html::to_text(&chat.message)))
                },
                what,
            )
            .await;
        let Event::TextMessage(chat) = event else {
            panic!("expected a private chat message");
        };
        html::to_text(&chat.message)
    }

    async fn stop(self) {
        self.bot.send(Command::Stop).await.expect("stop");
        let result = tokio::time::timeout(Duration::from_secs(10), self.runner)
            .await
            .expect("the bot did not close within 10 s");
        result.expect("bot run").expect("run");
    }
}

fn text_matches(event: &Event, pred: &impl Fn(&str) -> bool) -> bool {
    matches!(event, Event::TextMessage(chat) if pred(&html::to_text(&chat.message)))
}

fn rules(temp: &Path) -> QuizRules {
    QuizRules {
        questions: PathBuf::from(BANK),
        hiscores: temp.join("hiscores.sqlite"),
        // The bot is never a player, so one person is enough to start the game.
        min_players: 1,
        hint_patience_secs: 5,
        answer_gap_secs: 1,
        tick_secs: 1,
        cards: false,
        masters: vec!["MumquizTest".to_owned()],
        ..QuizRules::default()
    }
}

/// Starts the bot with its data in `data` (certificate and high score file), and connects one player
/// as `Tester`. The player listens first, so it also sees what the bot writes right after its login.
/// The caller owns `data`, so a test can look at the files after the bot stops.
async fn start_in(data: &Path, with: impl FnOnce(&mut QuizRules), superuser: bool) -> Game {
    let mut quiz = rules(data);
    with(&mut quiz);
    let config = Config {
        server: Server {
            host: host(),
            port: port(),
            name: if superuser {
                "SuperUser".to_owned()
            } else {
                "MumquizTest".to_owned()
            },
            cert_name: Some("mumquiz-test".to_owned()),
            // No channel configured: the bot plays in the channel it joins at login, the root.
            channel: None,
            password: if superuser {
                testenv_var("SUPERUSER_PASSWORD")
            } else {
                None
            },
            data_dir: Some(data.to_path_buf()),
            superuser,
            register: false,
            // The test server is self-signed, and this is a local test only.
            pin: false,
        },
        quiz,
        tokens: Vec::new(),
        strings: BTreeMap::new(),
    };

    let mut player = connect(client(&config, "Tester"))
        .await
        .expect("player login");
    // The root channel is 0 on every standard server; murmur names it after the server.
    let channel = player
        .handle
        .with_state(|state| state.channels.get(&0).map(|channel| channel.id))
        .expect("the root channel");
    player.handle.join_channel(channel).expect("join");

    let mut bot = Bot::new(config.clone()).expect("bot");
    let (sender, mut commands) = mpsc::channel::<Command>(8);
    let runner = tokio::spawn(async move { bot.run(&mut commands).await });

    // The bot logs in in the background: wait until the player sees it, so a message the test sends
    // is not lost before the login.
    let bot_name = config.server.name.clone();
    let deadline = tokio::time::Instant::now() + WAIT;
    let mut seen = Vec::new();
    loop {
        if player
            .handle
            .with_state(|state| state.users.values().any(|user| user.name == bot_name))
        {
            break;
        }
        if tokio::time::Instant::now() >= deadline {
            panic!("the bot did not join within {WAIT:?}");
        }
        let event = tokio::time::timeout(Duration::from_secs(1), player.events.recv())
            .await
            .unwrap_or_else(|_| panic!("the bot did not join within {WAIT:?}"));
        if let Some(Event::TextMessage(chat)) = event {
            seen.push(html::to_text(&chat.message));
        }
    }

    Game {
        data: data.to_path_buf(),
        config,
        bot: sender,
        runner,
        player,
        channel,
        seen,
    }
}

#[tokio::test]
#[ignore = "needs the local test server: testenv/up.sh"]
async fn game_over_the_wire() {
    let temp = tempfile::tempdir().expect("temp dir");
    let mut game = start_in(temp.path(), |_| {}, false).await;

    // Question, answer, point, winner, and the win in the high score table.
    let asked = game
        .message(|text| text.contains(QUESTION), "question")
        .await;
    assert!(asked.contains(QUESTION), "{asked}");

    game.say(&format!("ich rate: {ANSWER}"));
    let right = game
        .message(|text| text.contains("is right"), "correct answer")
        .await;
    assert!(right.contains(ANSWER), "{right}");
    let won = game.message(|text| text.contains("winner"), "winner").await;
    assert!(won.contains("Tester"), "{won}");

    game.say("!hiscore");
    let board = game
        .message(|text| text.contains("1. Tester: 1 points"), "hiscore")
        .await;
    assert!(!board.is_empty(), "{board}");

    // A player who is not a master may not use a master command.
    game.say("!reload");
    let refused = game
        .message(|text| text.contains("not on my watch"), "refusal")
        .await;
    assert!(refused.contains("Tester"), "{refused}");

    // The bot wrote its own high score file.
    assert!(game.data.join("hiscores.sqlite").exists());

    game.stop().await;
}

#[tokio::test]
#[ignore = "needs the local test server: testenv/up.sh"]
async fn chat_html_from_a_client_is_read_as_plain_text() {
    let temp = tempfile::tempdir().expect("temp dir");
    let mut game = start_in(temp.path(), |_| {}, false).await;
    game.message(|text| text.contains(QUESTION), "question")
        .await;
    game.say("<b>Sonne</b> <i>vermutlich</i>");
    let right = game
        .message(|text| text.contains("is right"), "correct answer")
        .await;
    assert!(right.contains("Sonne"), "{right}");
    game.stop().await;
}

#[tokio::test]
#[ignore = "needs the local test server: testenv/up.sh"]
async fn help_reaches_a_player_privately() {
    let temp = tempfile::tempdir().expect("temp dir");
    let mut game = start_in(temp.path(), |_| {}, false).await;
    game.say("!help");
    let text = game
        .whisper(|text| text.contains("quiz channel"), "private help")
        .await;
    assert!(!text.is_empty(), "{text}");
    game.stop().await;
}

#[tokio::test]
#[ignore = "needs the local test server: testenv/up.sh"]
async fn a_listener_counts_as_a_player() {
    let temp = tempfile::tempdir().expect("temp dir");
    let mut game = start_in(temp.path(), |quiz| quiz.min_players = 2, false).await;

    // One person is not enough: the bot waits, and the second one only follows the channel.
    game.message(|text| text.contains("Playing alone"), "waiting")
        .await;

    let listener = connect(client(&game.config, "Listener"))
        .await
        .expect("listener login");
    listener.handle.listen(&[game.channel]).expect("listen");

    game.message(|text| text.contains(QUESTION), "question")
        .await;
    game.stop().await;
}

#[tokio::test]
#[ignore = "needs the local test server: testenv/up.sh"]
async fn cards_are_sent_when_configured() {
    let temp = tempfile::tempdir().expect("temp dir");
    let mut game = start_in(temp.path(), |quiz| quiz.cards = true, false).await;
    let event = game
        .event(
            |event| {
                matches!(event, Event::TextMessage(chat)
                    if chat.message.contains("data-mumcord-card")
                        && html::to_text(&chat.message).contains(QUESTION))
            },
            "card question",
        )
        .await;
    let Event::TextMessage(chat) = event else {
        panic!("expected a chat message");
    };
    // The wire format Mumcord renders, with the fallback an official client shows.
    assert!(
        chat.message
            .starts_with("<div data-mumcord-card=\"v=1&amp;title=Question"),
        "{chat:?}"
    );
    game.stop().await;
}

#[tokio::test]
#[ignore = "needs the local test server: testenv/up.sh"]
async fn score_and_hiscore_answer_even_with_an_empty_board() {
    let temp = tempfile::tempdir().expect("temp dir");
    let mut game = start_in(temp.path(), |_| {}, false).await;
    game.message(|text| text.contains(QUESTION), "question")
        .await;

    game.say("!score");
    let board = game
        .message(|text| text.contains("no points yet"), "empty score board")
        .await;
    assert!(!board.is_empty(), "{board}");

    game.say("!hiscore");
    let top = game
        .message(
            |text| text.contains("no wins yet"),
            "empty high score board",
        )
        .await;
    assert!(!top.is_empty(), "{top}");

    game.say(&format!("ich rate: {ANSWER}"));
    let awarded = game
        .message(|text| text.contains("is right"), "answer")
        .await;
    assert!(awarded.contains(ANSWER), "{awarded}");

    // One player, so the target score is one: the round ends and the current points are zeroed.
    game.say("!score");
    let board = game
        .message(
            |text| text.contains("no points yet"),
            "score board after the win",
        )
        .await;
    assert!(!board.is_empty(), "{board}");

    // The all-time board now has the win, as one message.
    game.say("!hiscore");
    let top = game
        .message(|text| text.contains("1. Tester"), "high score board")
        .await;
    assert!(!top.is_empty(), "{top}");
    game.stop().await;
}

#[tokio::test]
#[ignore = "needs the local test server: testenv/up.sh"]
async fn stop_and_start_pause_the_quiz() {
    let temp = tempfile::tempdir().expect("temp dir");
    // The player is a master here, so the chat command is its own.
    let mut game = start_in(
        temp.path(),
        |quiz| quiz.masters = vec!["Tester".to_owned()],
        false,
    )
    .await;
    game.message(|text| text.contains(QUESTION), "question")
        .await;

    game.say("!stop");
    let stopped = game
        .message(|text| text.contains("quiz stopped"), "stop")
        .await;
    assert!(!stopped.is_empty(), "{stopped}");

    // An answer counts nothing during the pause, and no question is asked.
    game.say(&format!("ich rate: {ANSWER}"));
    let right = |event: &Event| matches!(event, Event::TextMessage(chat) if html::to_text(&chat.message).contains("is right"));
    game.quiet(right, "an answer during the pause").await;

    game.say("!start");
    game.message(|text| text.contains("quiz started"), "start")
        .await;
    game.message(|text| text.contains(QUESTION), "question after the pause")
        .await;
    game.say(&format!("ich rate: {ANSWER}"));
    let awarded = game
        .message(|text| text.contains("is right"), "answer after the pause")
        .await;
    assert!(awarded.contains(ANSWER), "{awarded}");
    game.stop().await;
}

#[tokio::test]
#[ignore = "needs the local test server: testenv/up.sh"]
async fn the_high_scores_survive_a_restart() {
    let temp = tempfile::tempdir().expect("temp dir");

    // One round: the single correct answer wins it because the target score for one player is one.
    let mut game = start_in(temp.path(), |_| {}, false).await;
    game.message(|text| text.contains(QUESTION), "question")
        .await;
    game.say(&format!("ich rate: {ANSWER}"));
    let won = game.message(|text| text.contains("winner"), "winner").await;
    assert!(won.contains("Tester"), "{won}");
    game.stop().await;

    // The same data folder, a new process: the win is still there, the round starts fresh.
    // The same data folder, a new process: the win is still in the file.
    let mut game = start_in(temp.path(), |_| {}, false).await;
    game.say("!hiscore");
    let board = game
        .message(|text| text.contains("1. Tester: 1 points"), "hiscore")
        .await;
    assert!(!board.is_empty(), "{board}");
    game.stop().await;
}

#[tokio::test]
#[ignore = "needs the local test server: testenv/up.sh"]
async fn the_channel_description_needs_the_write_permission() {
    // With the admin login of the test server the bot has the channel's Write permission and writes
    // the topic. Without it the bot plays and leaves the description alone; that decision is tested
    // in tests/gates.rs, because a denied request never reaches the other clients.
    if testenv_var("SUPERUSER_PASSWORD").is_none() {
        panic!("SUPERUSER_PASSWORD is not set (environment or testenv/.env)");
    }
    let temp = tempfile::tempdir().expect("temp dir");
    let mut game = start_in(temp.path(), |_| {}, true).await;
    let event = game
        .event(
            |event| {
                matches!(event, Event::ChannelUpdated(channel)
                    if channel.description.as_deref().unwrap_or("").contains("target score"))
            },
            "channel description",
        )
        .await;
    let Event::ChannelUpdated(channel) = &event else {
        panic!("expected a channel update");
    };
    assert!(
        channel
            .description
            .as_deref()
            .unwrap_or("")
            .contains("target score"),
        "{channel:?}"
    );
    game.stop().await;
}
