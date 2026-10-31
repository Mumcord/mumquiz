#!/usr/bin/env bash
# Starts the local test server: ./testenv/up.sh [port]
# Creates a throwaway .env when there is none, so CI and fresh clones work without a real password.
set -euo pipefail
cd "$(dirname "$0")"

port=${1:-${MUMQUIZ_TEST_PORT:-64738}}

if [ ! -f .env ]; then
  printf 'SUPERUSER_PASSWORD=%s\nSERVER_PASSWORD=\n' "$(openssl rand -hex 16)" > .env
  echo "testenv/.env created with a throwaway password (git-ignored)"
fi

PUID=$(id -u)
PGID=$(id -g)
export PUID PGID MUMQUIZ_TEST_PORT="$port"

docker compose -p mumquiz-testenv up -d

# murmur needs a few seconds for its virtual server and its self-signed certificate; a test that
# connects before that gets "tls handshake eof".
for _ in $(seq 60); do
  if timeout 2 bash -c "</dev/tcp/127.0.0.1/$port" 2>/dev/null; then
    echo "test server up on 127.0.0.1:$port"
    exit 0
  fi
  sleep 1
done
echo "the test server did not answer on 127.0.0.1:$port within 60s" >&2
docker compose -p mumquiz-testenv logs --tail 20 server >&2
exit 1
