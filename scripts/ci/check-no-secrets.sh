#!/usr/bin/env bash
# No secrets in the repository. Without arguments it checks the tracked files; with files as
# arguments it checks those, which is how scripts/ci/test-check-no-secrets.sh tests this script.
#
# Rules:
# - bot.toml, .env, *.sqlite and the test server data are never committed (.gitignore covers them).
# - Every key, password, token or secret value in a tracked file must be a placeholder: empty, or
#   something obviously fake (REPLACE-..., change-me..., placeholder, your-...).
set -euo pipefail

cd "$(dirname "$0")/../.."

status=0

is_placeholder() {
  case "$1" in
    "" | REPLACE-* | change-me* | placeholder* | your-* | example*) return 0 ;;
    *) return 1 ;;
  esac
}

files=("$@")
if [ ${#files[@]} -eq 0 ]; then
  mapfile -t files < <(git ls-files)
fi

for file in "${files[@]}"; do
  case "$file" in
    bot.toml | */bot.toml | .env | */.env | *.sqlite | testenv/data/* | testenv/certs/*)
      echo "::error file=$file::Secrets and bot data must not be committed." >&2
      status=1
      continue
      ;;
  esac

  # Values that look real, in TOML and in env files.
  while IFS= read -r line; do
    value=$(printf '%s\n' "$line" | sed -E 's/^[A-Za-z_]+[[:space:]]*=[[:space:]]*"?([^"]*)"?.*/\1/')
    if ! is_placeholder "$value"; then
      echo "::error file=$file::$line - use a placeholder, the real value stays out of the repository." >&2
      status=1
    fi
  done < <(grep -E '^[[:space:]]*(key|password|token|secret|SUPERUSER_PASSWORD|SERVER_PASSWORD)[[:space:]]*=' "$file" || true)
done

exit "$status"
