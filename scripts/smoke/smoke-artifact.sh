#!/usr/bin/env bash
# Smoke test for a built binary: the artifact of a test build or a release. Starts the local test
# server, runs the given binary against it and checks the lines it writes, so a build that cannot
# connect or cannot speak fails even when its unit tests pass.
#
# Only ever the local test server; scripts/ci/check-loopback.sh guards the hosts.
#
#   scripts/smoke/smoke-artifact.sh <path to the mumquiz binary> [run]
#                                   # run 1-9 gets its own port (40000 + 1000*run), so several smoke
#                                   # runs can live side by side on one machine
set -euo pipefail

binary=${1:?usage: smoke-artifact.sh <binary> [run]}
if [ ! -x "$binary" ]; then
  echo "$binary is not an executable (an artifact loses its exec bit if it is extracted wrong)" >&2
  exit 2
fi

cd "$(dirname "$0")/../.."

run=${2:-}
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

# The fixture bank, so the question and its answer are known and the run stays short.
cat > "$work/smoke.toml" <<TOML
[server]
host = "127.0.0.1"
port = ${MUMQUIZ_TEST_PORT:-64738}
name = "SmokeBot"
channel = "Root"
data_dir = "$work/data"
pin = false

[quiz]
questions = "tests/live-bank.toml"
hiscores = "$work/hiscores.sqlite"
min_players = 1
cards = false
verbose = true
TOML

echo "smoke: the binary reports its version"
"$binary" --version

echo "smoke: the binary connects, joins the channel and speaks"
"$binary" --config "$work/smoke.toml" > "$work/log.txt" 2>&1 &
pid=$!

# The bot needs a player before it asks, so the two lines are the proof that it read its config,
# joined the channel and wrote to the server.
for _ in $(seq 1 30); do
  if grep -q "playing in channel" "$work/log.txt" && grep -q "Playing alone is no fun" "$work/log.txt"; then
    break
  fi
  sleep 1
done

kill "$pid" 2>/dev/null || true
wait "$pid" 2>/dev/null || true

if ! grep -q "connected to 127.0.0.1" "$work/log.txt"; then
  echo "FAIL: the binary did not connect" >&2
  cat "$work/log.txt" >&2
  exit 1
fi
if ! grep -q "Playing alone is no fun" "$work/log.txt"; then
  echo "FAIL: the binary connected but never said anything" >&2
  cat "$work/log.txt" >&2
  exit 1
fi
echo "smoke: binary ok"
