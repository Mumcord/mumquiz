//! Reconnect gates: the waiting sequence and the loop around a failed login. No server needed.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::Duration;

use mumcord_protocol::DisconnectReason;
use mumquiz::bot::{Bot, Command, backoff, reconnect_delay};
use mumquiz::config::{Config, QuizRules, Server};
use tokio::sync::mpsc;

#[test]
fn the_reconnect_wait_grows_to_a_minute() {
    assert_eq!(backoff(1), Duration::from_secs(1));
    assert_eq!(backoff(2), Duration::from_secs(2));
    assert_eq!(backoff(3), Duration::from_secs(4));
    assert_eq!(backoff(6), Duration::from_secs(32));
    assert_eq!(backoff(7), Duration::from_secs(60));
    assert_eq!(backoff(50), Duration::from_secs(60));
}

#[test]
fn a_kick_waits_the_full_minute() {
    // Another instance with the same nick, or an admin: fighting is pointless, and every new
    // attempt counts against the server's autoban.
    let kick = DisconnectReason::Kicked {
        actor: None,
        reason: Some("You connected to the server from another device".to_owned()),
        ban: false,
    };
    assert_eq!(reconnect_delay(1, &kick), Duration::from_secs(60));
    assert_eq!(
        reconnect_delay(1, &DisconnectReason::Closed),
        Duration::from_secs(1)
    );
    assert_eq!(
        reconnect_delay(4, &DisconnectReason::Timeout),
        Duration::from_secs(8)
    );
    assert_eq!(
        reconnect_delay(10, &DisconnectReason::Error("reset".to_owned())),
        Duration::from_secs(60)
    );
}

#[tokio::test]
async fn a_failed_login_does_not_end_the_loop() {
    // Nothing listens on loopback port 9: the login fails, the loop retries, and Stop ends it.
    let temp = tempfile::tempdir().expect("temp dir");
    let config = Config {
        server: Server {
            host: "127.0.0.1".to_owned(),
            port: 9,
            name: "MumquizTest".to_owned(),
            // The bot plays in the channel it is put in at login, so none is configured.
            data_dir: Some(temp.path().to_path_buf()),
            register: false,
            pin: false,
            ..Server::default()
        },
        quiz: QuizRules {
            questions: PathBuf::from("tests/live-bank.toml"),
            hiscores: temp.path().join("hiscores.sqlite"),
            ..QuizRules::default()
        },
        tokens: Vec::new(),
        strings: BTreeMap::new(),
    };
    let mut bot = Bot::new(config).expect("bot");
    let (sender, mut commands) = mpsc::channel::<Command>(4);
    let runner = tokio::spawn(async move { bot.run(&mut commands).await });

    // Past the first wait: the loop is still running, so a second attempt has been made.
    tokio::time::sleep(Duration::from_millis(1500)).await;
    sender.send(Command::Stop).await.expect("stop");

    let result = tokio::time::timeout(Duration::from_secs(10), runner)
        .await
        .expect("the loop did not stop after the failed login");
    result.expect("join").expect("run");
}

#[test]
fn the_channel_permission_decides_the_topic() {
    use mumquiz::bot::may_write;

    let mut effective = std::collections::BTreeMap::new();
    assert!(!may_write(&effective, 1), "nothing known yet");
    effective.insert(1, 0x208); // Traverse + Text-Message
    assert!(!may_write(&effective, 1), "chat without Write");
    effective.insert(1, 0x201); // Write + Text-Message
    assert!(
        may_write(&effective, 1),
        "Write allows the channel description"
    );
}
