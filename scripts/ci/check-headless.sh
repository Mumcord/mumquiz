#!/usr/bin/env bash
# The bot stays headless: it may not pull in an audio device, a resampler or a UI. Without arguments
# it runs cargo tree; with a file as argument it checks that, which is how
# scripts/ci/test-check-headless.sh tests this script.
set -euo pipefail

cd "$(dirname "$0")/../.."

banned='cpal|alsa|rubato|nnnoiseless|audio_thread_priority|pipewire|wayland|glib|gtk|webkit|tauri'

input=${1:-}
if [ -z "$input" ]; then
  tree=$(cargo tree -e normal --locked)
else
  tree=$(cat "$input")
fi

if printf '%s\n' "$tree" | grep -iE "$banned"; then
  echo "::error::The bot must stay headless: no audio device, resampler or UI crate." >&2
  exit 1
fi
