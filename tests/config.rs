//! `bot.toml`: defaults, and a typo must be an error.

use mumquiz::config::Config;
use mumquiz::strings::Strings;

#[test]
fn an_empty_config_uses_the_upstream_defaults() {
    let config = Config::parse("").expect("empty config");
    assert_eq!(config.server.port, 64738);
    assert_eq!(config.server.name, "Mumquiz");
    assert_eq!(config.server.channel, None);
    assert_eq!(config.quiz.min_players, 2);
    assert_eq!(config.quiz.hint_patience_secs, 20);
    assert_eq!(config.quiz.stamina, 12);
    assert_eq!(config.quiz.recycle_percent, 80);
    assert_eq!(config.quiz.command_prefix, "!");
    assert!(!config.server.register, "anonymous by default");
}

#[test]
fn a_known_config_reads_every_rule() {
    let config = Config::parse(
        r#"
[server]
host = "localhost"
name = "q"
cert_name = "q"
channel = "Root/Quiz"
register = true

[[token]]
channel = "Root/Quiz"
password = "change-me-channel-password"

[quiz]
questions = "data/questions.example.toml"
hiscores = "/tmp/hiscores.sqlite"
min_players = 3
hint_patience_secs = 25
stamina = 6
recycle_percent = 40
masters = ["barbara"]

[strings]
botsnack = "Jetzt bin ich hungrig. Fuetter mich mit !botsnack."
"#,
    )
    .expect("config");
    // A name is a host too, not only an address.
    assert_eq!(config.server.host, "localhost");
    assert_eq!(config.quiz.min_players, 3);
    assert_eq!(config.quiz.stamina, 6);
    assert_eq!(config.quiz.masters, ["barbara"]);
    assert_eq!(config.server.channel, Some("Root/Quiz".to_owned()));
    assert!(config.server.register);
    assert_eq!(config.tokens.len(), 1);
    assert_eq!(config.tokens[0].channel, "Root/Quiz");
    // The bot presents the password as a Mumble access token at login.
    assert_eq!(
        mumquiz::config::Config::parse("[[token]]\nchannel = \"Quiz\"\npassword = \"change-me\"\n")
            .expect("token")
            .tokens[0]
            .password,
        "change-me"
    );
    assert_eq!(config.masters(), ["barbara", "q"]);
    let strings = Strings::with_overrides(&config.strings).expect("strings");
    assert!(strings.botsnack.contains("hungrig"));
    assert_eq!(strings.waiting, Strings::default().waiting);
}

#[test]
fn an_unknown_key_is_reported() {
    let error = Config::parse("[quiz]\nhint_patience = 20").expect_err("unknown key");
    assert!(error.to_string().contains("hint_patience"), "{error}");
    let config = Config::parse("[strings]\nunknown_line = \"hi\"").expect("config");
    let error = Strings::with_overrides(&config.strings).expect_err("no such message");
    assert!(error.to_string().contains("unknown_line"), "{error}");
}

#[test]
fn impossible_rules_are_rejected() {
    for text in [
        "[quiz]\nmin_players = 0",
        "[quiz]\nhint_patience_secs = 4",
        "[quiz]\nstamina = 0",
        "[quiz]\nrecycle_percent = 0",
        "[quiz]\nrecycle_percent = 101",
        "[quiz]\ntick_secs = 0",
        "[quiz]\ncommand_prefix = \"\"",
        "[server]\nname = \"\"",
    ] {
        let error = Config::parse(text).expect_err("invalid rule");
        assert!(
            error.to_string().contains("must be") || error.to_string().contains("empty"),
            "{error}"
        );
    }
}
