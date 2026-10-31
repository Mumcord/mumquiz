#!/usr/bin/env bash
# Tests scripts/ci/check-no-secrets.sh.
set -euo pipefail
cd "$(dirname "$0")/../.."
dir=$(mktemp -d)
trap 'rm -rf "$dir"' EXIT

fail() { echo "FAIL: $1" >&2; exit 1; }

cat > "$dir/good.toml" <<'TOML'
[server]
password = ""

[quiz]
[strings]
TOML
printf 'key = "REPLACE-WITH-A-LONG-RANDOM-KEY"\n' >> "$dir/good.toml"
printf 'SUPERUSER_PASSWORD=change-me-superuser\n' >> "$dir/good.toml"

scripts/ci/check-no-secrets.sh "$dir/good.toml" || fail "placeholders must pass"

printf 'password = "a real password"\n' > "$dir/bad.toml"
! scripts/ci/check-no-secrets.sh "$dir/bad.toml" || fail "a real secret must fail"

# A committed bot.toml or database fails even with an empty value.
printf 'password = ""\n' > "$dir/bot.toml"
! scripts/ci/check-no-secrets.sh "$dir/bot.toml" || fail "bot.toml must fail"
! scripts/ci/check-no-secrets.sh "$dir/quiz-hiscores.sqlite" || fail "a data file must fail"

echo "check-no-secrets.sh: ok"
