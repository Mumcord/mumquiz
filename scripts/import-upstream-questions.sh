#!/bin/sh
# Converts the upstream Python question list (zorgch/irc-quizbot) to the bot's TOML bank.
#   scripts/import-upstream-questions.sh <questions.py or URL> <out.toml>
# Nothing is fetched by the bot itself: run this once and keep the result. The imported bank keeps
# the GPL-3.0 licence of the project it came from.
set -eu

src=${1:?questions.py (file or URL) missing}
out=${2:?output file missing}

tmp=$(mktemp)
trap 'rm -f "$tmp"' EXIT

if [ -f "$src" ]; then
  cp "$src" "$tmp"
else
  curl -fsSL --retry 3 "$src" -o "$tmp"
fi

python3 - "$tmp" "$out" <<'PY'
import ast
import sys

path, out = sys.argv[1], sys.argv[2]
rows = None
for node in ast.parse(open(path, encoding="utf-8").read()).body:
    if isinstance(node, ast.Assign) and getattr(node.targets[0], "id", "") == "questions":
        rows = ast.literal_eval(node.value)
if not rows:
    sys.exit("no questions list found in the file")

def quote(value):
    return '"' + str(value).replace("\\", "\\\\").replace('"', '\\"') + '"'

written = 0
with open(out, "w", encoding="utf-8") as file:
    file.write(
        "# Imported from zorgch/irc-quizbot (GPL-3.0). One [[question]] block per question.\n\n"
    )
    for row in rows:
        if len(row) != 3:
            print(f"skipped row with {len(row)} fields: {row}", file=sys.stderr)
            continue
        category, question, answer = (str(field).strip() for field in row)
        if not category or not question or not answer:
            print(f"skipped empty row: {row}", file=sys.stderr)
            continue
        file.write("[[question]]\n")
        file.write(f"category = {quote(category)}\n")
        file.write(f"question = {quote(question)}\n")
        file.write(f"answer = {quote(answer)}\n\n")
        written += 1

print(f"{written} questions -> {out}")
PY
