# Plan: the first working shoryo

## Goal

After this plan, the person can run a real brainstorm through shoryo. The coding agent starts a server for a topic, sends rounds, waits for events, replies to asks, and ends the topic. The person reads the rounds as cards in the browser, asks back, and sends answers. The map, the finished picture, past rounds and the decisions are all on the screen. The shoryo skill tells the agent how to do this, and the binary is released through GitHub and installed with mise.

## Specification

- [画面](../spec/screen.md): what the screen shows and lets the person do
- [起動と往復と記録](../spec/server.md): start-up, the agent's commands, the stored data, distribution
- [shoryo skill](../spec/skill.md): what the skill tells the agent to do
- [CONTEXT.md](../../CONTEXT.md): the glossary. Use its terms in code names, messages and documents (`PROJECT.md`, "Conventions specific to this project").

The specification is written in Japanese. Links below point at its headings as written.

## Approach and why

### Layout

The repository becomes a Cargo workspace shaped like the author's other tool, kemi:

```
Cargo.toml          workspace + root package `shoryo` (the binary: src/main.rs, the CLI)
crates/
  shoryo-core/      domain: the topic state, round validation, events, derived views. Pure.
  shoryo-server/    HTTP server: page API, agent API, live push to the page, the data directory
  shoryo-webview/   embeds web/ into the binary
web/                the screen: HTML, CSS and plain JavaScript, no build step
skills/shoryo/      the shoryo skill (SKILL.md and references)
tests/              integration tests of the binary (the round trip)
web/tests/          browser tests of the screen
```

The root package stays the binary named `shoryo`, as in kemi, so `cargo install --git` and a release build at the root keep working. `shoryo-core` holds no I/O: no HTTP, no async runtime, no filesystem, no process spawning, no clock reads (time comes in as a value). A script, `scripts/check-domain-purity.sh`, enforces this the way kemi's does.

### Reuse ledger

Each layer records adopt or build, with one line why (skill `ba0918-reuse`). Add every crate with `cargo add` so the registry picks the version; never write a version from memory. When only part of a crate is used, turn its default features off.

| Layer | Decision | Why |
|---|---|---|
| HTTP server and live push | adopt `axum` + `tokio`; push to the page with Server-Sent Events | kemi already runs this stack in the same role; SSE is one-way server→page, which is all "返事はすぐ出る" needs |
| Asset embedding | adopt `rust-embed` | kemi's `kemi-webview` does exactly this |
| JSON | adopt `serde` + `serde_json` | ecosystem standard |
| CLI parsing | adopt `clap` (derive) | used by the author's kakoi |
| Per-user data directory | adopt `dirs` (`data_dir()`) | gives `~/.local/share` on Linux and the right place elsewhere; a hand-rolled XDG lookup would miss `XDG_DATA_HOME` and other platforms |
| Agent CLI → running server | adopt `ureq` or build on `std::net::TcpStream` (implementer's choice; both keep behavior) | the CLI is a short-lived client to a loopback server |
| Identifiers | build: the agent's own string IDs for what it writes; per-topic increasing integers for what the server creates | IDs only need to be unique and stable inside one topic ([状態データ](../spec/server.md#状態データ)); no crate needed (provisional answer 13) |
| Random secret for URLs | adopt `getrandom` | needed for the URL secret below; kemi uses it too |
| Screen | build: plain HTML/CSS/JS, no framework, no bundler | the single binary embeds files as they are; the mocks this design came from are plain JS |
| Diagram drawing | build: SVG from the grid text | the specification forbids automatic layout for LLM diagrams ([図](../spec/screen.md#図)); no library draws to an author-given grid |
| Browser tests | adopt Playwright (Node, `@playwright/test`) | a real browser is the only way to observe "the position does not move" and live updates; kemi tests its screen with Node scripts too |
| Lock against double start | build on `std::fs::File::create_new` for a lock file | standard library covers it |

### Provisional answers

The specification is silent on these. Each is the plan's answer; the person can overturn it at approval.

1. **One server process per topic; the agent's commands are short-lived clients.** `shoryo` has a long-running subcommand that starts the server for a topic ([議題と起動](../spec/server.md#議題と起動)), and short subcommands for ラウンドを出す, 待つ, 返す, 終える and 結果を渡す ([往復](../spec/server.md#往復)). The agent runs the server in the background. Command names are delegated ([任せたこと](../spec/server.md#任せたこと)). Overturn if: the person wants the commands to work without a running server (for example 結果を渡す reading the file directly).
2. **How a client finds the server: an endpoint file in the topic's data directory.** It holds the port and the URL secret, is written with mode 0600 when the server starts and is removed when it stops. A client given the topic name resolves the data directory the same way the server did. Overturn if: the person wants clients to take a URL instead.
3. **A random secret in every URL.** The page is served under `/s/<secret>/` and the agent API under the same secret, as kemi does, so `--bind` to a LAN does not let others drive the topic without the URL. Overturn if: the person wants no secret on 127.0.0.1.
4. **Topic identity on disk.** The topic directory is `<dirs::data_dir()>/shoryo/<repo-key>/<topic-name>/`. `<repo-key>` is the last path component of the repository top (or of the start directory outside a repository), then `-`, then the first 12 hex digits of the SHA-256 of the full absolute path. The full path is also stored inside the state. Topic names are restricted to `[A-Za-z0-9._-]`. Overturn if: the person wants human-chosen repository keys.
5. **Double start.** The server takes a lock file (`create_new`) in the topic directory, writes its process ID into it, and removes it on exit. A second start fails with a message naming the running process. A lock file whose process no longer exists is treated as stale and replaced. Overturn if: stale-lock recovery should be manual.
6. **One JSON file per topic, written atomically** (write a temporary file in the same directory, then rename). The pre-submit state is saved on each operation ([状態データ](../spec/server.md#状態データ)). Overturn if: never.
7. **Event acknowledgement.** 待つ takes the IDs of events received last time and returns every event not yet acknowledged ([待つ](../spec/server.md#待つ)). Unacknowledged events live in the state file, so they survive a restart. Overturn if: never.
8. **The diagram text format is the one the mocks used, plus an empty slot.** One item per line: `id = label` declares a node, `id = ? label` declares an empty slot drawn as a dashed frame (for the finished picture's undecided places, [完成図](../spec/screen.md#完成図)), `| a | b | . |` is one grid row (`.` is an empty cell), `a -> b : label` is an edge, a blank line is ignored. The format is delegated ([任せたこと](../spec/skill.md#任せたこと)); this records the choice so the skill can document it. Overturn if: the person wants another format.
9. **The JSON field names are delegated, but they are documented.** Whatever names the implementer chooses for the round, reply and result formats are written into `skills/shoryo/references/` in step 12. The agent is the writer of these formats, so an undocumented format is a defect.
10. **Browser tests run in CI.** The CI workflow gets a second job that installs Node and Playwright's Chromium and runs `scripts/test-web.sh` (step 11). Overturn if: the person wants browser tests to stay local-only.
11. **The release target is `x86_64-unknown-linux-musl` only**, a static binary, as the author's kakoi publishes. Overturn if: the person needs macOS or Windows builds now.
12. **The skill is installed by the person.** `skills/shoryo/` lives in this repository; copying or linking it into an agent's skill directory is the person's step, described in `README.md`. Overturn if: the person wants an install command.
13. **Who assigns IDs.** The agent assigns the IDs of the questions and decisions it writes (short strings, unique in the topic; core refuses a reused one). Core assigns the IDs of asks and events. A round's prerequisites are checked against the stored records together with the records carried in the same round, so a round may introduce a decision and ask a question resting on it at once. Overturn if: the person wants core to assign every ID.
14. **The server's lifetime.** 終える does not stop the server: the open page shows the ended state ([終える](../spec/server.md#終える)). The server stops on SIGINT or SIGTERM, or on a stop subcommand the agent runs when it is done with the topic. 結果を渡す reads the topic file directly when no server is running. ラウンドを出す, 待つ and 返す fail with a message telling the agent to start the server when none is running; starting it resumes the topic. Overturn if: the person wants 終える to stop the server, or the short commands to start one on their own.
15. **The repository top is `git rev-parse --show-toplevel` run in the start directory.** A git worktree therefore has its own top and its own topics, as the specification's "リポジトリのトップのパス" reads literally. Outside a git repository the start directory is used. Overturn if: the person wants worktrees of one repository to share topics.
16. **No visual reference is committed.** The mocks the screen was designed from stay outside the repository; the implementer works from the specification's prose. Overturn if: the person wants a copy of the last mock committed under `docs/mocks/` with neutral sample data (it must not carry the brainstorm's own text).

## Scope of change

`Cargo.toml`, `Cargo.lock`, `src/`, `crates/`, `web/`, `skills/shoryo/`, `tests/`, `scripts/`, `.github/`, `package.json` and `package-lock.json` at the root (for Playwright only), `README.md` (new), `PROJECT.md` (Stack and layout, Commands; each step that adds a command or a part updates it). Nothing in `docs/spec/` or `CONTEXT.md` changes: if a step needs a change there, stop (see Stop conditions).

## Step order and prerequisites

The order runs from the tightest constraint outward. The domain comes first because every later step reads and writes its state, and it is the only part that can be tested without a server or a browser. The server and CLI come next so the screen has a real API to call. The screen is split by what the person sees, in the order a round is used: answer, ask back, look back, see the shape. The skill is written last of the code because it documents the final formats. Release comes after everything works. The real brainstorm closes the plan because the specification's only success measure is the person's own experience ([受け持ち](../spec/skill.md#受け持ち)).

1. Workspace
2. Topic state and its file
3. Rounds: validation and applying
4. Events and the pre-submit state
5. Derived views: chains, map, marks
6. Server and CLI
7. Screen: the current round
8. Screen: asks, replies and diagrams
9. Screen: past rounds, decisions, review rounds, the ended state
10. Screen: map, finished picture, moving around
11. Browser tests in CI
12. The shoryo skill
13. Release
14. A real brainstorm

Each step needs the ones before it. Step 8 comes before 9 and 10 because both draw replies or diagrams with what step 8 builds; steps 9 and 10 do not need each other.

## Steps

## Step 1 — The repository is a workspace with the three crates

Purpose: give every later step its place without changing behavior. Specification: [作り方と配り方](../spec/server.md#作り方と配り方).
Prerequisites: none.
May change: `Cargo.toml`, `Cargo.lock`, `src/`, `crates/shoryo-core/`, `crates/shoryo-server/`, `crates/shoryo-webview/`, `web/` (an empty `index.html` placeholder is enough), `scripts/check.sh`, `scripts/check-domain-purity.sh`, `PROJECT.md` (Stack and layout, Commands).
Done when: the workspace declares edition, `rust-version`, license and lints once under `[workspace.*]` and every member and the root package inherit them (skill `ba0918-rust`, `references/project-setup.md`); each library crate root carries `#![deny(clippy::print_stdout, clippy::print_stderr)]`; `scripts/check.sh` runs the gates with `--workspace` and calls `scripts/check-domain-purity.sh`, which fails when `crates/shoryo-core/src` uses `axum`, `hyper`, `tokio`, `std::fs`, `std::net`, `std::process` or `std::time::SystemTime`; `PROJECT.md` describes the workspace and the commands as they now are.
Shown by: check — `scripts/check.sh` exits 0; `scripts/check-domain-purity.sh` exits 0, and exits non-zero after temporarily adding `use std::fs;` to a core file (revert it).
Left to the implementer: module names inside each crate.
Stop and hand back if: none.

## Step 2 — The topic state can be built, saved and loaded

Purpose: one type that holds everything [状態データ](../spec/server.md#状態データ) lists, and its JSON form. Specification: [状態データ](../spec/server.md#状態データ), [見直しの問い直し](../spec/skill.md#見直しの問い直し), [過去のラウンド](../spec/screen.md#過去のラウンド).
Prerequisites: step 1.
May change: `crates/shoryo-core/`.
Done when: the state type covers every item in [状態データ](../spec/server.md#状態データ), with stable per-topic IDs for questions, decisions, asks and events; a decision keeps its history, so the content it had as of any round can be computed after it was revised; the state round-trips through JSON unchanged; serialisation happens in core and the file writing happens in the server (step 6).
Shown by: test — `state_round_trips_through_json`, `decision_content_as_of_an_earlier_round_is_its_content_then`, `ids_are_never_reused_in_a_topic`.
Left to the implementer: field names (provisional answer 9), the exact shape of the history.
Stop and hand back if: an item in [状態データ](../spec/server.md#状態データ) cannot be represented without inventing behavior the specification does not state.

## Step 3 — A round is checked before it is accepted, and applying it updates the records

Purpose: the rules of [ラウンドを出す](../spec/server.md#ラウンドを出す) and how a round changes the state. Specification: [ラウンドを出す](../spec/server.md#ラウンドを出す), [状態データ](../spec/server.md#状態データ), [送られた答えの扱い](../spec/skill.md#送られた答えの扱い), [見直しの問い直し](../spec/skill.md#見直しの問い直し), [終える](../spec/server.md#終える).
Prerequisites: step 2.
May change: `crates/shoryo-core/`.
Done when: a round is refused, with a reason naming the question or option, when a question has not exactly one recommended option, an option lacks its consequence text, a prerequisite names a decision not in the records, or the previous round is not yet submitted; an accepted round becomes the current round, replaces the records it carries except the in-review marks, applies the review conclusions and the confirmed points it carries, and reopens an ended topic.
Shown by: test — `round_with_two_recommended_options_is_refused`, `round_with_option_lacking_consequence_is_refused`, `round_naming_unknown_decision_is_refused`, `round_before_previous_is_submitted_is_refused`, `round_keeps_in_review_marks_set_on_the_screen`, `review_conclusion_unchanged_clears_in_review`, `review_conclusion_changed_records_revision`, `round_after_end_reopens_topic`.
Left to the implementer: the wording of refusal reasons (they are returned to the agent, not shown to the person).
Stop and hand back if: none.

## Step 4 — Events and the pre-submit state follow the round-trip rules

Purpose: everything the person does before sending, and the event log 待つ reads. Specification: [待つ](../spec/server.md#待つ), [返す](../spec/server.md#返す), [終える](../spec/server.md#終える), [まとめて送る](../spec/screen.md#まとめて送る), [先送りのラベル](../spec/screen.md#先送りのラベル), [まだ開いてない印](../spec/screen.md#まだ開いてない印), [仮決めの一覧](../spec/screen.md#仮決めの一覧), [見直したい](../spec/screen.md#見直したい).
Prerequisites: step 3.
May change: `crates/shoryo-core/`.
Done when: these operations change the state and append events as the specification says — choosing an option, writing a note, setting and clearing the defer switch, opening a card, swapping a question's class, asking (new or as a follow-up), 見直したい and stopping it, submitting; reading events returns all unacknowledged ones and acknowledging removes them; a reply is accepted for an ask in a submitted round and stored with it; after submit, answers, asks and swaps are refused until the next round; after end, waiting and replying are refused, and asks still waiting get "no reply"; submit records which human-decided questions were sent unopened (opening or touching clears the mark; resetting to the recommendation does not bring it back; swapping a question's class is not touching it).
Shown by: test — one test per operation's observable effect, named for it (for example `submit_marks_untouched_human_question_as_sent_unseen`, `unacknowledged_events_are_returned_again`, `acknowledged_events_are_not_returned`, `reply_to_ask_in_submitted_round_is_kept_with_that_round`, `answers_are_refused_after_submit`, `deferred_question_is_not_an_answer`, `swapped_untouched_question_keeps_unopened_mark`, `stopping_review_request_removes_in_review`, `wait_is_refused_after_end`, `pending_ask_shows_no_reply_after_end`).
Left to the implementer: none.
Stop and hand back if: none.

## Step 5 — Chains, the map and the review marks are computed from the state

Purpose: the views the screen draws, computed where they can be tested without a browser. Specification: [前提の行](../spec/screen.md#前提の行), [点と線](../spec/screen.md#点と線), [道筋と全部](../spec/screen.md#道筋と全部), [見直しの目印](../spec/screen.md#見直しの目印).
Prerequisites: step 4.
May change: `crates/shoryo-core/`.
Done when: core returns, for a question, its prerequisite chains (direct prerequisites, each followed back up to three names, taking the first prerequisite where one branches); returns the map as nodes, edges and columns (decision nodes carrying their question, current-round question nodes in the current column, edge names from the question where the edge's target was decided, rejected options from their question's prerequisites or alone in that question's column, the order within a column chosen to reduce crossings); returns the path set for a selected node; returns which nodes carry a review mark (direct dependents of a revised decision, cleared by a confirmed point).
Shown by: test — `chain_follows_first_prerequisite_up_to_three_names`, `chain_lists_each_direct_prerequisite_separately`, `map_names_edge_by_question_of_its_target`, `rejected_option_without_prerequisite_sits_alone_in_its_column`, `path_excludes_nodes_that_are_not_prerequisites`, `review_mark_reaches_direct_dependents_only`, `confirmed_point_clears_review_mark`. The crossing reduction itself is not tested (no specification states a measure); it is checked by eye in step 10.
Left to the implementer: the crossing-reduction method.
Stop and hand back if: none.

## Step 6 — The server runs a topic and the agent's commands work end to end

Purpose: start-up, the data directory, the HTTP API and the CLI subcommands. Specification: [議題と起動](../spec/server.md#議題と起動), [出力](../spec/server.md#出力), [待ち受け先](../spec/server.md#待ち受け先), [開き直し](../spec/server.md#開き直し), [往復](../spec/server.md#往復) and its subsections, [記録の置き場所と寿命](../spec/server.md#記録の置き場所と寿命).
Prerequisites: step 5.
May change: `crates/shoryo-server/`, `crates/shoryo-webview/`, `src/`, `tests/`, `Cargo.toml`, `Cargo.lock`, `PROJECT.md` (Commands: how to run it locally).
Done when: starting a topic prints the URL and, for each round, "round N is on the screen" with the URL, and nothing of the round's content; the server listens on 127.0.0.1 unless `--bind` says otherwise and warns about plain HTTP then; `--port` fixes the port, otherwise a free port is used; the topic directory, lock and endpoint file follow provisional answers 2, 4 and 5; ラウンドを出す, 待つ (with acknowledgement and an optional time limit), 返す, 終える and 結果を渡す work through the CLI; the page API serves the state and accepts the person's operations; the page receives changes by SSE without reloading; two browser tabs see the same state; a restart of the same topic restores everything; nothing is written inside the repository; the server's lifetime follows provisional answer 14; `--version` prints the package version.
Shown by: test — integration tests in `tests/` that start the built binary with a temporary data directory (override `dirs` through `XDG_DATA_HOME` on Linux) and drive it through the CLI and the page API: `second_start_of_same_topic_is_refused`, `restart_restores_rounds_and_pre_submit_state`, `round_output_contains_no_question_text`, `default_bind_is_loopback`, `bind_elsewhere_prints_plain_http_warning`, `events_while_not_waiting_are_received_later`, `reply_reaches_open_page_without_reload` (an SSE client in the test), `end_refuses_while_a_round_is_unsent`, `result_is_available_after_end`, `result_reads_the_file_when_no_server_runs`, `ended_topic_page_stays_served_until_stop`, `data_is_written_outside_the_repository`.
Left to the implementer: command and flag names (delegated), URL paths under the secret, the SSE message shape.
Stop and hand back if: a needed crate cannot be added at a version that builds on `rust-version` (raising `rust-version` is the person's decision).

## Step 7 — The person can read and answer the current round

Purpose: the current-round tab as [今のラウンド](../spec/screen.md#今のラウンド) describes, without asks. Specification: [画面の構成](../spec/screen.md#画面の構成), [並び](../spec/screen.md#並び), [たたんだカード](../spec/screen.md#たたんだカード), [答え欄の初期値](../spec/screen.md#答え欄の初期値), [この答えだと](../spec/screen.md#この答えだと), [前提の行](../spec/screen.md#前提の行), [仮決めの一覧](../spec/screen.md#仮決めの一覧), [先送りのラベル](../spec/screen.md#先送りのラベル), [まだ開いてない印](../spec/screen.md#まだ開いてない印), [まとめて送る](../spec/screen.md#まとめて送る).
Prerequisites: step 6.
May change: `web/`, `crates/shoryo-server/` (only to serve what the page needs), `package.json`, `package-lock.json`, `scripts/test-web.sh`, `web/tests/`.
Done when: the page has the four tabs and the topic header with fixed text in English; cards start folded with the listed contents and open to the rest; the recommended option is preselected; the consequence text follows the selection; the chain shows short names and opens the full text; the provisional list changes in place, opens into a card, and swaps with the cards; the defer switch and the unopened mark work; sending sends everything once and locks the round.
Shown by: test — Playwright tests in `web/tests/` against the real binary with a fixture round, run by `scripts/test-web.sh`: `four_tabs_with_english_fixed_text`, `card_starts_folded_and_opens_to_premise_and_option_details`, `recommended_option_is_preselected`, `consequence_follows_selected_option`, `chain_shows_short_names_and_opens_full_text`, `provisional_row_opens_into_a_card`, `swap_moves_question_between_cards_and_list`, `unopened_mark_and_count_disappear_when_touched`, `send_locks_the_round`.
Left to the implementer: layout and styling within the specification; the visual details are the person's later call ([未決](../spec/screen.md#未決)).
Stop and hand back if: Playwright cannot run in the development environment (the person may need to install a browser dependency).

## Step 8 — The person can ask back and read replies with diagrams

Purpose: [聞き返し](../spec/screen.md#聞き返し) and [図](../spec/screen.md#図). Specification: [聞き返し](../spec/screen.md#聞き返し), [図](../spec/screen.md#図), [返す](../spec/server.md#返す).
Prerequisites: step 7.
May change: `web/`, `web/tests/`, `crates/shoryo-server/` (only to serve what the page needs).
Done when: the three ways to ask work; a card shows "writing" until its reply arrives; other cards stay usable meanwhile; the thread has the three display states, starting at latest-only; a reply's grid diagram is drawn at the given grid positions with edge names clear of the lines and reverse edges offset.
Shown by: test — `two_asks_pending_while_other_cards_stay_usable`, `reply_appears_under_its_card`, `thread_starts_latest_only_and_switches_to_all_and_hidden`, `follow_up_attaches_to_chosen_reply`, `diagram_nodes_sit_at_given_grid_positions`, `reverse_edges_do_not_overlap`.
Left to the implementer: the wording of the quick-ask buttons beyond the three examples in the specification.
Stop and hand back if: none.

## Step 9 — Past rounds, the decisions tab, review rounds and the ended state

Purpose: everything the person uses to look back and to change a decision. Specification: [過去のラウンド](../spec/screen.md#過去のラウンド), [決まったこと](../spec/screen.md#決まったこと), [見直したい](../spec/screen.md#見直したい), [直したこと](../spec/screen.md#直したこと), [終える](../spec/server.md#終える).
Prerequisites: step 8.
May change: `web/`, `web/tests/`, `crates/shoryo-server/` (only to serve what the page needs).
Done when: a past round shows its cards as answered, with replies and diagrams, decisions as of that round, and marks for revised decisions and answers sent unseen; the decisions tab lists the seven kinds; 見直したい can be pressed and stopped from every tab, and a decision in review carries the in-review mark in every card's chain; a review round shows its fixed items first, and each fixed item that changed a decision has its own 見直したい; an ended topic shows its state and blocks sending, asking, 見直したい and swaps.
Shown by: test — `past_round_shows_replies_and_diagrams_as_answered`, `past_round_shows_decision_as_of_that_round`, `answer_sent_unseen_is_marked_in_past_round_and_decisions_tab`, `review_request_can_be_stopped_before_next_round`, `decision_in_review_is_marked_in_chains`, `review_round_lists_fixed_items_before_cards`, `fixed_item_that_changed_a_decision_offers_review_request`, `ended_topic_blocks_all_input`.
Left to the implementer: none.
Stop and hand back if: none.

## Step 10 — The map, the finished picture, and moving around

Purpose: [地図](../spec/screen.md#地図) and [完成図](../spec/screen.md#完成図) on the screen, and not losing one's place. Specification: [地図](../spec/screen.md#地図) and its subsections, [完成図](../spec/screen.md#完成図), [画面の構成](../spec/screen.md#画面の構成), [見直したい](../spec/screen.md#見直したい).
Prerequisites: step 8 (the renderer) and step 5 (the computed map).
May change: `web/`, `web/tests/`, `crates/shoryo-server/` (only to serve what the page needs).
Done when: the map draws the computed nodes, edges and columns in both ranges, with rejected options dashed and decisions in review marked; hovering a node shows its details over the map without moving anything; every jump (from the map, from a past-round link) highlights where it landed, and a back control returns to where it came from, for the map with its range and selection; review marks show; the finished picture opens from every tab, is drawn with the step 8 renderer, and draws empty slots dashed.
Shown by: test — `hover_details_do_not_move_the_map`, `jump_highlights_the_destination_card`, `back_from_past_round_restores_map_range_and_selection`, `rejected_options_are_dashed`, `decision_in_review_is_marked_on_the_map`, `finished_picture_opens_from_every_tab`, `path_range_shows_only_prerequisites`, `review_marks_appear_on_dependents`, `finished_picture_draws_empty_slots_dashed`. The crossing reduction and the overall look are checked by the person in step 14.
Left to the implementer: none.
Stop and hand back if: none.

## Step 11 — Browser tests run in CI

Purpose: put steps 7–10's tests behind the same gate as the Rust tests (provisional answer 10). Specification: none beyond the gates the project already runs (skill `ba0918-ci`).
Prerequisites: steps 7–10.
May change: `.github/workflows/ci.yml`, `.github/dependabot.yml` (add the npm ecosystem with the same cooldown), `scripts/test-web.sh`.
Done when: a second job in `ci.yml` installs the Node version written in the workflow, installs Playwright's Chromium with its system dependencies, builds the binary and runs `scripts/test-web.sh`; every external action is pinned by full commit hash with its version as a comment; the job has a time limit and read-only permissions.
Shown by: check — `actionlint`, `zizmor .github/workflows/`, a search for `uses:` lines not pinned to a 40-character hash (none), then the run of the pushed branch on GitHub with conclusion "success".
Left to the implementer: whether the browser job reuses the Rust job's build artifact or builds again.
Stop and hand back if: Playwright's browser installation needs a privilege the hosted runner does not give.

## Step 12 — The shoryo skill

Purpose: the instructions the agent follows. Specification: every heading of [shoryo skill](../spec/skill.md).
Prerequisites: steps 6–10 (the formats must be final).
May change: `skills/shoryo/`, `README.md`, `PROJECT.md` (Stack and layout: what is now built), `tests/` (the examples test).
Done when: `skills/shoryo/SKILL.md` (English) covers each heading of [shoryo skill](../spec/skill.md): when to use shoryo, starting a topic and the one line in the conversation, classifying questions, writing a round, the waiting loop, replying, handling sent answers, deferred questions, review requests, review rounds, the finished picture, ending; `skills/shoryo/references/` documents the commands, the round, reply and result JSON formats, and the diagram text format, each with one complete example the binary accepts; `README.md` says what shoryo is, how to install the binary and the skill, and links the specification.
Shown by: artifact — `skills/shoryo/SKILL.md` and `skills/shoryo/references/*.md`; every JSON example in the references is fed to the binary by a test in `tests/` (`skill_examples_are_accepted`) so the documentation cannot drift from the code.
Left to the implementer: how the references are split into files.
Stop and hand back if: none.

## Step 13 — A release can be cut and installed with mise

Purpose: [作り方と配り方](../spec/server.md#作り方と配り方). Specification: [作り方と配り方](../spec/server.md#作り方と配り方).
Prerequisites: step 12.
May change: `.github/workflows/` (a release workflow or job), `CHANGELOG.md` (new), `Cargo.toml` (version only).
Done when: a release workflow builds the static binary for provisional answer 11's target, packages it with its checksum, and attaches both to a GitHub release created for a version tag, following the skills `ba0918-release` and `ba0918-ci`; the workflow passes `actionlint` and `zizmor`.
Shown by: external — the person creates the first release (a tag is permanent once published), then installs it with `mise use -g github:ba0918/shoryo` on their machine and runs `shoryo --version`, which prints the released version.
Left to the implementer: whether to use a hand-written workflow (as kakoi) or a release tool, as long as every external action is pinned.
Stop and hand back if: none.

## Step 14 — A real brainstorm runs through shoryo

Purpose: the one check the specification leaves to the person: whether shoryo stops the habit of answering 推奨 without reading ([受け持ち](../spec/skill.md#受け持ち), [完成図](../spec/screen.md#完成図)).
Prerequisites: step 13.
May change: nothing in the repository unless the person asks for fixes.
Done when: the person has run at least one brainstorm with the installed binary and skill, including one ask with a diagram, one deferred question, one 見直したい, and one review round.
Shown by: external — the person runs it and says whether the screen, the map and the finished picture work for them. Also note from that run whether the finished picture fit on one picture for that topic, which [完成図を描く](../spec/skill.md#完成図を描く) leaves unverified. Findings go to a new brainstorm, not into this branch.
Left to the implementer: none.
Stop and hand back if: none.

## Verification map

| Specification section | Steps |
|---|---|
| [画面の構成](../spec/screen.md#画面の構成), [今のラウンド](../spec/screen.md#今のラウンド) and its subsections except 聞き返し and 直したこと | 7 (4 for the state rules) |
| [聞き返し](../spec/screen.md#聞き返し), [図](../spec/screen.md#図) | 8 |
| [過去のラウンド](../spec/screen.md#過去のラウンド), [決まったこと](../spec/screen.md#決まったこと), [見直したい](../spec/screen.md#見直したい), [直したこと](../spec/screen.md#直したこと) | 9 (2–4 for the state rules) |
| [地図](../spec/screen.md#地図) and its subsections | 5, 10 |
| [完成図](../spec/screen.md#完成図) | 10, 14 |
| [起動](../spec/server.md#起動) and its subsections, [記録の置き場所と寿命](../spec/server.md#記録の置き場所と寿命) | 6 |
| [往復](../spec/server.md#往復) and its subsections | 3, 4, 6 |
| [状態データ](../spec/server.md#状態データ) | 2, 6 |
| [作り方と配り方](../spec/server.md#作り方と配り方) | 1, 13 |
| [shoryo skill](../spec/skill.md), every heading | 12, 14 |

## Left to the implementer

Plan-wide: module and function names; internal structure of each crate; JSON field names and command names (delegated by [任せたこと](../spec/server.md#任せたこと), documented in step 12); styling within the specification.

## Stop conditions

Beyond the general ones (skill `ba0918-cycle`):

- A step needs a change to `docs/spec/` or `CONTEXT.md`, or contradicts them.
- A provisional answer above turns out to break a specification requirement.
- A dependency needs a newer Rust than `rust-version`.

## Test command

- Rust gates and tests: `scripts/check.sh` (format check, clippy with warnings as errors, `cargo test --workspace --locked`, the domain-purity check).
- Browser tests: `scripts/test-web.sh` (builds the binary, then runs `npx playwright test`). Steps 7–10 add it; step 11 runs it in CI.

## Out of scope

- Rewriting the ba0918 or kotowari brainstorm to call shoryo ([作らないもの](../spec/skill.md#作らないもの)).
- Writing specifications from the result data (the workflow's job).
- Counting how often answers are the recommendation ([作らないもの](../spec/screen.md#作らないもの)).
- Visual details the person decides after using it ([未決](../spec/screen.md#未決)).
- Builds for targets other than provisional answer 11's.
