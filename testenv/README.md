# Local test server

The live tests and the smoke run talk to this server only, never to a public one. Requires Docker (with compose) and `openssl`. The compose project is
`mumquiz-testenv`, so a run here never touches the Mumcord test servers.

    testenv/up.sh            # or: testenv/up.sh 64750 for a second, parallel run
    testenv/down.sh

The server name is `Mumquiz Test Server`, the root channel is `Root`, the SuperUser password comes
from `testenv/.env` (git-ignored; `up.sh` creates a throwaway one on a fresh clone). Ports publish
on loopback only, so nothing on the network can reach the test server.

| Variable | Default | Meaning |
| --- | --- | --- |
| `MUMQUIZ_TEST_PORT` | `64738` | Host port of the test server. Give parallel runs their own port. |
| `DATA_ROOT` | `data` | Folder the server keeps its database in. |
| `SUPERUSER_PASSWORD` | from `.env` | SuperUser login, used by the live test that writes the channel description. |

Tests read the password from the environment first, then from `testenv/.env`
(`MUMQUIZ_TESTENV_DIR`, default `testenv`, relative to the crate root).
