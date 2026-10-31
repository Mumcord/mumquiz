#!/usr/bin/env bash
# Tests scripts/ci/check-loopback.sh.
set -euo pipefail
cd "$(dirname "$0")/../.."
dir=$(mktemp -d)
trap 'rm -rf "$dir"' EXIT

fail() { echo "FAIL: $1" >&2; exit 1; }

printf 'host = "127.0.0.1"\nhost = "localhost"\n# host = "public.example.org" is only prose\n' > "$dir/good.rs"
printf 'host = "mumble.example.org"\n' > "$dir/bad.rs"

scripts/ci/check-loopback.sh "$dir/good.rs" || fail "loopback hosts must pass"
! scripts/ci/check-loopback.sh "$dir/bad.rs" || fail "a public host must fail"

cat > "$dir/docker-compose.yml" <<'YAML'
services:
  server:
    image: "mumblevoip/mumble-server:v1.5.735"
    ports:
      - "127.0.0.1:${PORT}:64738/tcp"
YAML
scripts/ci/check-loopback.sh "$dir/docker-compose.yml" || fail "loopback publishing must pass"

cat > "$dir/bad-compose.yml" <<'YAML'
services:
  server:
    ports: ["64738:64738/tcp"]
YAML
mv "$dir/bad-compose.yml" "$dir/docker-compose.yml"
! scripts/ci/check-loopback.sh "$dir/docker-compose.yml" || fail "publishing on every address must fail"

echo "check-loopback.sh: ok"
