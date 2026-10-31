# Question bank

One `[[question]]` block per question:

```toml
[[question]]
category = "Astronomie"
question = "Welcher Planet ist der Sonne am naechsten?"
answer = "Merkur"

[[question]]
category = "Astronomie"
question = "Wie heisst unsere Galaxis?"
answer = "Milchstrasse"
answers = ["Milchstraße", "Milchstrasse"]
```

- `category`, `question` and `answer` are required; `answers` (extra spellings a player may type)
  is optional. `answer` may itself hold alternatives separated by `|`.
- A numeric answer is revealed from left to right in hints; a word is revealed in random order.
- The bank needs about eleven questions for the recent-question recycling to make sense
  (`quiz.recycle_percent`). A smaller bank works, and the bot warns at startup.
- The bank is read at startup and by `!reload` or SIGHUP. `quiz.questions` is the path, relative to
  the working directory unless it is absolute.

## Importing the upstream bank

The upstream bot keeps its questions in a Python file. `scripts/import-upstream-questions.sh` turns
that file into this TOML format. It fetches the file from GitHub (nothing is downloaded by the bot
itself), so run it once and keep the result:

```sh
scripts/import-upstream-questions.sh https://raw.githubusercontent.com/zorgch/irc-quizbot/master/questions.py questions.toml
```

The imported bank keeps the licence of the project it came from (GPL-3.0); the bot code is
AGPL-3.0-only.
