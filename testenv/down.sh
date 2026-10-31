#!/usr/bin/env bash
# Stops the local test server: ./testenv/down.sh. Data is kept, so the next run sees the same server.
set -euo pipefail
cd "$(dirname "$0")"
docker compose -p mumquiz-testenv down --remove-orphans
