# Keeping the bot connected

The bot is a normal client, so it can be dropped by the server at any time. `Bot::run` keeps it
connected: connect, play, and after a lost connection wait with a growing delay until the process is
stopped (`Command::Stop`, Ctrl-C, or the service stopping it). Every game state that matters is kept
across a reconnect: players, points, hunger, the recently asked questions and the high scores.

Every loss is logged with the number of the connection and the reason, so the cases can be told
apart:

```
connection 1 ended: Kicked { reason: Some("You connected to the server from another device") }; reconnecting in 60s
connection 2 ended: Error("Connection reset by peer (os error 104)"); reconnecting in 2s
connection 3 ended: Timeout; reconnecting in 8s
```

## Why a bot loses its connection

| Reason | What it means | What to do |
| --- | --- | --- |
| `Kicked { reason }` | An admin removed the bot, or a second client uses the same nick or certificate. murmur identifies a client by its certificate hash, so two instances with the same nick kick each other. | Run one instance per nick. The bot waits the full minute after a kick instead of doubling, so two bots cannot fight and the server stops counting the logins. |
| `Error(...)` | The network or the TLS connection failed. | Check reachability. murmur's autoban (`autobanAttempts`, 10 connections in 120 s by default) blocks a host that reconnects often: set `autobanAttempts = 0` for a server a bot runs on, or keep the bot single-instance. |
| `Timeout` | Nothing came from the server for 30 s (`READ_TIMEOUT`), while the bot pings every 5 s. Only happens behind a proxy or a stalled link. | Check the path to the server. |
| `Closed` | The server closed the connection normally. | Nothing; the bot reconnects. |

The protocol layer paces chat messages (a burst of 4, then one per 1.2 s), so the bot stays under
murmur's text rate limit. One known gap: the protocol crate knows murmur's `h9k` denial type but does
not answer its challenge, so a client that exceeds the rate limit would be disconnected. The quiz bot
sends at most one line per event, so it does not reach that limit.

## Run it as a service

The unit file restarts the process when it exits, which is what "always connected" needs: the bot
itself handles losing a connection, and the service handles a crash or a machine reboot.
`SIGTERM` and Ctrl-C both stop the bot the same way: what is still queued goes out at the server's
pace, then the connection closes. `SIGHUP` reads the question bank again, like `!reload`.

```sh
# /etc/mumquiz.env, mode 0600, owner mumquiz:
SERVER_PASSWORD=change-me

systemd/mumquiz.service
```

The identity must survive the restart: the certificate lives in `~/.local/share/mumquiz` for
the service user (or in the volume you mount), so the server keeps knowing the same user hash and the
ACL entry for it stays valid.

## Commands while reconnecting

A command is only answered for people in the bot's channel, so a player who is there can use
`!help`, `!score`, `!hiscore`, `!botsnack` at any time; the master commands need the master list
(`[quiz] masters` plus the bot). After a reconnect the players are read from the server state again,
so nobody has to rejoin for the game to continue.
