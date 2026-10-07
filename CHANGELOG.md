# Changelog

All notable changes to shoryo are recorded here. The version follows
[Semantic Versioning](https://semver.org/).

## [Unreleased]

The first release.

### Added

- `shoryo start <topic>` runs a local web server for one brainstorm topic and prints the page's
  URL. It listens on 127.0.0.1 unless `--bind` is given (then it warns that the page is plain
  HTTP); `--port` fixes the port. A second start of a running topic is refused; starting a topic
  that has data resumes it.
- The agent's commands: `round` puts a round on the screen and prints only a one-line notice,
  `wait` returns the person's asks, sent answers, review requests and class swaps (kept until
  acknowledged), `reply` answers an ask, `end` ends the topic, `result` prints the topic's whole
  data (also when no server runs), `stop` stops the server.
- The screen, built into the binary: the current round as folded cards with the recommendation
  preselected, the consequence of the chosen option, prerequisite chains, a list for provisional
  questions pre-approved by the LLM, a defer switch, a stamp on every question before the round
  can be sent, a confirmation of what is sent, asking back (multi-line) with replies and grid
  diagrams, past rounds as answered, the decisions tab with review requests, review rounds with
  their fixes, the result of a converged topic, the map of decisions and questions with path and
  all ranges, zoom, moving and going back, and the finished picture.
- The screen's language (English or Japanese) and theme (light, dark or following the OS) are
  switched from its header and kept in the per-user config file (`~/.config/shoryo/config.toml`
  on Linux).
- A topic's data is one JSON file per topic in the per-user data directory
  (`~/.local/share/shoryo/` on Linux), never in the repository, and is never deleted by shoryo.
- The shoryo skill (`skills/shoryo/`) tells a coding agent how to run a brainstorm through these
  commands; it is installed by copying or linking it into the agent's skill directory.
- Releases carry a static Linux x86_64 binary that mise's github backend can install
  (`mise use -g github:ba0918/shoryo`).
