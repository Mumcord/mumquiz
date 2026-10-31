# Configuration reference

`bot.toml` is read once at startup. An unknown key or section is an error, so a typo is reported
instead of ignored. The file holds names and possibly a password: keep it private, it is
git-ignored.

```sh
mumquiz --config bot.toml
MUMQUIZ_CONFIG=bot.toml mumquiz
```

## `[server]`: the Mumble server and the bot's identity

| Key | Type | Default | Environment | Meaning |
| --- | --- | --- | --- | --- |
| `host` | string | `127.0.0.1` | | Server address (name or IP). |
| `port` | integer | `64738` | | Server port. |
| `name` | string | `Mumquiz` | `MUMQUIZ_NAME` | Name the players see. |
| `cert_name` | string | `name`, with anything outside `A-Z a-z 0-9 - _` replaced by `_` | | Name of the persistent client certificate. Keep it stable to keep the bot's identity. |
| `channel` | string, optional | none | `MUMQUIZ_CHANNEL` | Channel to join at login, by name (`Quiz`) or path (`Root/Quiz`). Without it the bot plays in the channel the server puts it in at login. |
| `password` | string | none | `SERVER_PASSWORD` | Connect password. Prefer the environment variable so the file holds no secret. |
| `data_dir` | path | `$XDG_DATA_HOME/mumquiz` | `MUMQUIZ_DATA` | Certificate folder and pinned fingerprints. |
| `superuser` | boolean | `false` | `SUPERUSER_PASSWORD` | Log in as `SuperUser` without a certificate. Local test servers only. |
| `register` | boolean | `false` | | Register the certificate at login so the server knows the bot as a user; needs `Self-Register` or the `Register` right from an admin. Without it the bot is an anonymous client. |
| `pin` | boolean | `true` | | Trust the server certificate by its pinned SHA-256 fingerprint (trust on first use). The first connection stores the pin, later ones must see the same certificate; a change is refused and logged. `pin = false` accepts any certificate — local test servers only, and the bot warns about it. |

The bot writes the channel description when a round starts, so it needs the channel's `Write`
permission. Without it, the bot still plays and logs the denial.

## `token` (`tokens`): channel passwords, one block per channel

```toml
[[token]]
channel = "Root/Quiz"
password = "change-me-channel-password"
```

| Key | Type | Meaning |
| --- | --- | --- |
| `channel` | string, required | The channel the password belongs to, by name or by path. |
| `password` | string, required | Never logged, never committed. `[[tokens]]` is accepted as well. |

Every password is presented to the server at login as a Mumble access token, so a channel whose ACL
uses that token can be entered. The bot checks the server's effective permissions before it joins a
channel that is enter-restricted and says so in the log when no password was given for it; the
decision itself is the gate test in `tests/gates.rs`. A password only helps when the server's ACL
matches it, so `!join Lobby` into a locked channel still needs the right `[[token]]` block.

## `[quiz]`: the game

| Key | Type | Default | Meaning |
| --- | --- | --- | --- |
| `questions` | path | `questions.toml` | Question bank, one `[[question]]` block per question. |
| `hiscores` | path | `quiz-hiscores.sqlite` | High score database, created on first use. |
| `min_players` | integer, above 0 | `2` | Below this many players the bot waits. The bot itself never counts. |
| `hint_patience_secs` | integer, at least 5 | `20` | Seconds a question may stay unanswered before the bot hints. |
| `stamina` | integer, above 0 | `12` | Questions before the bot gets hungry and needs a `!botsnack`. |
| `recycle_percent` | integer, 1–100 | `80` | Share of the bank kept as recently asked questions. |
| `answer_gap_secs` | integer, above 0 | `5` | Seconds between an answer and the next question. |
| `tick_secs` | integer, above 0 | `1` | Seconds between two game ticks. |
| `waiting_secs` | integer, above 0 | `30` | Seconds between two "waiting for players" messages. |
| `command_prefix` | string, not empty | `!` | Command prefix. |
| `masters` | list of names | none | Names allowed to use `!reload`, `!join`, `!op`, `!deop`. The bot itself is always a master. |
| `keep_score_on_rename` | boolean | `true` | Keep a player's score when their name changes. |
| `cards` | boolean | `true` | Send game messages as Mumcord message cards. |
| `verbose` | boolean | `false` | Log every question asked and every command seen. |

## `[strings]`: the messages

Every message is a template with placeholders. See [strings.md](strings.md). Unknown keys are an
error. `help` is one key with several lines, separated by newlines.

## Environment variables

| Variable | Meaning |
| --- | --- |
| `MUMQUIZ_CONFIG` | Config file when `--config` is not given. |
| `MUMQUIZ_QUESTIONS` | Question bank, overriding `quiz.questions`. |
| `MUMQUIZ_DATA` | Certificate folder, overriding `server.data_dir`. |
| `MUMQUIZ_NAME`, `MUMQUIZ_CHANNEL` | Name and channel, overriding `[server]`. |
| `SERVER_PASSWORD` | Connect password when `server.password` is not set. Empty counts as unset. |
| `SUPERUSER_PASSWORD` | Password for `superuser = true`. |
| `RUST_LOG` | `error`, `warn`, `info` (default) or `debug`. |

SIGHUP reads the question bank again; Ctrl-C sends what is still queued and closes.

## A complete example

```toml
[server]
host = "mumble.example.org"
name = "q"
channel = "Root/Quiz"
cert_name = "q"

[[token]]
channel = "Root/Quiz"
password = "REPLACE-WITH-THE-CHANNEL-PASSWORD"

[quiz]
questions = "questions.toml"
hiscores = "quiz-hiscores.sqlite"
min_players = 3
hint_patience_secs = 25
stamina = 6
masters = ["barbara"]

[strings]
botsnack = "Jetzt bin ich hungrig. Fuetter mich mit !botsnack damit es weitergeht"
```

The connect password comes from `SERVER_PASSWORD` in the environment.
