# Commands

Everything a player types in the channel (or in a private message to the bot) is read as chat text:
the HTML the server sends is turned into plain text first, so `<b>Mercury</b>` reads as `Mercury`.

Commands start with `quiz.command_prefix` (`!` by default) and are matched case-insensitively. A
normal message is checked for the answer before the command part, so `!score` never hides an answer
and an answer never hides a command.

## General

| Command | Answer |
| --- | --- |
| a message that contains the answer | the sender scores a point, if a question is still open |
| `!help` or `!help <name>` | the help as one private message (a whisper), only to a player, one line per line, commands marked as code |
| `!score` | the top five of this round, in the channel, as one message with one player per line. With nobody on the board it says `no points yet this round.` instead of staying silent. |
| `!hiscore` | the top five of all time, in the channel, as one message with one player per line |
| `!botsnack` | feeds the bot: hunger goes to zero and it asks again |
| anything else with the prefix | the `unknowncmd` line |

An answer is matched loosely. Case, punctuation and the spellings `ae`/`ä`, `oe`/`ö`, `ue`/`ü` and
`ss`/`ß` count as the same, so `Koenig`, `König` and `koenig` are one answer. A short answer (three
letters or fewer) must appear as its own word: the answer `rot` does not match `proton`.

The answer is read from the chat text: a message that arrives as HTML (`<b>Sonne</b>`) is
converted to plain text first, so tags and the entities Mumble uses do not change the answer.

Only people in the bot's channel (or following it with Listen) are players, and only a player's
message counts as an answer. A question is answered once: the first correct message wins the point,
and the same answer again gives nothing.

## Master commands

Masters are `[quiz] masters` plus the bot itself. Mumble has no channel operator and a client may
not kick, so these commands only change what the bot itself may do:

| Command | Master only | Answer |
| --- | --- | --- |
| `!reload` | yes | reads the question bank again |
| `!join [channel]` | yes | joins the configured channel, or the one given |
| `!op <name>` | yes | adds a master to the bot's list |
| `!deop <name>` | yes | removes a master; the bot never removes itself |
| `!stop` | yes | pauses the quiz: nothing is asked and no answer counts. Players, points and the high scores stay. The question that was open is dropped, so nobody wins it afterwards. |
| `!start` | yes | resumes the quiz after a `!stop` |

A non-master that tries one gets the `invalidname` line in the channel, and nothing else. Real
channel rights (ACL, Speak, Write) belong to the server's administrators, not to the bot.
