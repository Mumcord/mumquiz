<!-- Keep the CLA line: a pull request without it is not merged (CONTRIBUTING.md). -->

## What changed

One paragraph: what this pull request does, and why.

## Gates

- [ ] `cargo fmt --all --check`
- [ ] `cargo clippy --all-targets --locked -- -D warnings`
- [ ] `cargo test --locked`
- [ ] `scripts/ci/lint-ci.sh` (needs actionlint and shellcheck)
- [ ] `scripts/ci/check-no-secrets.sh`, `scripts/ci/check-loopback.sh`, `scripts/ci/check-headless.sh`
- [ ] `cargo deny check`
- [ ] `scripts/smoke/smoke.sh` when the change touches the game, the connection or the test server

## CLA

- [ ] I agree to the Mumcord CLA
