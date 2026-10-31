#!/usr/bin/env bash
# Lints the GitHub workflows with actionlint and every shell script with shellcheck, exactly as the
# "Lint workflows and scripts" CI job does. Needs both tools on PATH (the CI job pins the versions).
# Run from anywhere: scripts/ci/lint-ci.sh
set -euo pipefail

cd "$(dirname "$0")/../.."

for tool in actionlint shellcheck; do
  command -v "$tool" >/dev/null || { echo "$tool is not installed" >&2; exit 1; }
done

status=0

echo "actionlint"
actionlint || status=1

# The shell comes from each script's shebang. -x follows sourced files, and --source-path lets the
# scripts find their own helpers.
echo "shellcheck"
mapfile -t scripts < <(git ls-files '*.sh')
for script in "${scripts[@]}"; do
  shellcheck -x --source-path=SCRIPTDIR "$script" || status=1
done
exit "$status"
