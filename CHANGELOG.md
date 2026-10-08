# Changelog

All notable changes to shoryo are recorded here. The version follows
[Semantic Versioning](https://semver.org/).

## [0.1.0] - 2026-10-08

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
  can be sent, a confirmation of what is sent, asking the LLM (multi-line) with replies and grid
  diagrams, past rounds as answered, the decisions tab with review requests, review rounds with
  their fixes, the result of a converged topic, the map of decisions and questions with path and
  all ranges, zoom, moving and going back, and the finished picture.
- Stamps show their local date, with the year and time on hover or focus (and tap once sent).
  Past result rounds keep the picture and records shown when sent and say whether the person
  proceeded or requested a review. Review requests ask for confirmation, cannot be sent between
  sending and the next round, and can be withdrawn until the LLM gives its conclusion.
- Arrivals from the LLM are announced with toasts, a notification icon and a background-tab
  title count. View opens the arrival itself, including folded replies; Back returns to the
  previous place. Map points show details on focus and move into view when outside the map.
- A plain wheel over diagrams scrolls the page; Ctrl/⌘ + wheel zooms the diagram instead.
  Touch uses one finger to move the map or a dialog picture, and two fingers for in-page pictures.
  Keyboard operation, preserved typing focus and reduced motion are supported.
- The screen's language (English or Japanese) and theme (light, dark or following the OS) are
  switched from its header and kept in the per-user config file (`~/.config/shoryo/config.toml`
  on Linux).
- The header shows "Waiting for your action" while the agent's `wait` command is active,
  without claiming the model is processing. After submission, a persistent footer waits for
  the next round or the topic's end and suggests checking the agent after three minutes.
- A topic's data is one JSON file per topic in the per-user data directory
  (`~/.local/share/shoryo/` on Linux), never in the repository, and is never deleted by shoryo.
  Stamp and send times use the server's UTC clock; each round keeps its picture and records.
- The shoryo skill (`skills/shoryo/`) tells a coding agent how to run a brainstorm through these
  commands; install it with `gh skill install ba0918/shoryo shoryo` and select your agent
  and installation scope.
- Releases carry a static Linux x86_64 binary that mise's github backend can install
  (`mise use -g github:ba0918/shoryo`).

[0.1.0]: https://github.com/ba0918/shoryo/releases/tag/v0.1.0
