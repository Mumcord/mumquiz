# Contributing to the quiz bot

The bot is licensed under the AGPL-3.0-only. To keep the option to relicense later, every
contribution needs a signed-off Contributor License Agreement (CLA).

## Contributor License Agreement

By contributing you agree to the following:

1. You grant the maintainer a perpetual, worldwide, non-exclusive, royalty-free license to use,
   modify, distribute and relicense your contribution.
2. You confirm that you have the right to contribute it (it is your own work, or you have permission
   from the owner, including your employer if relevant) and that it does not knowingly infringe
   anyone else's rights.
3. You keep the copyright to your contribution.
4. Your contribution is also available to everyone under the AGPL-3.0-only, like the rest of the bot.

This is a plain summary, not legal advice. Every pull request must contain this line in its
description; the template has a checkbox for it. Pull requests without it are not merged.

```
I agree to the Mumcord CLA
```

## Guidelines

- Keep changes small and focused, one logical change per commit, Conventional Commits (`feat:`,
  `fix:`, `docs:`, `chore:` ...).
- The game logic stays pure: an injected clock and random source, no network calls in `src/quiz.rs`.
  Only `src/bot.rs` talks to a server.
- New logic gets tests. A bug is first reproduced with a test, then fixed.
- Never include secrets, and never connect tests to a public Mumble server: use `testenv/`.
- Question banks you import from the upstream project keep their GPL-3.0 licence; say where they
  came from (see `NOTICE`).

## Gates

CI runs these checks; every one of them can be run locally. Each gate script has its own test with
fixtures (`scripts/ci/test-*.sh`), so a gate that is broken itself is noticed.

| Gate | Locally | What it checks |
| --- | --- | --- |
| Format | `cargo fmt --all --check` | `rustfmt.toml` (edition 2024, 100 columns) |
| Lint | `cargo clippy --all-targets --locked -- -D warnings` | no warnings, `unwrap_used` and `dbg_macro` are errors |
| Tests | `cargo test --locked` | game, answers, config, storage, chat HTML, the gates below |
| Workflows and scripts | `scripts/ci/lint-ci.sh` | actionlint over `.github/workflows`, shellcheck over every `*.sh` (CI pins the versions) |
| Secrets | `scripts/ci/check-no-secrets.sh` | nothing that holds a key, password or token is committed; tracked values are placeholders |
| Loopback | `scripts/ci/check-loopback.sh` | the live tests and the smoke run only talk to the local test server, and the test server publishes on loopback only |
| Headless | `scripts/ci/check-headless.sh` | no audio device, resampler or UI crate in the dependency tree |
| Documentation | `cargo test --test gates` | every `[server]` and `[quiz]` key, every message key and every command is documented |
| Changelog | `scripts/ci/check-changelog-released.sh` | a released section of `CHANGELOG.md` never changes |
| Dependencies | `cargo deny check` | advisories, licences, duplicates and sources (`deny.toml`) |
| Live tests | `scripts/smoke/smoke.sh` | the bot binary and the whole bot against the local test server |
| Test builds | `scripts/smoke/smoke-artifact.sh <binary>` | a built binary runs against the local test server, connects, joins the channel and speaks |
| Notices | `scripts/generate-notices.sh` | `THIRD_PARTY_NOTICES.md` from the licences of the dependencies (`about.toml`, pinned cargo-about in `scripts/tools.env`) |

Where a shellcheck warning is a false positive, add a
`# shellcheck disable=SCxxxx # reason` comment directly above that one command; blanket disables are
not accepted.

If a gate fails, fix the finding; do not disable the gate. When a dependency must be accepted in
`deny.toml`, add it with the reason and the issue that tracks it.

## Pipeline

| Workflow | Runs | What it does |
| --- | --- | --- |
| `ci.yml` | every pull request, push to the default branch, manual run | the gates above, the build matrix, the live tests |
| `dependency-audit.yml` | called by `ci.yml`, weekly on the default branch | cargo-deny: advisories, licences, duplicates, sources |
| `build-and-smoke.yml` | reusable, called by `test-builds.yml` and later by a release run | builds the binary for `x86_64-unknown-linux-gnu` and `aarch64-unknown-linux-gnu`, smokes it against the local test server, uploads it as an artifact |
| `test-builds.yml` | every pull request, manual run | the reusable build and smoke, artifacts only, nothing published |
| `notices.yml` | push to the default branch with dependency changes, manual run | regenerates `THIRD_PARTY_NOTICES.md` and opens or updates the pull request `chore: regenerate-notices`; it never pushes to the default branch |

The push triggers look at the default branch, which is `main`; a repository whose branch is still
called `master` gets no run on push, only the pull request checks and manual runs. A gate script is
never disabled to make a run pass: fix the finding, or accept a dependency in `deny.toml` with the
reason and the issue that tracks it.

## Changelog

Add entries to the `## [Unreleased]` or `## [X.Y.Z] - Unreleased` section. Sections of released
versions (a heading with a date) must never change: CI fails a pull request that edits one. Correct
an old entry with a note in the next version.

## Live tests

    testenv/up.sh
    cargo test --test live -- --ignored --test-threads=1

The suite is also the smoke run (`scripts/smoke/smoke.sh`), which builds the binary, checks that it
connects and speaks, and then runs the suite. Give a parallel run its own port with
`scripts/smoke/smoke.sh 2` or `MUMQUIZ_TEST_PORT=64750`. The compose project is `mumquiz-testenv`, so
the run does not touch the Mumcord test servers.
