#!/usr/bin/env bash
# Smoke run: the bot binary against the local test server, then the live suite. Exit 0 = passed.
# Only ever the local server; scripts/ci/check-loopback.sh guards the hosts.
#
#   scripts/smoke/smoke.sh [run]     # run 1-9 gets its own port (40000 + 1000*run), so several
#                                    # smoke runs can live side by side on one machine
set -euo pipefail
cd "$(dirname "$0")/../.."

run=${1:-}
if [ -n "$run" ]; then
  if ! [[ "$run" =~ ^[1-9]$ ]]; then
    echo "run must be 1-9, got '$run'" >&2
    exit 2
  fi
  export MUMQUIZ_TEST_PORT=$((40000 + 1000 * run))
fi

work=$(mktemp -d)
cleanup() {
  testenv/down.sh
  rm -rf "$work"
}
trap cleanup EXIT

testenv/up.sh

cat > "$work/smoke.toml" <<TOML
[server]
host = "127.0.0.1"
port = ${MUMQUIZ_TEST_PORT:-64738}
name = "SmokeBot"
channel = "Root"
data_dir = "$work/data"
pin = false

[quiz]
questions = "data/questions.example.toml"
hiscores = "$work/hiscores.sqlite"
min_players = 1
cards = false
verbose = true
TOML

cargo build --locked --quiet

echo "smoke: the binary connects, joins the channel and speaks"
"./target/debug/mumquiz" --config "$work/smoke.toml" > "$work/log.txt" 2>&1 &
pid=$!
for _ in $(seq 1 15); do
  if grep -q "connected to 127.0.0.1" "$work/log.txt" && grep -q "Playing alone is no fun" "$work/log.txt"; then
    break
  fi
  sleep 1
done
kill "$pid" 2>/dev/null || true
wait "$pid" 2>/dev/null || true

if ! grep -q "connected to 127.0.0.1" "$work/log.txt"; then
  echo "FAIL: the bot did not connect" >&2
  cat "$work/log.txt" >&2
  exit 1
fi
if ! grep -q "Playing alone is no fun" "$work/log.txt"; then
  echo "FAIL: the bot connected but never said anything" >&2
  cat "$work/log.txt" >&2
  exit 1
fi
echo "smoke: binary ok"

echo "smoke: live suite"
cargo test --locked --test live -- --ignored --test-threads=1 --nocapture
