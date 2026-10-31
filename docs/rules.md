# Game rules

The game is the upstream Python bot's, with the parts Mumble cannot do left out.

## Round

1. The bot counts the players in its channel (and the people following it with Listen). The bot
   itself is never a player.
2. Below `quiz.min_players` it says the `waiting` line, and repeats it every `waiting_secs`
   (30 s by default).
3. With enough players it asks a question, chosen at random from the bank, skipping the questions
   asked recently. The recent list holds `recycle_percent` of the bank (80 % by default); when it is
   full, the oldest question falls out. If every question is recent, the bank starts again.
4. `hint_patience_secs` (20 s) without an answer: the bot gives a hint, revealing
   `answer length / 6` characters (at least one). Spaces are never hidden, numbers are revealed from
   left to right, words in random order. At most five hints; a one-letter answer gets the
   `hintone` line instead. After five hints, or when the answer is too short to hint, the bot gives
   the answer away (`rightanswer` and `wishluck`).
5. A correct message awards one point to its sender. When a player reaches the target score of the
   round, `1 + players / 2`, the bot names them as the winner, stores the win in the high score
   table and starts the round again.
6. At the start of a round the bot writes the channel description (the `channeltopic` line): target
   score, previous winner, all-time winner. This needs the channel's `Write` permission; without it
   the bot plays and leaves the description alone, and says so once in the log. An official Mumble
   client shows a description below 128 bytes, so the bot cuts it there.

## Hunger

Every question the bot asks costs one hunger; every answer it awards gives one back. Above
`quiz.stamina` (12 by default) it stops asking and complains once with the `botsnack` line. A
`!botsnack` sets hunger to zero and the game continues.

## Time

The game clock is the bot's own; every `tick_secs` (1 s) it decides what to do next. Nothing is
sent faster than the server's rate limit: the protocol layer paces chat messages, so a burst is
queued rather than dropped.

## Reconnect and shutdown

A lost connection is retried with a growing wait, 1 s to 60 s. On Ctrl-C the bot first sends what is
still queued at the server's pace, then closes. SIGHUP reads the question bank again, which is the
same as `!reload`.
