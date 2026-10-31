# Message strings

The defaults are the upstream bot's wording. `[strings]` in `bot.toml` replaces single lines, and an
unknown key is an error. Every line may use `{placeholders}` and the markdown below; a placeholder the bot cannot fill
stays in the text, so a typo shows up in chat instead of dropping words.

| Key | Placeholders | Default |
| --- | --- | --- |
| `question` | `{category}`, `{question}` | `**<{category}> Q:** {question}` |
| `waiting` | — | Playing alone is no fun. Let's wait for others... |
| `botsnack` | — | I'm hungry. Please feed me with `!botsnack`. |
| `hint` | `{hint}` | HINT: `{hint}` |
| `hintone` | — | HINT: only one character! |
| `rightanswer` | `{answer}` | the answer was: **{answer}** |
| `wishluck` | — | better luck with the next question! |
| `correctanswer` | `{answer}`, `{name}` | **{answer}** is right! congratulations, {name}! |
| `winner` | `{name}` | congratulations to **{name}**, you're winner!!! |
| `thanks` | — | ta. :-) |
| `score` | `{rank}`, `{name}`, `{points}` | {rank}. **{name}**: {points} points |
| `channeltopic` | `{target}`, `{winner}`, `{alltime}`, `{alltime_wins}` | **happy quizzing.** target score: {target}. previous winner: {winner}. all-time winner: {alltime} ({alltime_wins}). |
| `invalidname` | `{name}` | not on my watch, **{name}**! |
| `unknowncmd` | — | ... wat. |
| `help_channelinfo` | `{channel}` | **{channel}** is a quiz channel. |
| `help_botinfo` | `{bot}` | I am **{bot}**, and *I* ask the questions around here! :-> |
| `reloaded` | — | reloaded questions. |
| `score_empty` | — | no points yet this round. |
| `hiscore_empty` | — | no wins yet, the board is empty. |
| `stopped` | — | quiz stopped. I stay here, but I ask nothing until `!start`. |
| `started` | — | quiz started. |
| `help` | — | the lines of the message, one per line in the file |

Example:

```toml
[strings]
question = "Frage: {question}"
correctanswer = "{answer} stimmt :) congratz {name}!"
help = """
!score zeigt die besten Spieler dieser Runde.
!hiscore zeigt die besten Spieler aller Zeiten.
fuettern mich mit !botsnack."""
```

## Chat text is markdown

Every line the bot sends goes through the markdown subset in `src/markdown.rs`, so the templates may
use it:

| | |
| --- | --- |
| `**text**` | bold |
| `*text*` | italic |
| `` `text` `` | inline code |
| `[text](https://example.dev)` | a link, http(s) only |
| `> text` | a quote |
| `- text` | a list item |

Lines are joined with a break, everything else stays text, an unterminated marker stays text, and a
target that is not a web URL is not turned into a link. A client without cards reads the same words.

## Cards

With `quiz.cards = true` (the default) the bot sends Mumcord message cards: an accent stripe, an
icon, a title and the text. Titles are `Question`, `Hint`, `Correct`, `Winner`, `Score` and
`Notice`, with the status colours `info`, `success` and `warning`. The wire format is the one in
`docs/message-cards.md` of the Mumcord repository, so an official Mumble client shows the same line
as plain chat text. With `quiz.cards = false` the bot sends plain text for everyone.
