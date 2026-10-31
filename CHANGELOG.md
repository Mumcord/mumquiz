# Changelog

## [Unreleased]

Internal: the pipeline now has the weekly dependency audit as its own workflow, the reusable test
builds that smoke a built binary against the local test server, and generated third-party notices
(`scripts/generate-notices.sh`, `THIRD_PARTY_NOTICES.md`, `about.toml`).

## [0.1.0] - Unreleased

First version: the game of `zorgch/irc-quizbot` on the Mumble protocol, as a normal client with its
own certificate.

- Game: players in the bot's channel, question, hints, points, target score, winner, hunger and
  `!botsnack`, with the upstream timings (20 s hint patience, stamina 12, 80 % recent questions).
- Commands: `!help`, `!score`, `!hiscore`, `!botsnack`, `!reload`, `!join`, `!op`, `!deop`. `!help`,
  `!score` and `!hiscore` answer with one message, one line per line, not one message per line. An
  empty board answers with a line of its own instead of staying silent (`score_empty`,
  `hiscore_empty`).
- `!stop` and `!start` for masters: the bot stays connected and keeps its players, points and
  high scores, but asks nothing and accepts no answer while it is stopped. The question that was
  open when the quiz was stopped is dropped, so nobody wins it afterwards.
- Loose answer matching: case, punctuation, `ae`/`ä`, `oe`/`ö`, `ue`/`ü`, `ss`/`ß`; short answers
  need their own word.
- High scores in a SQLite file, as the upstream bot had; a rename keeps the score.
- Mumcord message cards for the game lines, plain chat text with `quiz.cards = false`.
- Config in `bot.toml` with `[server]`, `[quiz]`, `[strings]` and `[[token]]`; unknown keys are an
  error. The channel is optional: without it the bot plays in the channel it joins at login.
- Channel passwords as `[[token]]` blocks (channel plus password), presented at login as Mumble access
  tokens; the bot warns when it should enter a restricted channel without a password for it.
- Pinned server certificate (trust on first use), persistent bot certificate, reconnect with a
  growing wait, clean shutdown, SIGHUP reloads the bank.
- `server.register = true` registers the bot's certificate at login, so the server's ACL rights
  apply to it as a known user.
- Every lost connection is logged with the connection number and the reason (`Kicked`, `Closed`,
  `Timeout`, `Error`), and the wait before the next one is decided per reason: a kick waits the full
  minute so two instances with the same nick cannot fight, everything else doubles from 1 s to 60 s.
  SIGTERM stops the bot the same way Ctrl-C does, so a service unit drains the queued lines first.
- `scripts/import-upstream-questions.sh` converts the upstream question list to the TOML bank.
- Tests for the game, the answers, the config, the storage, the chat HTML, the reconnect loop and the
  documentation gate, plus the live suite and the smoke run against the local test server.
- CI gates: format, lint, tests, actionlint and shellcheck, secrets, loopback, headless (the bot
  stays without audio devices or a UI), documentation, released changelog sections and dependencies.
  Each gate script has its own test with fixtures.

Differences to the upstream bot: no NickServ or ChanServ (Mumble has none), no channel ops and no
kick for the bot, the channel topic as the channel description, and the players taken from the
server state instead of an IRC `NAMES` reply.
