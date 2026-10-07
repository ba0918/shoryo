# Project Context

## What this is

shoryo runs grill-me style brainstorm rounds (an LLM interviewing a person until a specification is settled) on a local browser screen instead of in the chat. A person reads each round's questions as cards, asks back about a question before answering, and sends the whole round at once. The screen shows how decisions connect (the map) and what the topic will end up as (the finished picture). shoryo has two parts: a local web server and CLI that a coding agent drives through commands, and the shoryo skill that tells the agent how to write rounds and handle answers. The workflow that calls it (for example the ba0918 or kotowari brainstorm) keeps the question tree and writes the specification from shoryo's result data.

The specification is in `docs/spec/`:

- `screen.md`: what the screen shows and lets the person do
- `server.md`: start-up, the commands the agent uses, and the stored data
- `skill.md`: what the shoryo skill tells the agent to do

## Stack and layout

- A Cargo workspace. The root package `shoryo` is the binary (`src/main.rs`, the CLI). The edition, the minimum supported Rust version, the version and the lint configuration are declared once under `[workspace.*]` in `Cargo.toml` and inherited by every member; the toolchain version is in `rust-toolchain.toml`.
- `crates/shoryo-core/`: the domain (the topic state, round validation, events, derived views). It holds no I/O; `scripts/check-domain-purity.sh` enforces this.
- `crates/shoryo-server/`: the HTTP server, the page and agent APIs, and the data directory.
- `crates/shoryo-webview/`: embeds `web/` into the binary.
- `web/`: the screen (HTML, CSS and plain JavaScript, no build step).
- The screen's files are embedded in the binary; the release is that one binary (`docs/spec/server.md`, "作り方と配り方").
- `docs/spec/`: the specification. `CONTEXT.md`: the glossary.
- Not built yet: the server, the screen, and the shoryo skill.

## Commands

| Purpose | Command |
|---|---|
| Install | (none beyond the pinned toolchain; rustup reads `rust-toolchain.toml`) |
| Build | `cargo build --workspace --locked` |
| Test | `cargo test --workspace --locked` |
| Lint | `cargo clippy --workspace --all-targets --locked -- -D warnings` |
| Domain purity | `scripts/check-domain-purity.sh` |
| All gates (what CI runs) | `scripts/check.sh` |
| Format check | `cargo fmt --all --check` |
| Run locally | `cargo run -- start <topic>` (prints the page URL; the agent commands are `round`, `wait`, `reply`, `end`, `result`, `stop`; see `cargo run -- --help`) |

## Conventions specific to this project

- Documents written for agents (`AGENTS.md`, `PROJECT.md`, the shoryo skill) are in English. The specification in `docs/spec/` and the glossary `CONTEXT.md` are in Japanese.
- Use the terms in `CONTEXT.md` with the meanings given there, in code names and messages as well as documents. For example, the unit shoryo handles is a 議題 (topic), never a "session": "session" means the conversation with the LLM.
- The fixed text on the screen (buttons, headings, tab names) is in English. Text the LLM writes is shown in the language it was written in.

## Constraints

- The server listens on 127.0.0.1 only unless `--bind` is given, and then warns that the page is plain HTTP.
- A topic's data lives in the per-user data directory, outside the repository, and shoryo never deletes it on its own (`docs/spec/server.md`, "記録の置き場所と寿命"). Never write brainstorm records into a repository.
- The agent passes structured data only. It never writes HTML for the screen, and it never writes a question's text into the chat.
- Distribution is a GitHub release installed through mise's github backend, so the binary must run without any file beside it.

## Glossary

`CONTEXT.md` is authoritative.
