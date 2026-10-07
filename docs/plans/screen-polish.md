# Plan: stamps, settings, and a screen that reads like the mocks

## Goal

After this plan, the person answers rounds by stamping each question (provisional ones arrive pre-approved), switches the screen between English and Japanese and between light and dark from a thin fixed header with the choice kept per user, sees a "result" when a topic converges, and works on a screen whose visual hierarchy matches the mocks it was designed from, with the map labels, the map hover box and the ask field fixed.

## Specification

- [画面](../spec/screen.md)
- [起動と往復と記録](../spec/server.md)
- [shoryo skill](../spec/skill.md)
- [CONTEXT.md](../../CONTEXT.md)

The specification is in Japanese; links point at its headings. The revision this plan implements is the commit "docs: 画面の言語とテーマの切替、判子による決裁、収束したときの結果を仕様に足す" on branch `first-tool`.

## Approach and why

### What changes and why now

Running the first version on a real brainstorm showed four kinds of problems:

- The "sent unseen" record was wrong. It treated "never opened or touched" as "not read", but a folded card already shows enough to judge. The specification now replaces it with stamps ([判子](../spec/screen.md#判子)).
- The screen needs language and theme switches. It has many controls, and the person wants dark mode. See [ヘッダーと切替](../spec/screen.md#ヘッダーと切替) and [設定ファイル](../spec/server.md#設定ファイル).
- A round without questions showed only a send button. The specification now defines it as the [結果](../spec/screen.md#結果) of a converged topic.
- Four defects on the screen, found by the person:
  1. Map node labels overflow their boxes. Labels are cut by character count, and Japanese characters are about twice as wide.
  2. The map hover box is clipped by the map container and shows the literal text "null" for a current-round question.
  3. The screen reads far worse than the mocks. Controls outweigh the question and its options (details below).
  4. The ask field cannot take a newline.

Visual details remain the person's call after use ([未決](../spec/screen.md#未決)). Defects 1–4 are fixed here because they break readability. Final styling is reviewed by the person in step 8.

The person then reviewed the running screen in kemi and pinned these comments, quoted in substance:

- c1 (question card): "Elements are just stacked top to bottom; it does not feel designed. Reorganise and restructure it."
- c2 (the select box in the provisional list): "It is just placed there, not designed. There are really many things like this."
- c3 (Send all): "Show a confirmation dialog of what will be sent when Send all is pressed."
- c4 (Past rounds): "Past rounds do not keep the card form, so it is hard to see where one question ends."
- c5 (map): "Really bad. Lines and text overlap everywhere. The tooltip shown on clicking a box is clipped. Scrolling is hard, and impossible by touch. Zoom in and out inside the area is needed. Selecting an item changes the view and there is no way back to the previous state."

The common thread of c1 and c2 is that parts were placed, not designed. So step 3 is a design pass over the whole screen, not a restyling to resemble the mocks.

### The mocks are the visual reference

The screen was designed from local mocks that are not in the repository. They are at the main checkout's `.agents/tmp/mocks/`, namely `v2.html` with `app2.js`, `v3.html` with `map3.js`, and `style.css`. Read them as files, or serve that directory with `python3 -m http.server` and open `v2.html` and `v3.html`. Do not copy their sample data into the repository: it holds the text of a real brainstorm. Copy styling and structure only.

The person compared the real screen with mock 2 at the same width. The differences to close:

| | Mock | Current screen |
|---|---|---|
| Options | each option a bordered row; the chosen row tinted; a solid "recommended" tag | bare browser radio buttons; a small outlined tag |
| Consequence (この答えだと) | a tinted band with a "→" heading | a thin left rule, no heading |
| Controls | one-line note; small chip buttons for quick asks; opening is a text link | defer checkbox, a multi-line note, three full-size ask buttons, an input, "Open" and "Make provisional" buttons, all equally heavy on every card |
| Card head | id and question large; badges right-aligned | small badges stacked above the question |
| Send | sticky bar at the bottom of the viewport with a status line | at the very end of the page |
| Layout | uses the width | one narrow centred column |

### Reuse ledger

| Layer | Decision | Why |
|---|---|---|
| Config file format | adopt `toml` (serde) via `cargo add` | the specification names TOML; kakoi already uses the crate |
| Config directory | adopt `dirs` (`config_dir()`) | already a dependency; honours `XDG_CONFIG_HOME` |
| Screen strings in two languages | build: one JavaScript table of `en` and `ja` strings | a few dozen fixed strings; no library needed for that |
| Theme | build: CSS custom properties switched by an attribute on the root element, plus `prefers-color-scheme` for "follow the OS" | standard CSS |
| Measuring label width on the map | build: the SVG text's own measured length (`getComputedTextLength`) | the browser already knows the width; character counts cannot |

### Provisional answers

Each answer has an overturn condition: what the person would say to reverse it.

1. **Branch.** The work runs on branch `screen-polish`, created from `first-tool`. Its pull request targets `first-tool`, so PR #1 grows to include it. Overturn if: the person merges PR #1 first and wants this to target `main`.
2. **Stamp in the data.** Each answer records its stamp as `person` or `pre_approved`. In the `submitted` event, this field replaces `sent_unseen`. The old "opened/touched" fields are removed. Overturn if: never; the specification removed them.
3. **Config keys.** `language = "en" | "ja"`, `theme = "system" | "light" | "dark"`. An absent key means the default (English, system). Overturn if: the person wants other names.
4. **Config through the page API.** The page reads and writes the config through the agent-independent page API under the URL secret, the same way it sends operations. A broken config file is reported to the page as a flag, and writes are then refused. Overturn if: never.
5. **Japanese strings use the glossary's words.** The table's `ja` column uses CONTEXT.md's terms (聞き返し, 仮決め, 先送りのラベル, 見直したい, 判子, 代決, 差し戻し, 完成図, 地図). Any string with no glossary term is the implementer's wording and is listed in the report. Overturn if: the person rewrites strings at review.
6. **Language and theme apply to fixed strings only.** Text the agent wrote is never translated. Terminal output stays English ([設定ファイル](../spec/server.md#設定ファイル)).
7. **Map labels wrap by measured width.** Up to two lines fit the node box; the rest is cut with "…". The full text is in the hover box. Overturn if: the person prefers bigger boxes.
8. **The hover box lives in a top-level layer** (fixed position, outside the scrolling map container). It flips to the other side of the node at the viewport edge. An empty detail line is omitted, never printed as "null". Overturn if: never.
9. **The ask field is a multi-line text area.** Enter sends; Shift+Enter inserts a newline; Enter while an input method is composing does nothing. Overturn if: the person wants Ctrl+Enter to send instead.
10. **The result view** is what the screen shows for a round with no questions. Its send button is labelled "Proceed with this result" / "この結果で進める", and it sends an ordinary `submitted` event with no answers. Overturn if: the person wants a dedicated event kind.
11. **The visual overhaul keeps every `data-*` hook the tests use.** Existing browser tests stay valid, and tests are changed only where the specification changed (stamps replace the unopened mark).

12. **Send asks for confirmation (c3).** "Send all" opens a dialog listing, per question, the chosen answer, its stamp (person or pre-approved), the defer switch and the note, with "Send" and "Back". Only "Send" submits. Overturn if: the person finds the extra step tiresome.
13. **The map can be zoomed and moved, and selections can be undone (c5).** Wheel and pinch zoom inside the map area. Drag, or touch-drag, moves it. A "fit" control shows everything. A back control returns to the range, selection and zoom before the last change. Nodes and edge labels are laid out so that they do not overlap. Edge labels may be hidden at low zoom and shown on hover or selection. Overturn if: the person prefers a different navigation model.
14. **The design pass is judged by the person, not by tests.** The implementer reads the skill `frontend-design:frontend-design` if available, and works from the mocks and the comments c1–c5. The bar is that each screen reads as designed: a clear hierarchy, grouped controls, and consistent components (options, chips, stamps, badges, dialogs). Overturn if: never.
15. **Review requests in the result.** While any 見直したい is pending in a result round, its button reads "Send review requests" / 「見直しを頼む」 instead of "Proceed with this result". The confirmation dialog (answer 12) opens for result rounds too and lists the pending requests. The agent treats an empty `submitted` as "proceed" only when no review request is pending. Otherwise it issues the next round re-asking those decisions ([収束したとき](../spec/skill.md#収束したとき)). Overturn if: the person wants a separate button for each.
16. **Map nodes open details on click or tap.** Hovering shows details with a mouse. Clicking or tapping a node opens the same details box, which holds a "Show path to this" button and the 見直したい controls. The range changes only through that button, so a tap never jumps the map by surprise. Overturn if: the person wants a click to change the range directly, as now.
17. **One back control and one history.** A single back control in the screen undoes the last navigation, whether it was a map change (range, selection, zoom) or a jump to another tab. It is the same history 移動と現在地 already requires. Overturn if: the person wants the map to keep its own separate back.
18. **The header holds the title, the original request, the finished-picture opener and the switches.** The title is on the left with the original request expandable under it. The finished-picture opener and the theme and language switches are on the right. All of it stays pinned. Overturn if: the person wants the finished picture elsewhere.

## Scope of change

`crates/shoryo-core/`, `crates/shoryo-server/`, `src/`, `web/`, `web/tests/`, `tests/`, `skills/shoryo/`, `Cargo.toml`, `Cargo.lock`, `README.md`, `PROJECT.md` (Commands and Stack and layout, only where something changed). Nothing in `docs/spec/` or `CONTEXT.md`.

## Step order and prerequisites

The steps run from the data outward.

1. Stamps and the result in core. Everything on the screen reads them, and core can be tested without a browser.
2. The config file in the server. The page needs it before the header exists.
3. The visual overhaul. It changes the base styles and layout, so the new UI in steps 4–6 is built once, in the new style.
4. The header, language, and theme.
5. Stamps and the result on the screen.
6. The map and ask-field fixes.
7. The skill and its references. They document the final formats.
8. The person's review of the finished screen through kemi.

Each step needs the ones before it, except steps 4, 5 and 6: each needs step 3, but not each other.

## Steps

## Step 1 — Stamps and the result are rules of the topic state

Purpose: the state and operations follow [判子](../spec/screen.md#判子). Specification: [判子](../spec/screen.md#判子), [まとめて送る](../spec/screen.md#まとめて送る), [仮決めの一覧](../spec/screen.md#仮決めの一覧), [結果](../spec/screen.md#結果), [決まったこと](../spec/screen.md#決まったこと), [状態データ](../spec/server.md#状態データ), [送られた答えの扱い](../spec/skill.md#送られた答えの扱い).
Prerequisites: none.
May change: `crates/shoryo-core/`, `crates/shoryo-server/` (the operations and the page view), `tests/`, and, only to keep the existing browser suite green, `web/view-data.js`, `web/app.js` and `web/tests/` fixtures and tests that relied on the removed unopened mark (make the fixture stamp before submitting; remove or rewrite unopened-mark assertions as stamp assertions). The real stamp UI is step 5.
Done when: these rules hold, and the old opened/touched/sent-unseen fields are gone.
- Human-decided questions start unstamped; provisional ones start pre-approved.
- Stamping and unstamping are operations.
- Changing the choice or the defer switch removes the stamp; changing the note does not.
- Unstamping a pre-approved row is a send-back.
- A swap puts the question in the initial stamp state of its new class.
- Submit is refused while any question lacks a stamp.
- The `submitted` event carries each answer's stamp (`person` / `pre_approved`).
- Decisions decided from an answer sent pre-approved carry the 代決 mark; a decision later revised through a review request loses it.
- A round with no questions is accepted and can be submitted with no answers.
- `scripts/test-web.sh` still passes.

Shown by: test — core tests:
- `human_question_starts_unstamped_and_provisional_starts_pre_approved`
- `changing_choice_removes_stamp_but_changing_note_does_not`
- `unstamping_pre_approved_row_is_a_send_back`
- `swap_resets_stamp_to_new_class_initial_state`
- `submit_is_refused_while_a_question_is_unstamped`
- `deferred_question_needs_a_stamp`
- `submitted_event_carries_stamp_kind`
- `decision_from_pre_approved_answer_is_marked_and_loses_mark_when_revised`
- `round_without_questions_is_submitted_with_no_answers`

Left to the implementer: field names inside the state (document them in step 7).
Stop and hand back if: none.

## Step 2 — Language and theme live in the per-user config file

Purpose: [設定ファイル](../spec/server.md#設定ファイル). Specification: [設定ファイル](../spec/server.md#設定ファイル), [切替は設定に残る](../spec/screen.md#切替は設定に残る).
Prerequisites: step 1.
May change: `crates/shoryo-server/`, `src/`, `tests/`, `Cargo.toml`, `Cargo.lock`.
Done when:
- The server reads the config file each time the page is opened.
- A page write changes the file atomically.
- A broken file starts the server with a stderr warning and the defaults. The page gets a "config unreadable" flag, writes are refused, and the file is left byte-for-byte unchanged.
- The config is never written inside a topic's data.
- Both test harnesses (`tests/round_trip/` and the browser fixtures) set `XDG_CONFIG_HOME` to a temporary directory, so no test reads or writes the developer's real config.

Shown by: test — round-trip tests:
- `language_switch_is_written_to_the_config_file_and_survives_restart`
- `switch_made_in_one_topic_applies_when_another_topic_page_opens`
- `broken_config_starts_with_defaults_and_is_left_unchanged`
- `config_is_not_written_into_topic_data`

Left to the implementer: none.
Stop and hand back if: none.

## Step 3 — A design pass over the whole screen

Purpose: close the differences in the table under "The mocks are the visual reference", and address c1, c2 and c4 (provisional answer 14). Specification: [たたんだカード](../spec/screen.md#たたんだカード), [この答えだと](../spec/screen.md#この答えだと), [仮決めの一覧](../spec/screen.md#仮決めの一覧), [聞き返し](../spec/screen.md#聞き返し), [過去のラウンド](../spec/screen.md#過去のラウンド), [未決](../spec/screen.md#未決).
Prerequisites: step 1.
May change: `web/` (except `web/tests/`, which must keep passing unchanged here).
Done when:
- Options are bordered rows with the chosen row tinted and a solid recommended tag.
- The consequence is a tinted band with its heading.
- Per-card controls are compact: a one-line note, chip-sized quick asks, opening as a text link, and the class swap as a small secondary control.
- The card head shows the question large, with badges right-aligned.
- The send bar is sticky at the bottom.
- The layout uses the width, with a readable maximum.
- Colours are CSS custom properties, so step 4 can add the dark theme.
- The question card is restructured (c1). The question, its options with the consequence, and the stamp area are the main body. Why-now, premises, asks, note and defer are grouped as secondary parts, folded or compact.
- The provisional list picks options the way cards do (c2), not with a bare select box.
- Past rounds keep the card form, with clear separation between questions (c4).
- Every control on every tab uses the same small set of designed components.

Shown by: check — `scripts/test-web.sh` passes. Existing tests may change only where they located elements whose structure the redesign replaced, and only by a new `data-*` hook, never by a weaker assertion. The look itself is the person's check in step 8.
Left to the implementer: exact spacing, colours and type sizes within the mocks' direction.
Stop and hand back if: a mock layout contradicts a specification heading.

## Step 4 — The header switches language and theme

Purpose: [ヘッダーと切替](../spec/screen.md#ヘッダーと切替). Specification: [ヘッダーと切替](../spec/screen.md#ヘッダーと切替), [言語](../spec/screen.md#言語), [テーマ](../spec/screen.md#テーマ), [切替は設定に残る](../spec/screen.md#切替は設定に残る), [画面の構成](../spec/screen.md#画面の構成).
Prerequisites: steps 2 and 3.
May change: `web/`, `web/tests/`.
Done when:
- A thin header stays at the top while scrolling, with the title on the left and, on the right, a theme icon that cycles light → dark → system and an en | ja switch.
- Every fixed string comes from the two-language table.
- Japanese uses the glossary's terms.
- Text the agent wrote is unchanged by the switch.
- A config-unreadable flag shows a notice in the header, and the switches still change the open page for as long as it stays open.
- With no setting, the theme follows `prefers-color-scheme`.
- The header follows provisional answer 18.

Shown by: test — browser tests:
- `header_stays_fixed_while_scrolling`
- `switching_to_ja_translates_fixed_text_and_leaves_agent_text`
- `default_language_is_english`
- `theme_icon_cycles_light_dark_system`
- `unreadable_config_is_shown_in_the_header`
- `switches_still_work_for_the_page_when_config_is_unreadable`
- `theme_follows_os_when_unset` (emulating both colour schemes)
- `finished_picture_and_original_request_are_reachable_from_the_header_on_every_tab`

Left to the implementer: the Japanese wording of strings that have no glossary term (list them in the report).
Stop and hand back if: none.

## Step 5 — Stamps and the result on the screen

Purpose: [判子](../spec/screen.md#判子) and [結果](../spec/screen.md#結果) as the person uses them. Specification: [判子](../spec/screen.md#判子), [仮決めの一覧](../spec/screen.md#仮決めの一覧), [まとめて送る](../spec/screen.md#まとめて送る), [過去のラウンド](../spec/screen.md#過去のラウンド), [決まったこと](../spec/screen.md#決まったこと), [結果](../spec/screen.md#結果).
Prerequisites: steps 1 and 3.
May change: `web/`, `web/tests/`.
Done when:
- "Send all" opens the confirmation dialog of provisional answer 12, and only its "Send" submits (c3).
- The result round follows provisional answer 15.
- Each card and each provisional row has its stamp.
- Pre-approved rows show the 代決 state and can be sent back by unstamping or by changing the choice.
- The send bar shows how many questions lack a stamp, and clicking it moves to the first one.
- Send is disabled until all are stamped.
- Past rounds and the decisions tab mark answers sent pre-approved as 代決.
- A round with no questions shows the result: finished picture, decisions with the 代決 mark, not-building, rejected, undecided and delegated, and a "Proceed with this result" send button with 見直したい available.
- Tests that relied on the removed unopened mark are rewritten for stamps.

Shown by: test — browser tests:
- `send_is_disabled_until_every_question_is_stamped`
- `unstamped_count_jumps_to_first_unstamped_question`
- `provisional_row_arrives_pre_approved_and_can_be_sent_back`
- `pre_approved_answers_are_marked_in_past_rounds_and_decisions`
- `round_without_questions_shows_the_result`
- `send_all_opens_a_confirmation_listing_each_answer_and_its_stamp`
- `result_with_pending_review_request_sends_review_requests_instead_of_proceed`

Left to the implementer: the stamp's visual form.
Stop and hand back if: none.

## Step 6 — A usable map, and the ask field

Purpose: defects 1, 2 and 4, and c5 (provisional answers 7, 8, 9, 13). Specification: [地図](../spec/screen.md#地図) and its subsections, [聞き返し](../spec/screen.md#聞き返し).
Prerequisites: step 3.
May change: `web/`, `web/tests/`.
Done when:
- Map labels never extend past their box for long Japanese text (provisional answer 7).
- The hover box is never clipped by the map container and shows no "null" line (provisional answer 8).
- The ask field takes newlines (provisional answer 9).
- The map zooms (wheel and pinch) and moves (drag and touch-drag) inside its area, and has a fit control.
- In the 道筋 range, nodes and edge labels do not overlap at the default zoom, and edge labels are shown there. In the 全部 range, labels may be hidden at low zoom (its overlap method stays the person's call, [未決](../spec/screen.md#未決)).
- Clicking or tapping a node opens its details as provisional answer 16 says.
- The back control follows provisional answer 17.

Shown by: test — browser tests:
- `long_japanese_label_stays_inside_its_node_box`
- `hover_box_is_fully_visible_and_has_no_null_line`
- `shift_enter_inserts_a_newline_and_enter_sends`
- `map_zooms_and_moves_inside_its_area`
- `map_back_restores_previous_range_selection_and_zoom`
- `path_range_nodes_and_edge_labels_do_not_overlap_at_default_zoom`
- `tapping_a_node_opens_details_without_changing_the_range`

Left to the implementer: none.
Stop and hand back if: none.

## Step 7 — The skill documents stamps, the result, and careful classification

Purpose: the agent's side of the revision. Specification: [問いの仕分け](../spec/skill.md#問いの仕分け), [収束したとき](../spec/skill.md#収束したとき), [送られた答えの扱い](../spec/skill.md#送られた答えの扱い).
Prerequisites: steps 1–6.
May change: `skills/shoryo/`, `tests/` (the examples test), `README.md`, `PROJECT.md`.
Done when:
- SKILL.md says not to put behaviour- or design-shaping questions in 仮決め, and to make them human-decided when unsure.
- It says to send a question-less round when the topic converges. An empty `submitted` with no pending review request means "proceed". With pending requests, the next round re-asks them (provisional answer 15).
- It says that pre-approved answers are answers.
- The references document the stamp field in `submitted`, the result round, and any new state fields.
- Every JSON example still passes `skill_examples_are_accepted`.

Shown by: artifact — `skills/shoryo/SKILL.md` and `skills/shoryo/references/*.md`, with `skill_examples_are_accepted` passing.
Left to the implementer: none.
Stop and hand back if: none.

## Step 8 — The person reviews the finished screen

Purpose: the visual check left to the person ([未決](../spec/screen.md#未決)).
Prerequisites: step 7.
May change: nothing unless the person asks.
Done when: the person has looked at the screen in both themes and both languages through `kemi --live <page URL>` (comments pinned to places) and has said whether it reads well enough.
Shown by: external — the person runs it. Findings go to `ba0918-iterate` on this branch.
Left to the implementer: none.
Stop and hand back if: none.

## Verification map

| Specification section | Steps |
|---|---|
| [判子](../spec/screen.md#判子), [まとめて送る](../spec/screen.md#まとめて送る), [仮決めの一覧](../spec/screen.md#仮決めの一覧) | 1, 5 |
| [結果](../spec/screen.md#結果), [収束したとき](../spec/skill.md#収束したとき) | 1, 5, 7 |
| [ヘッダーと切替](../spec/screen.md#ヘッダーと切替) and its subsections, [設定ファイル](../spec/server.md#設定ファイル) | 2, 4 |
| [過去のラウンド](../spec/screen.md#過去のラウンド), [決まったこと](../spec/screen.md#決まったこと) | 1, 5 |
| [地図](../spec/screen.md#地図) and its subsections, [聞き返し](../spec/screen.md#聞き返し) | 6 |
| [まとめて送る](../spec/screen.md#まとめて送る) (confirmation) | 5 |
| [問いの仕分け](../spec/skill.md#問いの仕分け), [送られた答えの扱い](../spec/skill.md#送られた答えの扱い) | 7 |
| [未決](../spec/screen.md#未決) (visual details) | 3, 8 |

## Left to the implementer

Plan-wide: names; internal structure; field names (documented in step 7); styling within the mocks' direction.

## Stop conditions

Beyond the general ones (skill `ba0918-cycle`):

- A step needs a change to `docs/spec/` or `CONTEXT.md`.
- A mock's layout contradicts a specification heading.

## Test command

- `scripts/check.sh` and `scripts/test-web.sh`.
- On this machine a version-manager shim may shadow cargo. If `cargo --version` is not 1.99.0, prefix the commands with `PATH=$HOME/.cargo/bin:$PATH`.

## Out of scope

- Counting stamps or pre-approval rates ([作らないもの](../spec/screen.md#作らないもの)).
- Watching the config file for changes.
- Rewriting the ba0918 or kotowari brainstorm.
- Release and version bump.
