# shoryo

[日本語](README-ja.md)

shoryo puts an LLM's requirements interview in a local browser instead of a chat window.
The agent asks questions in rounds. You check and send your answers, then the calling workflow
writes the specification once you agree on the result.

Questions about policy and behavior appear as cards. You can ask the LLM about a question
before answering, stamp an answer after checking it, and send the whole round at once.
Questions whose answers are cheap to change later appear in a separate provisional list.
The agent fills in a recommendation and stamps it on your behalf. You can change these answers
before sending the round. They become decisions only when you send them.

The map shows how questions and decisions connect. The finished picture shows the intended
result. You can read past rounds, replies and decisions, and request a review of a decision
while the latest round is still open. When the discussion converges, you check the result
before the workflow writes the specification.

shoryo has two parts:

- The `shoryo` binary runs a local web server for each topic. The agent uses its commands to
  send rounds, wait for answers and reply to questions. The binary includes the screen's files.
- The [shoryo skill](skills/shoryo/SKILL.md) tells the agent how to write rounds, classify
  questions and handle answers.

The brainstorm workflow that uses shoryo chooses the questions and writes the specification
from shoryo's result data. The browser does not call a model API
itself. The agent exchanges questions, answers and replies through the CLI.

## Install

### Binary

Install the static Linux x86_64 binary from GitHub releases with
[mise](https://mise.jdx.dev/):

```sh
mise use -g github:ba0918/shoryo
shoryo --version
```

Or build from source with Rust 1.99 or later:

```sh
cargo install --locked --git https://github.com/ba0918/shoryo
```

### Agent skill

Install the skill with [GitHub CLI](https://cli.github.com/). For Claude Code, install it at
user scope so it is available across projects:

```sh
gh skill install ba0918/shoryo shoryo --agent claude-code --scope user
```

For another agent, change `--agent` to its identifier. Run `gh skill install --help` for the
supported agents and installation options.

## Use

Ask your agent to "do this brainstorm in shoryo". It starts the topic's server and gives you a
URL to open in your browser. The server listens on `127.0.0.1` by default. Using `--bind` to
listen elsewhere exposes the page over plain HTTP.

Use the header to switch between English and Japanese and choose a light, dark or system
theme. These settings apply across topics. On Linux, the default config file is
`~/.config/shoryo/config.toml`.

The header shows "Waiting for your action" only while the agent's `wait` command is active. Otherwise,
it shows no agent status. This does not tell you whether the model is running or has stopped.

After you send a round, a footer shows what you are waiting for. Sending answers or review
requests starts a wait for the next round. Proceeding with a result starts a wait for the topic
to end. The footer stays visible across tabs and survives reloads. After three minutes without
the expected output, it adds a reminder to check the agent. The reminder does not mean the
agent has stopped.

A new round or reply outside the visible area triggers a toast with a View button. The
header's notification icon lists the last ten arrivals and shows an unread count. You can
open the list after a toast disappears. Back returns to the place you left.

Topic data stays outside the repository in your per-user data directory. On Linux, the
default location is `~/.local/share/shoryo/`. shoryo never deletes this data. Remove a topic's
directory yourself when you no longer need it.

## Specification

- [The screen](docs/spec/screen.md)
- [Diagram controls, notifications and keyboard access](docs/spec/interaction.md)
- [Start-up, the agent's commands and the stored data](docs/spec/server.md)
- [The shoryo skill](docs/spec/skill.md)
- [Glossary](CONTEXT.md)

The specification is written in Japanese.

## License

[MIT](LICENSE)
