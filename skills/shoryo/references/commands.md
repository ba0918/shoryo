# Commands

Read this before running any shoryo command for the first time in a session. Every command
names the topic; run all of them from the same directory (the repository, or wherever the topic
was started), because the topic is found from the repository top of the current directory and
the topic name.

A topic name uses letters, digits, `.`, `_` and `-` only.

| Command | What it does | Prints |
|---|---|---|
| `shoryo start <topic> [--port N] [--bind ADDR]` | Starts the topic's server and keeps running until stopped. Resumes the topic when it has data. Refuses when the topic's server is already running. | One line with the page URL. With `--bind` other than 127.0.0.1, a warning on stderr that the page is plain HTTP. |
| `shoryo round <topic> [FILE]` | Puts the next round on the screen. Reads the round JSON from FILE, or from stdin. | `Round N is on the screen: <URL>` — never the round's content. |
| `shoryo wait <topic> [--ack ID,ID…] [--timeout SECONDS]` | Acknowledges the events with the given ids, then waits until there is at least one unacknowledged event and prints all of them. With `--timeout`, returns `{"events":[]}` when nothing happens in time; without it, waits as long as it takes. | `{"events":[…]}` |
| `shoryo reply <topic> <ASK_ID> [FILE]` | Replies to an ask. Reads the reply JSON from FILE, or from stdin. The reply appears on the open page at once. Accepted for asks of a round already sent. | Nothing. |
| `shoryo end <topic>` | Ends the topic. Refused while a round has not been sent. The page stays open and shows the ended state. | Nothing. |
| `shoryo result <topic>` | The topic's whole data, from the running server or, when none runs, from the stored file. | The data as JSON. |
| `shoryo stop <topic>` | Stops the topic's server. | Nothing. |
| `shoryo --version` | The installed version. | `shoryo <version>` |

A refused or failed command prints `shoryo: <reason>` on stderr and exits with status 1. `round`,
`wait`, `reply` and `end` need the server: when it is not running they say so and name the
`shoryo start` command to run. After `end`, `wait` and `reply` are refused; `round` is accepted
and reopens the topic.

## Where the data lives

`<per-user data directory>/shoryo/<repository key>/<topic>/` — on Linux
`~/.local/share/shoryo/…` unless `XDG_DATA_HOME` says otherwise. Nothing is written inside the
repository, and shoryo never deletes a topic's data; deleting it is the person's decision.
