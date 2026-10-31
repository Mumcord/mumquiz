#!/usr/bin/env bash
# Fails when a pull request edits the CHANGELOG.md section of an already released version:
#   scripts/ci/check-changelog-released.sh <changelog on the base branch> <changelog of the pull request>
# A section is released once its heading carries a date ("## [0.1.0] - 2026-11-01"); only the
# Unreleased sections may change. Dating a section for the first time is fine, because the base
# still said Unreleased. Tested by scripts/ci/test-check-changelog-released.sh.
set -euo pipefail

base=${1:?base changelog missing}
head=${2:?changelog of the pull request missing}
here=$(dirname "$0")/..

status=0
while IFS= read -r version; do
  if ! diff -u --label "$version on the base branch" --label "$version in this pull request" \
    <("$here/changelog-section.sh" "$version" "$base") <("$here/changelog-section.sh" "$version" "$head"); then
    echo "::error file=CHANGELOG.md::The released section [$version] must not change. Add the entry under Unreleased, or publish a patch version." >&2
    status=1
  fi
done < <(grep -oP '^## \[\K[0-9]+\.[0-9]+\.[0-9]+(?=\] - [0-9]{4}-[0-9]{2}-[0-9]{2}\s*$)' "$base")
exit "$status"
