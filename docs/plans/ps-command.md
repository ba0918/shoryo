# shoryo ps

Implementation plan against approved specification commit `ec32372`.
This document proposes future work; it is not implementation evidence or approval.

## Goal

A person can run `shoryo ps` to find the page URL and the PID of every running topic server (and, with `-a`, every stopped topic) without asking the LLM, and the LLM can do the same through `--format json`.

## Specification

Read these sections before the steps that rest on them; the specifications remain authoritative and this plan does not restate them.

- [議題の一覧](../spec/ps.md), all of it: [出す範囲と対象](../spec/ps.md#出す範囲と対象), [範囲の外で動いているサーバーの知らせ](../spec/ps.md#範囲の外で動いているサーバーの知らせ), [動いているかの決め方](../spec/ps.md#動いているかの決め方), [止め方](../spec/ps.md#止め方), [出力の形](../spec/ps.md#出力の形), [text](../spec/ps.md#text), [json](../spec/ps.md#json), [URL の secret](../spec/ps.md#url-の-secret), [値の用意](../spec/ps.md#値の用意), [読めない議題](../spec/ps.md#読めない議題), [並び順](../spec/ps.md#並び順), [作らないもの](../spec/ps.md#作らないもの), [任せたこと](../spec/ps.md#任せたこと).
- [起動と往復と記録](../spec/server.md): [議題と起動](../spec/server.md#議題と起動) (how the current directory picks the repository), [設定ファイル](../spec/server.md#設定ファイル) (command output stays English), [状態データ](../spec/server.md#状態データ) (the topic's creation time), [記録の置き場所と寿命](../spec/server.md#記録の置き場所と寿命) (data is grouped per repository and never deleted).
- [shoryo skill](../spec/skill.md): [議題名がわからない再開](../spec/skill.md#議題名がわからない再開), [生成と読み出しを必要な範囲に絞る](../spec/skill.md#生成と読み出しを必要な範囲に絞る).

Use [PROJECT.md](../../PROJECT.md), [AGENTS.md](../../AGENTS.md) and [CONTEXT.md](../../CONTEXT.md) for commands, rules and terminology. In code, messages and help text the unit is a "topic", never a "session".

## Approach and why

`ps` is a read-only scan of the per-user data directory. It never talks to a server, so it adds no API and cannot hang on an unresponsive server. Everything it needs either already sits in the topic's files (the lock holding the PID, the endpoint file holding the URL, the state file holding title, repository, `ended` and rounds) or is added to those same files in Step 2.

Where the code lives:

- The scan, and the record it produces for one topic, belong in `shoryo-server`, next to `location.rs` and `store/`. That crate already owns the data directory layout, the repository key, the lock's liveness check (`store::lock::running_holder`) and the endpoint file. A new module (for example `crates/shoryo-server/src/listing.rs`) exposes one function that, given the data root, returns one record per topic directory with every value the [json](../spec/ps.md#json) table names, plus the warnings for unreadable topics.
- Filtering by range and target, ordering, the outside-range count, and rendering text or JSON are pure functions over those records. Keep them free of I/O so they are unit-testable. They live in `shoryo-server` too (the listing module or a sibling), together with the record's `Serialize` derive and the `~` shortening: the root crate depends only on clap, serde_json, shoryo-server and ureq, and must not gain `serde` or `dirs`.
- `src/main.rs` only parses arguments, calls the scan and the renderer, writes stdout and the warnings to stderr, and maps errors to exit status 1 as the other commands do.
- `shoryo-core` is untouched: it must stay free of I/O (`scripts/check-domain-purity.sh`), and nothing here is domain logic of a topic.

The order of steps puts the listing with the values that exist today first. A person can already find URLs and PIDs after Step 1. Step 2 then adds the values that need new stored data. Every assertion is made on what `ps` (or `shoryo result`) prints. Tests touch the topic's files only to set up a condition the specification names: an old state file, an old server's endpoint file, a corrupt file, or a modification time.

Test fixtures produce a "stopped" topic by `shoryo stop` or by dropping the harness's `Server` (which kills and reaps it). After sending any signal to a server's PID, a test must reap the child (`Server::finish` or a new wait helper) before running `ps` or `start`. A killed child that is not yet reaped keeps `/proc/<pid>`, so it still looks running to `running_holder`. That is a harness artifact, not a product fault, and must not be "fixed" in production code. Use distinct topic names across simulated repositories, because `Env::state_file` matches by topic name across the whole data directory.

### Reuse layers

| Layer | Adopt or build | Reason |
|---|---|---|
| Argument parsing (`-a`/`--all`, `--all-repos`, `--format`) | Adopt clap (installed) | Every other subcommand already uses its derive API; `ValueEnum` gives the closed `text`/`json` choice. |
| Finding topic directories and the current repository's directory | Adopt `TopicLocation::find` and `TopicName::parse` (codebase) plus `std::fs::read_dir` | The repository key and data root are already computed there; reusing them keeps `ps`'s range identical to the other commands'. |
| Is the server running, and its PID | Adopt `store::lock::running_holder` (codebase), made visible to the scan | The specification requires the same judgment as the other commands. |
| URL and server facts | Adopt `store::Endpoint` and `read_endpoint` (codebase), extended in Step 2 | The endpoint file is already where a running server's facts live and it is removed on stop. |
| Reading the topic | Adopt `store::read_state` plus serde_json (installed) | Reading leniently into `serde_json::Value` lets a partly unexpected file still yield the fields it has; the strict `load_existing` would discard everything on one bad field. |
| Times | Adopt `clock::now` and its RFC 3339 formatter (codebase), exposed for `SystemTime` values | Same format as every other stored time; no date crate needed. |
| Last write time | Adopt `std::fs::Metadata::modified` (standard library) | The state file is always replaced whole, so its modification time is the last write. |
| JSON output | Adopt serde / serde_json (installed) | Derive `Serialize` on the record with explicit `null`s. |
| Text table | Build, a few lines, in `shoryo-server` | Five to eight padded columns. A table crate (comfy-table, tabled) is the ecosystem staple, but the text form is declared free to change and not worth a new dependency. Wide-character alignment is not a contract. |
| Home directory shortening | Adopt `dirs::home_dir` (installed in `shoryo-server`, used there) | Already a dependency for the data directory; the root crate does not have it. |
| Sending signals in tests | Adopt the platform's `kill` command through `std::process::Command` | The release targets Linux only (`store/lock.rs` relies on `/proc` already). No libc or nix dev-dependency is needed. |

### Provisional answers and overturn conditions

The specification is silent on these; the plan takes them so the implementer does not have to.

1. **When `created_at` is set, and how it is stored.** The server keeps a flag meaning "the state file was absent when this server loaded the topic". On the first successful save while that flag is set, it sets `created_at` to `clock::now()` before writing, and clears the flag. Checking `created_at.is_none()` alone is wrong: it would backfill old topics. A topic that already had a state file without `created_at` keeps it absent forever. A backfilled value would be a later time presented as the creation time. The field is `Option<Timestamp>` with `#[serde(default, skip_serializing_if = "Option::is_none")]`: absent, not `null`, when unset. This leaves old files' keys unchanged. The first save is usually the first round. It can also be `shoryo end` on a topic with no rounds, which is still "the first time the state is written" ([状態データ](../spec/server.md#状態データ)).
   **Overturn if:** the person wants old topics to receive a creation time on their next save (this would also need a specification change, since [値の用意](../spec/ps.md#値の用意) says they stay null).
2. **New endpoint fields are optional on read.** `started_at`, `version` and `repository` are added to `Endpoint` with serde defaults. An endpoint file written by an older running binary still reads, and those three values become `null`.
   **Overturn if:** a reason appears to refuse old endpoint files. That would break `round`, `wait` and the other commands against an older running server, so it is unlikely.
3. **`version` is the binary's package version** (`env!("CARGO_PKG_VERSION")`, for example `0.1.4`, without a `shoryo ` prefix). It is written by the server at start.
   **Overturn if:** the person wants the full `shoryo --version` line.
4. **What counts as a topic directory.** It is every directory directly under a repository directory (`<data dir>/shoryo/<repository key>/`) whose name parses as a `TopicName`. Other entries are skipped silently: files, invalid names, and a repository directory that is not a directory. A topic directory with no state file and no live lock is still a topic ([出す範囲と対象](../spec/ps.md#出す範囲と対象)'s pre-round case).
   **Overturn if:** stray directories created by other tools show up and the person wants them hidden, for example by requiring a `lock.guard` or `state.json` inside.
5. **No data directory yet** (`<data dir>/shoryo` absent) means zero topics. The output is the zero-result line or `[]`, and the exit status is 0. When the platform has no data directory at all, `ps` fails with the existing `NoDataDirectory` error and exit status 1, like every other command.
   **Overturn if:** the person prefers a warning when nothing has ever been started.
6. **Where `repository` and the server's values come from.** Every endpoint value (`url`, `started_at`, `version`, `repository`) is read only through `read_endpoint`. It checks `running_holder` first, so these values count only while the lock names a live process. An endpoint file left behind by a SIGKILL is ignored. `repository` is the state file's value first, then the live endpoint's, otherwise `null`. Range membership never uses this value: it uses the repository directory the topic sits in ([出す範囲と対象](../spec/ps.md#出す範囲と対象)). A running topic whose lock is live but whose endpoint file is not written yet (the instant after `start` takes the lock) has a `server` object with `pid` set and `url`, `started_at` and `version` `null`. `server: null` is reserved for stopped topics.
   **Overturn if:** the two repository sources disagree in practice. Neither should, since both are written from the same `TopicLocation`.
7. **What is unreadable, and how it is reported.** Any error reading the state file, the endpoint file, the lock or the state file's metadata makes that value unreadable. This covers content that is not JSON, fields with unexpected types, and I/O errors other than "not found", such as permission denied or a directory in the file's place. None of these may propagate out of the scan. Each yields `null` for exactly the values it prevented, and one stderr line `shoryo: warning: <topic dir>: <reason>`, in the existing warning style. The listing goes on and the exit status stays 0. Values that could still be read stay filled. In particular, `updated_at` comes from the state file's modification time even when its content is corrupt, so ordering is unaffected. A missing state file is normal, not unreadable. A missing `created_at` is normal, with no warning.
   **Overturn if:** the person wants one summary warning instead of one per topic.
8. **The order among equal times.** After running-first and newest-`updated_at`-first (with `null` last within each group), ties break by repository path and then topic name. A `null` repository path uses the repository directory's name in its place. This keeps the output deterministic.
   **Overturn if:** the person wants another secondary order.
9. **`latest_round` and `ended` for a topic whose state file has no rounds.** `latest_round` is `null` when the stored topic has no round. When the state file is absent, `title`, `ended`, `latest_round`, `created_at` and `updated_at` are all `null` ([json](../spec/ps.md#json)'s success condition).
   **Overturn if:** none expected; this is the specification's case spelled out.
10. **The outside-range count.** The count scans every repository even without `--all-repos`, but only running servers outside the chosen range are counted. With `--all-repos` there is no outside, so no note.
    **Overturn if:** none expected.

## Scope of change

- `src/main.rs` (the `Ps` subcommand, calling the scan and the renderer, and printing).
- `crates/shoryo-server/src/` — a new listing module; `store/mod.rs` (`StoredTopic.created_at`, setting it on first save), `store/endpoint.rs` (new fields), `store/lock.rs` (visibility of `running_holder`), `lib.rs` (writing the new endpoint fields, exporting the listing), `clock.rs` (exposing the formatter for `SystemTime`), `app/shared.rs` only if the first-save hook lands there.
- `tests/round_trip/` — a new `ps.rs` module registered in `main.rs`; `harness.rs` gains small helpers (a start directory per simulated repository, sending a signal to a server's PID).
- `skills/shoryo/SKILL.md`, `skills/shoryo/references/commands.md`.
- `README.md`, `README-ja.md`, `PROJECT.md`, `CHANGELOG.md`.

Nothing else. In particular, not `shoryo-core`, not `web/`, and not the specifications.

## Step order and prerequisites

Steps run in order: 1 → 2 → 3 → 4 → 5 → 6. Steps 2–4 build on the listing from Step 1. Step 5 documents what Steps 1–4 built. Step 6 is the final gate.

## Step 1 — `shoryo ps` lists topics with today's values

Purpose: the command exists with its arguments, range and target, running detection, both output forms, the zero-result line and the outside-range note, using only values the files already hold. Specification: [出す範囲と対象](../spec/ps.md#出す範囲と対象), [範囲の外で動いているサーバーの知らせ](../spec/ps.md#範囲の外で動いているサーバーの知らせ), [動いているかの決め方](../spec/ps.md#動いているかの決め方), [出力の形](../spec/ps.md#出力の形), [text](../spec/ps.md#text), [json](../spec/ps.md#json), [議題と起動](../spec/server.md#議題と起動).
Prerequisites: none.
May change: `src/main.rs`, `crates/shoryo-server/src/` (new listing and rendering module(s), `lib.rs`, `store/lock.rs` visibility of `running_holder` to `pub(crate)`, `store/mod.rs` exports, `clock.rs` to expose a `pub(crate)` RFC 3339 formatter for a `SystemTime`), `tests/round_trip/ps.rs`, `tests/round_trip/main.rs`, `tests/round_trip/harness.rs`.
Done when: `shoryo ps`, `-a`, `--all-repos` and `-a --all-repos` select as the table in [出す範囲と対象](../spec/ps.md#出す範囲と対象) says. The JSON array has every key of [json](../spec/ps.md#json): keys whose values do not exist yet (`created_at`, `server.started_at`, `server.version`) are `null`, and `updated_at` already comes from the state file's modification time. Text shows the PID and the whole URL of each running topic. Zero results print one line in text and `[]` in JSON, with exit status 0. The outside-range note appears only in text and only without `--all-repos`. `shoryo ps --help` says that the default form is text, unlike `wait` and `result`.
Shown by: test — in `tests/round_trip/ps.rs`, each RED first, then GREEN, then REFACTOR. A simulated repository is a start directory under the test's work directory (the harness's work directory is not a git repository, so the start directory itself is the repository):
- `ps_lists_only_running_topics_of_the_current_repository` — running topics in repositories A and B, and a stopped one in each. `ps` (JSON) in A returns only A's running topic, with its `pid` and `url`.
- `ps_all_and_all_repos_select_independently` — the same fixture under `-a`, `--all-repos` and `-a --all-repos` gives the rows the table in [出す範囲と対象](../spec/ps.md#出す範囲と対象) requires. In particular, `-a` in A never shows B's stopped topic, and `-a --all-repos` does. `-a` includes a topic stopped before its first round (only `topic` and `data_dir` non-null).
- `ps_json_rows_have_the_same_keys_whether_running_or_stopped` — under `-a --format json`, a running and a stopped topic have identical key sets, and the stopped topic's `server` is `null`.
- `ps_text_shows_pid_and_full_url` — the text output contains the running topic's PID and its URL exactly as `start` printed it.
- `ps_with_nothing_to_list_prints_one_line_or_an_empty_array` — in an empty data directory, text prints one non-empty line and JSON prints `[]`, both with exit status 0.
- `ps_text_notes_servers_running_in_other_repositories` — run from a directory with no topics while two servers run elsewhere and one stopped topic exists elsewhere. Text names the count 2 and `--all-repos`, with and without `-a` (stopped topics are not counted). JSON has no such note, and the same text run with `--all-repos` has no note.
- `ps_help_states_the_text_default` — `shoryo ps --help` mentions that the default format is text. ([出力の形](../spec/ps.md#出力の形) requires the help to say so. Assert only that the help names the `text` default, not its wording.)
Left to the implementer: module and type names; how the scan, the record and the rendering are split into modules inside `shoryo-server`; the help wording, as long as it states the text default; the text columns' widths, the title truncation length, and the English wording of the zero-result line and the note ([任せたこと](../spec/ps.md#任せたこと), [text](../spec/ps.md#text)).
Stop and hand back if: the repository key for the current directory cannot be obtained without starting a topic (it should come from `TopicLocation::find` with any valid name, taking the parent directory). If that turns out to create directories or files, stop rather than create them from `ps`: reading must not create data.

## Step 2 — creation time and the running server's facts

Purpose: the values that are not stored today appear in `ps`. Specification: [値の用意](../spec/ps.md#値の用意), [json](../spec/ps.md#json), [状態データ](../spec/server.md#状態データ).
Prerequisites: Step 1.
May change: `crates/shoryo-server/src/store/mod.rs`, `store/endpoint.rs`, `lib.rs`, `clock.rs`, `app/shared.rs`, the listing module, `tests/round_trip/ps.rs`, `tests/round_trip/harness.rs`.
Done when: a new topic has a non-null `created_at` in `ps` JSON after its first round, and `shoryo result <topic>` prints the same value in the stored data. A topic whose state file was written without `created_at` keeps `null` there, and `ps` prints no warning for it. As soon as a server starts, `server.started_at` and `server.version` are non-null, and `repository` is non-null even before the first round. An endpoint file without the new fields still lets `round`, `wait`, `stop` and `ps` work.
Shown by: test — RED, GREEN, REFACTOR:
- `created_at_is_recorded_when_the_state_is_first_written` — start a new topic: `created_at` is `null` before the first round. After `round`, it is an RFC 3339 UTC time no earlier than the test's clock before the round. Restarting the topic and sending another round leaves it unchanged.
- `topics_written_before_created_at_list_it_as_null_without_warning` — write a state file of the current format with the `created_at` key removed (derive it from a real topic's `shoryo result`), then list it: `created_at` is `null`, stderr has no warning, and starting the topic and sending a round does not add it.
- `a_started_server_lists_its_start_time_version_and_repository` — right after `start`, with no round: `server.started_at` is set, `server.version` equals the package version, and `repository` is the canonicalized start directory's path. `title`, `ended`, `latest_round`, `created_at` and `updated_at` are all `null`.
- `commands_still_reach_a_server_whose_endpoint_lacks_the_new_fields` — before the first round, while a server runs, rewrite its endpoint file without the three new keys. `ps` lists it with `started_at`, `version` and `repository` `null`. Then `round`, `wait --timeout 1` and `stop` all still work against it.
Use the harness's `clock_ms` and `stored_time` helpers for the time comparison.
Left to the implementer: where the first-save hook lives (`store::save`, `Shared::change_or`, or the `StoredTopic` value), as long as only a topic without a state file at start gets `created_at`; the field order in the files.
Stop and hand back if: adding `created_at` to `StoredTopic` makes an existing state file fail `load_existing` (for example through a `deny_unknown_fields` somewhere on the path), because that would make old topics unreadable, which [値の用意](../spec/ps.md#値の用意) forbids.

## Step 3 — stopping a listed server by its PID

Purpose: what [止め方](../spec/ps.md#止め方) promises is pinned by tests, including the unresponsive case [動いているかの決め方](../spec/ps.md#動いているかの決め方) describes. Specification: [止め方](../spec/ps.md#止め方), [動いているかの決め方](../spec/ps.md#動いているかの決め方).
Prerequisites: Steps 1 and 2.
May change: `tests/round_trip/ps.rs`, `tests/round_trip/harness.rs`. Production code only if a test below still fails after the child has been reaped (see the fixture rule under "Approach and why"). In that case, change the lock or shutdown path in `crates/shoryo-server/src/store/lock.rs` or `lib.rs`.
Done when: a server stopped through the PID `ps` printed disappears from `ps`, and the same topic starts again, after either SIGTERM or SIGKILL. A stopped (SIGSTOP) server is still listed with its PID and URL, and `ps` returns promptly.
Shown by: test — RED, GREEN, REFACTOR. These tests may pass on first run, because the code already handles the signals (`lib.rs` `stop_on_signal`, `store/lock.rs` stale-lock replacement). They are still required: the specification asks for round-trip tests of these conditions, and they pin the PID that `ps` prints, which is new:
- `sigterm_to_the_listed_pid_stops_the_server_and_the_topic_starts_again` — signal, reap, then `ps` no longer lists it and `start` succeeds.
- `sigkill_to_the_listed_pid_leaves_a_topic_that_starts_again` — signal, reap, then `ps` (without `-a`) no longer lists it although its lock and endpoint files remain, and `start` succeeds.
- `a_server_that_does_not_answer_is_still_listed_with_its_pid_and_url` — send SIGSTOP to the listed PID and run `ps`: the row is there with `pid` and `url`. Then send SIGKILL and wait for exit. SIGTERM would not act on a stopped process, so the test must not rely on it here.
Left to the implementer: how the harness sends signals (the `kill` command is the expected route) and waits for exit.
Stop and hand back if: the `kill` command is unavailable in the test environment and the only alternative is a new dev-dependency. Adding one is a dependency decision for the person.

## Step 4 — unreadable topics and ordering

Purpose: one broken topic never hides the others, and the rows come in the specified order. Specification: [読めない議題](../spec/ps.md#読めない議題), [並び順](../spec/ps.md#並び順).
Prerequisites: Steps 1 and 2.
May change: the listing module, the ordering function, `tests/round_trip/ps.rs`, `tests/round_trip/harness.rs`.
Done when: a topic with a corrupt state file is listed with `null` for what could not be read; when its server runs, its PID and URL are listed too. One warning naming that topic goes to stderr, the other topics are listed, and the exit status is 0. Under `-a`, running topics come before stopped ones. Within each group the rows go from newest `updated_at` to oldest, with `null` last, without grouping by repository under `--all-repos`.
Shown by: test — RED, GREEN, REFACTOR:
- `a_corrupt_topic_is_listed_with_a_warning_and_the_rest_still_appear` — one healthy stopped topic, one topic whose state file holds invalid JSON, and one running topic whose state file is corrupted while it runs (overwrite the file in place; the server keeps its in-memory state).
- `running_topics_come_first_then_newest_updates` — topics spread over two repositories, interleaved by time: a stopped topic updated most recently (repository B), an older running topic (A), an older stopped topic (A), a slightly newer stopped topic (B), and a stopped topic with no state file. Under `-a --all-repos` the running one comes first. Then the stopped ones follow in pure time order across both repositories, and the one without `updated_at` comes last. Set modification times with `std::fs::File::set_modified` rather than sleeping.
Left to the implementer: the wording of the warning after the `shoryo: warning: <topic dir>: ` prefix.
Stop and hand back if: none.

## Step 5 — the skill and the user-facing documents

Purpose: the LLM learns to find a topic whose name it does not know, and people learn that `ps` exists. Specification: [議題名がわからない再開](../spec/skill.md#議題名がわからない再開), [生成と読み出しを必要な範囲に絞る](../spec/skill.md#生成と読み出しを必要な範囲に絞る), [止め方](../spec/ps.md#止め方).
Prerequisites: Steps 1–4 (the documents describe the finished behavior).
May change: `skills/shoryo/SKILL.md`, `skills/shoryo/references/commands.md`, `README.md`, `README-ja.md`, `PROJECT.md`, `CHANGELOG.md`.
Done when:
- `SKILL.md` has a short section, in English, covering three points. When asked to resume a topic whose name is unknown, run `shoryo ps -a --format json` from the repository and match by title and update time. When more than one candidate fits, ask the person by title rather than guess. Then read `shoryo result <topic> --current` before resuming.
- `references/commands.md` has a `shoryo ps` row with its arguments and outputs, and says that `ps` reads files only and works from any directory.
- `README.md` and `README-ja.md` say, under "Use" / 「使い方」, that `shoryo ps` shows the URL and PID of running topics, and that a person may stop a left-over server with `kill <PID>`. Their specification lists link `docs/spec/ps.md`.
- `PROJECT.md`'s "Run locally" row mentions `ps` among the commands.
- `CHANGELOG.md` under `## [Unreleased]` has an `### Added` entry for `shoryo ps`, and one for the new `created_at` in the stored topic and the new endpoint fields. It also has a **BREAKING** entry under `### Changed` for the Rust API. `shoryo_server::store::StoredTopic` and `Endpoint` gain public fields, so code that builds them with struct literals no longer compiles. This follows the style of the 0.1.4 entry.
- No fenced block whose info string is read by `tests/round_trip/skill_examples.rs` (`json round`, `json reply`, `json output`, …) is added for `ps`, unless it is a real, accepted example.
Shown by: check — `cargo test --locked --test round_trip skill_examples` (the reference examples still pass); then a person reads the `SKILL.md` section and the README paragraphs against [議題名がわからない再開](../spec/skill.md#議題名がわからない再開). The specification verifies the skill by a human in a real brainstorm, so no automated test asserts the skill's wording.
Left to the implementer: wording and placement within each document.
Stop and hand back if: none.

## Step 6 — gates

Purpose: the change passes everything CI runs. Specification: none beyond the steps above.
Prerequisites: Steps 1–5.
May change: only fixes to files already in scope.
Done when: every command below exits 0.
Shown by: check — `scripts/check.sh` (format, clippy with warnings as errors, all tests, domain purity), then `scripts/test-web.sh`. The web tests are expected to be unaffected; they confirm that `start` still serves the page with the changed endpoint file.
Left to the implementer: none.
Stop and hand back if: a web test fails for a reason outside this change's scope.

## Verification map

| Specification section | Shown in |
|---|---|
| [出す範囲と対象](../spec/ps.md#出す範囲と対象) | Step 1 |
| [範囲の外で動いているサーバーの知らせ](../spec/ps.md#範囲の外で動いているサーバーの知らせ) | Step 1 |
| [動いているかの決め方](../spec/ps.md#動いているかの決め方) | Steps 1 and 3 |
| [止め方](../spec/ps.md#止め方) | Step 3 |
| [出力の形](../spec/ps.md#出力の形), [text](../spec/ps.md#text), [json](../spec/ps.md#json) | Steps 1 and 2 |
| [URL の secret](../spec/ps.md#url-の-secret) | Step 1 (the URL is the one `start` printed; nothing new is printed) |
| [値の用意](../spec/ps.md#値の用意), [状態データ](../spec/server.md#状態データ) | Step 2 |
| [読めない議題](../spec/ps.md#読めない議題) | Step 4 |
| [並び順](../spec/ps.md#並び順) | Step 4 |
| [議題名がわからない再開](../spec/skill.md#議題名がわからない再開) | Step 5 (human check); in a real brainstorm afterwards |

## Left to the implementer

Plan-wide: names of modules, types, functions and tests (the test names above may be adjusted to the file's conventions as long as each behavior keeps one test); internal structure and helper extraction; comment placement following `ba0918-placement`. Nothing about inputs, outputs, error behavior or stored fields is left open beyond what each step says.

## Stop conditions

Beyond each step's own conditions and the general ones of the cycle:

- A change to `shoryo-core` appears necessary.
- A new dependency, runtime or dev, appears necessary.
- Making `ps` work requires `ps` to write or create anything in the data directory.
- A specification section above turns out to contradict the code it describes (for example, a lock that does not hold the PID).

## Test command

The project fixes it: `cargo test --workspace --locked`, and `scripts/check.sh` for the full gate. For a fast loop on this work, use `cargo test --locked --test round_trip ps::`.

## Out of scope

Everything under [作らないもの](../spec/ps.md#作らないもの): showing the topic name on the screen or in the LLM's line, extending `stop`, an interactive stop, querying servers, and a separate `list` command. Also out of scope: deleting topic data, other operating systems, and a release or version bump. The release is a separate step, taken when the person decides to cut one.
