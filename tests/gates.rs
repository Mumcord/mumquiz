//! Gates that run as tests: the documentation must cover what the code accepts, and the repository
//! must stay free of secrets. The CI job "Gates" runs the same checks as shell scripts.

use std::collections::BTreeSet;

fn read(path: &str) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|error| panic!("reading {path}: {error}"))
}

/// Quoted names in the block that starts with `marker` (the arms of a match).
fn quoted_names(source: &str, marker: &str) -> BTreeSet<String> {
    let start = source
        .find(marker)
        .unwrap_or_else(|| panic!("no {marker} in the source"));
    let rest = &source[start + marker.len()..];
    let mut out = BTreeSet::new();
    // Only the arms of the match itself, at the base depth of the block.
    let mut depth = 0;
    for line in rest.lines() {
        let trimmed = line.trim();
        if depth == 0 {
            if trimmed == "}" {
                break;
            }
            if trimmed.starts_with('"') && trimmed.contains("=>") {
                let end = trimmed[1..].find('"').unwrap_or(0);
                out.insert(trimmed[1..1 + end].to_owned());
            }
        }
        depth += trimmed.matches('{').count();
        depth = depth.saturating_sub(trimmed.matches('}').count());
    }
    out
}

/// Field names of a struct.
fn fields(source: &str, marker: &str) -> BTreeSet<String> {
    let start = source
        .find(marker)
        .unwrap_or_else(|| panic!("no {marker} in the source"));
    let rest = &source[start + marker.len()..];
    let mut out = BTreeSet::new();
    for line in rest.lines() {
        let trimmed = line.trim();
        if trimmed == "}" {
            break;
        }
        if let Some(rest_of_line) = trimmed.strip_prefix("pub ")
            && let Some((name, _)) = rest_of_line.split_once(':')
        {
            out.insert(name.to_owned());
        }
    }
    out
}

#[test]
fn every_config_key_is_documented() {
    let source = read("src/config.rs");
    let docs = read("docs/configuration.md");
    for marker in [
        "pub struct Config {",
        "pub struct Server {",
        "pub struct QuizRules {",
        "pub struct Token {",
    ] {
        for name in fields(&source, marker) {
            // A key of a section is written as `key`, the section itself as `[key]`.
            assert!(
                docs.contains(&format!("`{name}`")) || docs.contains(&format!("`[{name}]`")),
                "{name} of {marker} is missing in docs/configuration.md"
            );
        }
    }
}

#[test]
fn every_message_key_is_documented() {
    let source = read("src/strings.rs");
    let docs = read("docs/strings.md");
    let keys = quoted_names(&source, "match key.as_str() {");
    assert!(
        keys.contains("help"),
        "the help lines are a key of their own"
    );
    for key in keys {
        assert!(
            docs.contains(&format!("`{key}`")),
            "{key} is missing in docs/strings.md"
        );
    }
}

#[test]
fn every_command_is_documented() {
    let source = read("src/quiz.rs");
    let docs = read("docs/commands.md");
    let commands = quoted_names(&source, "match command.as_str() {");
    assert_eq!(
        commands,
        [
            "help", "score", "hiscore", "botsnack", "reload", "join", "op", "deop", "stop", "start"
        ]
        .into_iter()
        .map(String::from)
        .collect::<BTreeSet<String>>()
    );
    for command in commands {
        assert!(
            docs.contains(&format!("`!{command}")),
            "!{command} is missing in docs/commands.md"
        );
    }
}

#[test]
fn the_repository_keeps_secrets_and_data_out() {
    let gitignore = read(".gitignore");
    for entry in ["bot.toml", ".env", "*.sqlite", "target/"] {
        assert!(gitignore.contains(entry), "{entry} must be git-ignored");
    }
    // The example config must hold placeholders only, never a value that could be real.
    let example = read("examples/bot.example.toml");
    assert!(
        example.contains("Copy to bot.toml"),
        "the example must say it is a starting point"
    );
    for line in example.lines() {
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        if !["key", "password", "token", "secret"].contains(&key.trim()) {
            continue;
        }
        let value = value.trim().trim_matches('"');
        assert!(value.is_empty() || value.starts_with("change-me"), "{line}");
    }
    assert!(read("testenv/.env.example").contains("change-me"));
}

#[test]
fn the_live_tests_only_use_the_local_test_server() {
    let live = read("tests/live.rs");
    assert!(
        live.contains("127.0.0.1"),
        "the default host must be loopback"
    );
    assert!(
        live.contains("may only talk to the local test server"),
        "the guard must stay"
    );
    let compose = read("testenv/docker-compose.yml");
    assert!(
        compose.contains("127.0.0.1:"),
        "publish the test server on loopback only"
    );
}
