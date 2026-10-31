#!/usr/bin/env bash
# Tests scripts/ci/check-headless.sh.
set -euo pipefail
cd "$(dirname "$0")/../.."
dir=$(mktemp -d)
trap 'rm -rf "$dir"' EXIT

fail() { echo "FAIL: $1" >&2; exit 1; }

cat > "$dir/good.txt" <<'TREE'
mumquiz v0.1.0
├── anyhow
├── clap
├── mumcord-protocol
│   └── rustls
├── rusqlite
└── tokio
TREE

cat > "$dir/bad.txt" <<'TREE'
mumquiz v0.1.0
├── cpal
│   └── alsa
└── rubato
TREE

scripts/ci/check-headless.sh "$dir/good.txt" || fail "a headless tree must pass"
! scripts/ci/check-headless.sh "$dir/bad.txt" || fail "an audio tree must fail"

# The real tree of this repository is headless.
scripts/ci/check-headless.sh || fail "the bot dependencies must stay headless"

echo "check-headless.sh: ok"
