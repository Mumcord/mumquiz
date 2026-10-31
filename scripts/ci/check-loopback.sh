#!/usr/bin/env bash
# The live tests and the smoke run may only talk to the local test server. Without arguments it
# checks the tracked test inputs; with files as arguments it checks those, which is how
# scripts/ci/test-check-loopback.sh tests this script.
#
# Rules:
# - every host a test or the smoke run uses is loopback (127.0.0.1, localhost, ::1);
# - the test server publishes its ports on loopback only, so nothing on the network can reach it.
set -euo pipefail

cd "$(dirname "$0")/../.."

status=0

files=("$@")
if [ ${#files[@]} -eq 0 ]; then
  mapfile -t files < <(git ls-files 'tests/*' 'scripts/smoke/*' 'testenv/*')
fi

for file in "${files[@]}"; do
  # Hosts the tests connect to.
  while IFS= read -r line; do
    host=$(printf '%s\n' "$line" | sed -E 's/.*host = "([^"]*)".*/\1/')
    case "$host" in
      127.0.0.1 | localhost | ::1) continue ;;
      *)
        echo "::error file=$file::$line - live tests may only use a loopback host." >&2
        status=1
        ;;
    esac
  done < <(grep -E '^[^#]*host = "[^"]+"' "$file" || true)

  # Published ports of the compose file: a mapping must name a loopback address, and a mapping
  # without one publishes on every address. Volume lines are not port mappings.
  if [ "$(basename "$file")" = "docker-compose.yml" ]; then
    bad=$(awk '
      function check(line) {
        if (!match(line, /"[^"]*"/)) return
        mapping = substr(line, RSTART + 1, RLENGTH - 2)
        count = split(mapping, parts, ":")
        last = parts[count]
        sub(/\/(tcp|udp)$/, "", last)
        if (last !~ /^[0-9]+$/) return
        if (count < 3) { print line; return }
        if (parts[1] != "127.0.0.1" && parts[1] != "localhost" && parts[1] != "::1") print line
      }
      /^[^#]*ports:/ { check($0); in_ports = 1; next }
      /^[^[:space:]]/ { in_ports = 0 }
      in_ports && /"/ { check($0) }
    ' "$file")
    if [ -n "$bad" ]; then
      while IFS= read -r line; do
        echo "::error file=$file::$line - publish the test server on 127.0.0.1 only." >&2
      done <<< "$bad"
      status=1
    fi
  fi
done

exit "$status"
