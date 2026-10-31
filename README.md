<div align="center">

<img src="assets/mumquiz-banner.svg" alt="mumquiz: a quiz bot for Mumble servers" width="560">

**Quiz night in your Mumble channel: the bot asks, everyone types, the fastest right answer scores.**

[![CI](https://img.shields.io/github/actions/workflow/status/Mumcord/mumquiz/ci.yml?branch=main&label=CI&logo=github)](https://github.com/Mumcord/mumquiz/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/Mumcord/mumquiz?include_prereleases&sort=semver&logo=github)](https://github.com/Mumcord/mumquiz/releases)
[![Licence](https://img.shields.io/badge/licence-AGPL--3.0--only-8b6cff)](#licence)
[![Rust 1.90+](https://img.shields.io/badge/rust-1.90%2B-b7410e?logo=rust)](Cargo.toml)
[![mumcord-protocol](https://img.shields.io/crates/v/mumcord-protocol?label=mumcord-protocol&logo=rust)](https://crates.io/crates/mumcord-protocol)

[Quick start](#quick-start) · [Commands](#what-the-bot-does) · [Docs](#documentation) · [mumcord.app/en/mumquiz](https://mumcord.app/en/mumquiz)

</div>

## What is mumquiz?

A quiz bot for [Mumble](https://mumble.info/) servers: it asks questions, reads the answers in chat,
gives hints, awards points and remembers the wins. It is the game of
[zorgch/irc-quizbot](https://github.com/zorgch/irc-quizbot) ported to Rust and to the Mumble
protocol, using [mumcord-protocol](https://crates.io/crates/mumcord-protocol) — the same protocol
layer as the [Mumcord](https://github.com/Mumcord/mumcord) client.

The bot logs in like a person: its own certificate, normal login, normal chat. So it works with an
unmodified standard Mumble server. No server plugin, no server config change, no protocol
extension.

## Quick start

```sh
cp examples/bot.example.toml bot.toml        # bot.toml is git-ignored: it holds names and secrets
cp data/questions.example.toml questions.toml
cargo build --release
./target/release/mumquiz
```

Join the channel the bot is configured for (`Quiz`), say `!help`, and the game starts as soon as
two players are there.

```sh
mumquiz --config bot.toml --questions questions.toml
```

## What the bot does

| Command | Meaning |
| --- | --- |
| (a message that contains the answer) | the player scores a point |
| `!help [name]` | the help, privately, to a player |
| `!score` | top five of this round |
| `!hiscore` | top five of all time |
| `!botsnack` | feed the bot: it stops being hungry |
| `!reload` | read the question bank again (master) |
| `!join [channel]` | join another channel (master) |
| `!op <name>` / `!deop <name>` | add or remove a master (master) |

The rules are the upstream bot's: it waits for `min_players` before it starts, gives a hint after
`hint_patience_secs`, reveals a few letters per hint, gives up after five hints, and needs a
`!botsnack` after `stamina` questions. The target score of a round is `1 + players / 2`.

In Mumcord the game messages show as message cards; in an official Mumble client they show as plain
chat text. Set `quiz.cards = false` for plain text in Mumcord too.

## Where IRC and Mumble differ

- **Players** are the people in the bot's channel, or following it with Listen. Mumble has no
  `NAMES` list, so the bot uses the server state it already has.
- **Channel topic** → the channel description. The bot writes it when the round starts, when the
  server allows it.
- **Ops, ChanServ, NickServ** do not exist for a client. `!op` and `!deop` only change the bot's
  own master list; a real channel ACL stays with the server admin. The bot cannot kick: `!`
  commands from a non-master are answered with the "not on my watch" line, and that is all it may do.
- **Renames**: a Mumble user keeps their session, so the bot maps the name change and keeps the
  score (`quiz.keep_score_on_rename`).
- **Reconnect**: the upstream bot reconnected immediately; this one waits with a growing delay
  (1 s to 60 s) so a broken server is not hammered.
- **Answers** are matched loosely: case, punctuation and the spellings `ae`/`ä`, `oe`/`ö`,
  `ue`/`ü`, `ss`/`ß` count as the same. A short answer must be its own word.

## Documentation

| | |
| --- | --- |
| [Configuration](docs/configuration.md) | every `bot.toml` key, default and environment variable |
| [Commands](docs/commands.md) | what players can type, and who is a master |
| [Game rules](docs/rules.md) | waiting, hunger, hints, points, wins |
| [Strings](docs/strings.md) | the message templates and their placeholders |
| [Question bank](docs/questions.md) | the TOML format and the importer for the upstream bank |
| [Keeping the bot connected](docs/connection.md) | the reconnect wait, the log lines, the service unit |

## Tests and gates

```sh
cargo test                 # game, answers, config, storage, chat HTML, documentation gates
scripts/smoke/smoke.sh     # the bot binary and the whole bot against the local test server
```

The live suite (`tests/live.rs`) is ignored by default and runs against `testenv/` only, never
against a public server. CI runs the gates: format, lint, tests, actionlint and shellcheck, secrets,
loopback, headless, documentation, changelog, dependencies. The list, how to run each one locally and
what to do when one fails is in [CONTRIBUTING.md](CONTRIBUTING.md).

## Part of the Mumcord family

| | Project | What it is |
| --- | --- | --- |
| <img src="https://raw.githubusercontent.com/Mumcord/mumcord/main/src-tauri/icons/128x128.png" width="28" alt=""> | [Mumcord](https://github.com/Mumcord/mumcord) | A modern, Discord-style desktop client for any Mumble server |
| <img src="https://raw.githubusercontent.com/Mumcord/mumbot/main/docs/assets/icon/mumbot-64.png" width="28" alt=""> | [Mumbot](https://github.com/Mumcord/mumbot) | Headless bots for Mumble servers: webhooks, tests, the relay bot |
| <img src="https://raw.githubusercontent.com/Mumcord/mumpeer/main/assets/png/mumpeer-64.png" width="28" alt=""> | [mumpeer](https://github.com/Mumcord/mumpeer) | The relay for Mumcord's peer-to-peer features |
| <img src="assets/png/mumquiz-64.png" width="28" alt=""> | **mumquiz** | This quiz bot |

## Licence

AGPL-3.0-only, like the rest of Mumcord. The game itself and the wording of the messages come from
[zorgch/irc-quizbot](https://github.com/zorgch/irc-quizbot) (GPL-3.0); a question bank imported with
`scripts/import-upstream-questions.sh` keeps that licence and stays yours to edit.
