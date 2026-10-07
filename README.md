# shoryo

shoryo runs grill-me style brainstorm rounds — an LLM interviewing you until a specification is
settled — on a local browser screen instead of in the chat.

Each round's questions arrive as cards. You can ask back about a question before answering,
stamp each answer once you have checked it, and send the whole round at once. Questions whose
answer is cheap to change later sit in a separate list with the recommendation filled in and
already stamped by the LLM; you send back only the ones that look wrong. When the topic
converges, the screen shows the result for you to check before the specification is written.
The screen also shows how the decisions connect (the map) and what the topic will end up as
(the finished picture), and keeps every past round, reply and decision readable.

shoryo has two parts:

- **The `shoryo` binary**: a small local web server, one per topic, with the commands a coding
  agent uses to send rounds, wait for your answers and reply to your questions. The screen is
  built into the binary.
- **The shoryo skill** (`skills/shoryo/`): tells the agent how to write rounds, classify
  questions and handle your answers.

The brainstorm workflow that calls shoryo (for example the ba0918 or kotowari brainstorm) still
decides which questions to ask and writes the specification from shoryo's result data.

## Install

The binary is published on GitHub releases as a static Linux (x86_64) build. With
[mise](https://mise.jdx.dev/):

```sh
mise use -g github:ba0918/shoryo
shoryo --version
```

Or build it from source with Rust (the toolchain is pinned in `rust-toolchain.toml`):

```sh
cargo install --locked --git https://github.com/ba0918/shoryo
```

Then install the skill by copying or linking `skills/shoryo/` into your agent's skill directory,
for example:

```sh
ln -s "$PWD/skills/shoryo" ~/.claude/skills/shoryo
```

## Use

Ask your agent to run the brainstorm on shoryo ("do this brainstorm in shoryo"). It starts the
topic's server and tells you the URL; open it in your browser. The page listens on 127.0.0.1
only, unless the server is started with `--bind`.

The header switches the screen between English and Japanese and between light and dark. The
choice is kept for every topic in your per-user config file (`~/.config/shoryo/config.toml` on
Linux).

A topic's data stays in your per-user data directory (`~/.local/share/shoryo/` on Linux), never
in the repository. shoryo never deletes it; remove a topic's directory yourself when you no
longer need it.

## Specification

- [The screen](docs/spec/screen.md)
- [Start-up, the agent's commands and the stored data](docs/spec/server.md)
- [The shoryo skill](docs/spec/skill.md)
- [Glossary](CONTEXT.md)

The specification is written in Japanese.

## License

MIT
