#!/usr/bin/env bash
# Prints one CHANGELOG.md section (its heading and the lines up to the next heading) for a version.
#   scripts/changelog-section.sh <version> <file>
set -euo pipefail

version=${1:?version missing}
file=${2:?file missing}

awk -v version="$version" '
  /^## \[/ {
    in_section = ($0 == "## [" version "] - Unreleased") ||
                 ($0 ~ "^## \\[" version "\\] - [0-9]{4}-[0-9]{2}-[0-9]{2}$")
  }
  in_section { print }
' "$file"
