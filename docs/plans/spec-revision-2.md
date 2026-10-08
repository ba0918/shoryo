# Plan: the second specification revision (質問, 見直す, dated stamps, arrivals, diagram gestures)

## Goal

After this plan, shoryo matches the specification as revised in commit d6457de:

- the screen uses the words 質問 and 見直す;
- a review request asks for confirmation first, cannot be sent between sending a round and the next round, and can be withdrawn until the LLM concludes it;
- stamps carry the date they were pressed or sent;
- past result rounds show what was sent then;
- arrivals from the LLM are announced by toasts, a notification icon and the tab title;
- the wheel scrolls the page over diagrams unless Ctrl/⌘ is held.

## Specification

Read the specification in Japanese. The links below point at its headings.

- [画面](../spec/screen.md)
- [操作と知らせ](../spec/interaction.md) (new in d6457de)
- [起動と往復と記録](../spec/server.md)
- [shoryo skill](../spec/skill.md)
- [CONTEXT.md](../../CONTEXT.md) (the glossary: 質問, 見直す)

`git show d6457de` shows exactly what changed. The steps below link only the headings they rest on.

## Approach and why

### What is already done

The following were checked against the code and need no product change.

- **LLM status** ([LLM の状態](../spec/server.md#llm-の状態), [ヘッダーの LLM の状態](../spec/screen.md#llm-の状態)).
  - `crates/shoryo-server/src/app.rs` counts `round`, `wait` (start and return), `reply` and `end`. It does not count `result` or `stop`.
  - The server-start time seeds the last counted command (`AgentActivity::heard` starts at `Instant::now()` in `Shared::new`).
  - The page decides the 10-minute boundary with its own clock (`agentStatus` in `web/view-data.js`).
  - The status is hidden after the topic ends, and stays "working" after the person proceeds with the result.
  - Existing tests cover waiting/working and the 10-minute boundary (`status_reports_whether_a_wait_is_in_progress`, `web/tests/after-send.spec.js`). No test covers which commands are counted. Step 1 adds one; it is expected to pass at once.
  - Step 4 switches the "stays working after proceeding" rule to read the stored "how it was sent".
- **決まったこと** ([決まったこと](../spec/screen.md#決まったこと), its new success condition: seven sections; 代決 marked; the mark removed once revised by a review). Covered by:
  - `decisions_tab_lists_the_seven_kinds` (`web/tests/look-back.spec.js`);
  - the decisions-tab half of `pre_approved_answers_are_marked_in_past_rounds_and_decisions`;
  - `decision_from_pre_approved_answer_is_marked_and_loses_mark_when_revised` (`crates/shoryo-core/tests/domain/views.rs`).
- **Typing dots.** A reply being written shows three bouncing dots and visually hidden text (`web/components/thread.js`).
- **Reduced motion.** A global reduced-motion rule stops every animation (`web/style.css`).
- **Withdrawing a review.** "見直しをやめる" already acts without a confirmation.
- **Reply diagrams.** They are drawn without `PanZoom`, so they never zoom. `diagramView` is used only by the result and the finished-picture dialog. Step 8 adds a test that keeps it so.

### Order of the work

1. **Data and server rules first (Step 1).** Every later screen step reads the new stored fields: stamp times, sent times, per-round picture and records, and how a result was sent. The new refusal of review requests is a server rule.
2. **Words next (Step 2).** Later tests then use the final strings, and the glossary check runs once over the whole screen.
3. **Screen features that read the new data (Steps 3–5):** dated stamps, the past result round, and the 見直す confirmation and window.
4. **Navigation and header (Steps 6–7).** Step 6 sets the header order, the back stack and the page title. Toasts in Step 9 push onto the back stack and live in the header, so Step 6 comes before Step 9. Step 7 adds focus on map points.
5. **Interaction (Steps 8–10):** diagram gestures, arrivals, and the operation promises. They are last on the screen because they observe everything before them.
6. **Skill, README and changelog (Step 11).** They describe the end state.
7. **The person's check (Step 12).**

### Provisional answers

The specification leaves these open. Each answer keeps the approved behaviour; each names what would overturn it.

1. **How times are stored.**
   - Times are RFC 3339 strings in UTC with a `Z` suffix and millisecond precision, for example `2026-10-08T04:01:13.123Z`.
   - The server takes the time from its own clock when it accepts the operation. The domain crate never reads a clock: `Topic::apply` (and whatever records a send) takes the time as an argument. `shoryo-core` gains no time crate.
   - Why: [状態データ](../spec/server.md#状態データ) says "UTC で記録する". The data is read by the workflow LLM (`shoryo result`), where an explicit UTC string reads unambiguously. The browser's `Date` parses this form.
   - Overturn if: the person wants epoch numbers in the data.
2. **What the stored data looks like.**
   - A person's stamp keeps its time in a field beside the stamp. The stamp's own value (`"person"`/`"pre_approved"`) and the `submitted` event's `stamp` field stay unchanged, so the agent's event contract in `skills/shoryo/references/events-and-replies.md` does not change.
   - Each round keeps `submitted` and stores beside it:
     - its sent time;
     - the finished picture and the four non-decision records (`not_building`, `undecided`, `delegated`, `rejected`) as they stand after that round was applied — a list the round omitted keeps its earlier value;
     - for a result round, how it was sent: `proceeded`, or `review_requested` when any decision was in review at the moment it was sent.
   - Decisions as of a round are already reconstructable from their history (`content_as_of`).
   - Why: [状態データ](../spec/server.md#状態データ) requires a past result to be redrawn "送られたときの中身で". Omitted lists mean "unchanged" ([round.rs](../../crates/shoryo-core/src/round.rs) `RecordsInput`), so the effective state is what the round showed. `submitted` stays because the screen reads `round.submitted` in several places and Step 1 does not touch `web/`.
   - Overturn if: the person wants only the lists literally carried by that round.
3. **When a review request is refused.**
   - The server refuses `request_review` whenever the current round has been sent. That covers both "until the next round arrives" and "after proceeding, until the topic ends": after a send, only a new round (`round`) or `end` changes the state again.
   - The refusal has its own kind, distinct from `round_submitted`, so the screen can say why. The name is left to the implementer.
   - The refusal adds no event and no record.
   - `stop_review` is allowed while the decision is in review. A review conclusion in a round already removes the mark, so "until the LLM gives its conclusion" needs no extra state. The `NextRoundArrived` refusal and its strings go.
   - Overturn if: never — this is [待つ](../spec/server.md#待つ) and [見直す](../spec/screen.md#見直す) read directly.
4. **How the screen tells which arrival happened** ([操作と知らせ / 任せたこと](../spec/interaction.md#任せたこと) leaves this to the plan).
   - Every view the page accepts (`accept()` in `web/app.js`, whether it came from the live stream or from the view an operation fetched) is compared with the previously accepted view:
     - a new last round is "next round", or "result" when it has no questions;
     - each ask that is `replied` in the new view and was not `replied`, or did not exist, in the previous accepted view is one "reply". Two replies that land between two accepted views are two arrivals.
   - Only the first view after the page loads (there is no previous view) announces nothing. A view after the stream reconnects is compared like any other, so what arrived while it was down is still announced.
   - Why: the page cannot see individual changes. The server's live stream sends the latest view and may skip intermediate versions (it watches a `tokio::sync::watch` counter), and `accept()` drops views older than the one it holds. Comparing two accepted states finds every arrival regardless, and needs no server change.
   - Overturn if: an arrival is found that comparing accepted views cannot see (then the server sends the kind).
5. **The finished-picture button.**
   - [ヘッダーと切替](../spec/screen.md#ヘッダーと切替) lists the header's right side "この順に" and does not include it. [画面の構成](../spec/screen.md#画面の構成) still requires the finished picture to open from every tab.
   - The button therefore moves out of the header to the right end of the tab row, which every tab shows.
   - Overturn if: the person wants it back in the header (then say where in the order).
6. **What pushes onto the back stack** ([移動と現在地](../spec/screen.md#移動と現在地)).
   - These moves push the place before the move:
     - a jump from a point or a link (the existing `jump` event);
     - a toast's 見る;
     - the button that goes to the first question without a stamp;
     - a choice in the notification list. This goes beyond the specification's list of four. [届いたものの知らせ](../spec/interaction.md#届いたものの知らせ) says a choice there moves "to the same place as the toast's 見る", and the plan reads that as the same move, back entry included.
   - Choosing a map range, selecting a point, the "show the path" button and zooming no longer push. Manual tab switches never push.
   - A place records: the tab, the chosen past round, the map's range, selection, root and view, the current round's number, and an anchor. The anchor is the card, provisional row, decision item or result nearest the top of the viewport, with its offset.
   - "Back" restores the place and scrolls the anchor back to its offset.
   - A place is gone when its anchor no longer exists in what that place would show. For example, a card of the current round once the next round has arrived.
   - Whether a place is gone is a pure function of the view data. The top-level component prunes gone places on every render, so "戻る" shows only while a reachable place exists, and pressing it always moves.
   - Overturn if: the person wants map range changes to be undoable with "戻る" again, or wants a notification-list choice not to push.
7. **The page title.**
   - The page title becomes the topic's title (the untitled text when empty).
   - While the browser tab is hidden it is prefixed with the count of arrivals, for example "(2) 議題名" ([届いたものの知らせ](../spec/interaction.md#届いたものの知らせ)).
   - Overturn if: the person wants "shoryo" kept in the title.
8. **When an arrival counts as already seen** (no toast, not counted in the badge).
   - The current tab shows it, and its element intersects the viewport after the redraw:
     - for a reply, the reply element;
     - for a next round, the first element of the current round (the fixes, the first card, or the result);
     - for a result, the result section.
   - Overturn if: the person finds toasts missing or redundant in use.
9. **Where an arrival's 見る lands.**
   - Next round: the top of the current round.
   - Result: the result section.
   - Reply: the reply element itself. Before scrolling, the page selects its tab and round, opens the card (or the provisional row as a card) and its thread, and expands an elided middle.
   - Overturn if: never — this is the table in [届いたものの知らせ](../spec/interaction.md#届いたものの知らせ) made concrete.
10. **The wheel hint and Mac detection.**
    - A plain wheel over a zoomable diagram shows the hint for 1.5 seconds after the last wheel event.
    - The hint shows ⌘ when `navigator.userAgentData?.platform ?? navigator.platform` matches /Mac/, and Ctrl otherwise.
    - Ctrl+wheel and ⌘(Meta)+wheel both zoom on every platform. Meta+wheel has no browser meaning off the Mac, so this changes nothing approved.
    - Overturn if: the person wants the hint longer or shorter.
11. **Stamp dates.**
    - The date on a stamp is `M/D` in the browser's time zone, as [判子](../spec/screen.md#判子) shows ("10/8").
    - The year and time appear in a tooltip. It shows on hover and on focus for every dated stamp, and also on tap for any stamp that can no longer be pressed: the person's and 代決 alike, in past rounds and in the current round once sent. Its format is left to the implementer, as long as it shows the four-digit year, the hour and the minute.
    - Past rounds show each question's stamp, read only. It replaces the past round's "代決" badge on questions; decision items keep their own 代決 mark.
    - Overturn if: the person rewrites the look (stamp looks are 未決, [未決](../spec/screen.md#未決)).
12. **The 代決 mark in a past result round.** A decision there carries 代決 when it came from a pre-approved answer and had not been revised as of that round. Overturn if: the person wants today's mark.
13. **A refused review request on the screen.** While it cannot be sent, "見直す" is drawn disabled, not hidden. If a stale page still sends one, the refusal text says that 見直す cannot be used until the next round arrives. Overturn if: the person prefers hiding it.
14. **How the skill handles a late withdrawal** ([見直しの問い直し](../spec/skill.md#見直しの問い直し) does not cover it).
    - `review_stopped` can now arrive after the agent has already asked the decision again, until it sends a conclusion.
    - In that case the agent keeps the decision as it was and sends the conclusion `unchanged`. It ignores the answer to the re-asked question, because stopping a review returns the decision to its original state ([見直す](../spec/screen.md#見直す): 「見直しをやめる」を押すと「元に戻る」).
    - Overturn if: the person wants the re-asked answer applied anyway.
15. **How the round-trip tests read stored times.** They parse the RFC 3339 string with a small standard-library helper in `tests/round_trip/harness.rs` (date and time fields to milliseconds since the epoch). No dev-dependency is added. Overturn if: Step 1 adopts the `time` crate in `shoryo-server` and the parser grows beyond a few lines; then the root package may take the same crate as a dev-dependency.

### Wording this plan decides

[画面の任せたこと](../spec/screen.md#任せたこと) delegates the English for newly decided Japanese text to this plan, with the same meaning. Japanese marked "declared" is the specification's own text. Tests may pin it. Everything else here is the plan's choice. Tests must not pin it.

| Key (suggested) | Japanese | English |
|---|---|---|
| thread heading | LLM への質問（{count}） (declared) | Asks to the LLM ({count}) |
| follow-up button | この返事に続けて質問 (declared) | Follow up on this reply |
| ask placeholder | この問いについて質問する | Ask about this question |
| follow-up placeholder | 続きの質問 | Your follow-up |
| review button / stop | 見直す / 見直しをやめる (declared) | Review this / Stop review |
| in review mark | 見直し中 (declared) | In review |
| review confirmation | LLM に伝えて、次のラウンドで問い直してもらいます。LLM が見直しの結論を出すまでは、見直しをやめられます。 | The LLM is told now and asks this again in the next round. You can stop the review until the LLM gives its conclusion. |
| result review confirmation | 見直しを頼むで送ると、次のラウンドで問い直されます (declared)。LLM が見直しの結論を出すまでは、見直しをやめられます。 | Send with “Send review requests” and this is asked again in the next round. You can stop the review until the LLM gives its conclusion. |
| confirmation buttons | 見直す / 見直さない (declared) | Review / Don't review |
| refused review request | 送ってから次のラウンドが届くまでは、見直すを使えません。 | Review requests cannot be sent until the next round arrives. |
| past result: how sent | この結果で進めました / 見直しを頼みました | Proceeded with this result / Asked for a review |
| toast: next round | 第 {n} ラウンドが届きました (declared) | Round {n} has arrived |
| toast: result | 結果が届きました (declared) | The result has arrived |
| toast: reply | {question}への返事が届きました (declared; `{question}` is the first 20 characters, cut with …) | A reply to “{question}” has arrived |
| toast action | 見る (declared) | View |
| notification icon label / empty list | 届いたもの / まだ届いたものはありません | Arrivals / Nothing has arrived yet |
| wheel hint | Ctrl＋ホイールで拡大 / ⌘＋ホイールで拡大 (declared) | Ctrl + wheel to zoom / ⌘ + wheel to zoom |
| confirm-send review list | 見直し中のもの | Review requests |

Japanese text elsewhere that still says 聞き返し or 見直したい changes to the glossary's words. This covers `refusal.unknown_ask`, `refusal.already_replied`, `refusal.follows_another_question` and `result.hint`. The English names `ask`, `reply <ask id>` and the English screen's "Ask" stay ([CONTEXT.md](../../CONTEXT.md)).

## Scope of change

- `crates/shoryo-core/`
- `crates/shoryo-server/`
- `scripts/check-domain-purity.sh`
- `tests/round_trip/`
- `web/` and `web/tests/`
- `skills/shoryo/`
- `README.md`
- `CHANGELOG.md`

## Step order and prerequisites

| Step | What it produces | Needs |
|---|---|---|
| 1 | Stored times, per-round picture, records and how sent; review request window; counted-command test | — |
| 2 | Words on the screen and in code | 1 (only for deleting `refusal.next_round_arrived`) |
| 3 | Dated stamps, now and in past rounds | 1, 2 |
| 4 | Past result rounds as sent; stored "how sent" drives the notice and the status | 1, 2 |
| 5 | 見直す confirmation, the closed window, withdrawal until the conclusion | 1, 2, 4 |
| 6 | Header order, finished picture in the tab row, back stack, page title | 2 |
| 7 | Map points: details on focus, panned into view | 6 |
| 8 | Wheel, Ctrl/⌘ and touch over diagrams | 2 |
| 9 | Toasts, notification icon, tab title count | 6 |
| 10 | Operation promises: keyboard, focus, reduced motion | 5, 7, 9 |
| 11 | Skill, README, changelog | 1–10 |
| 12 | The person's check | 1–11 |

Steps 3 and 4 are independent of each other, and so are 6 and 8. Step 5 needs Step 4 because its tests press 見直す in a past result round. Run the steps in the listed order anyway: they touch the same files, and the order keeps every diff small.

## Verification map

| Specification section | Steps |
|---|---|
| [状態データ](../spec/server.md#状態データ) | 1, 3, 4 |
| [待つ](../spec/server.md#待つ) (refused review requests) | 1, 5 |
| [LLM の状態](../spec/server.md#llm-の状態) | already done; 1 (counted commands); 4 (stored "how sent"); 11 (README) |
| [画面の構成](../spec/screen.md#画面の構成) | 6 |
| [ヘッダーと切替](../spec/screen.md#ヘッダーと切替), [切替は設定に残る](../spec/screen.md#切替は設定に残る) | 6, 9 |
| [言語](../spec/screen.md#言語), [質問](../spec/screen.md#質問) | 2, 10, 12 |
| [判子](../spec/screen.md#判子) | 3, 12 |
| [まとめて送る](../spec/screen.md#まとめて送る), [見直す](../spec/screen.md#見直す), [結果](../spec/screen.md#結果), [直したこと](../spec/screen.md#直したこと) | 1, 5 |
| [送ったあとの知らせ](../spec/screen.md#送ったあとの知らせ) | 4 |
| [過去のラウンド](../spec/screen.md#過去のラウンド) | 3, 4, 5 |
| [決まったこと](../spec/screen.md#決まったこと) | already done (`decisions_tab_lists_the_seven_kinds`, `pre_approved_answers_are_marked_in_past_rounds_and_decisions`, `decision_from_pre_approved_answer_is_marked_and_loses_mark_when_revised`) |
| [詳しい情報の出し方](../spec/screen.md#詳しい情報の出し方) | 7 |
| [移動と現在地](../spec/screen.md#移動と現在地) | 6, 9 |
| [地図と完成図を動かす](../spec/interaction.md#地図と完成図を動かす) | 8, 12 |
| [届いたものの知らせ](../spec/interaction.md#届いたものの知らせ) | 9, 12 |
| [操作の約束](../spec/interaction.md#操作の約束) | 10, 12 |
| [skill.md](../spec/skill.md) | 11 |

## Left to the implementer

Plan-wide:

- names of functions, components, CSS classes and `data-*` hooks;
- how view data is split between functions in `web/view-data.js`.

Every choice that changes behaviour is a provisional answer above.

## Stop conditions

Beyond each step's own:

- The specification turns out to contradict itself, or contradicts a provisional answer, in a way the code makes visible.
- A test can only pass by pinning wording the specification does not declare.
- A gate fails for a reason outside this plan's scope.

Never use `git stash` in this worktree. Its stash stack is shared with other worktrees. Set work aside with a temporary commit instead.

## Test command

Prefix every command with `PATH=$HOME/.cargo/bin:$PATH` (otherwise mise's older cargo comes first):

- `PATH=$HOME/.cargo/bin:$PATH scripts/check.sh` — format, lint, domain purity, Rust tests;
- `PATH=$HOME/.cargo/bin:$PATH scripts/test-web.sh` — builds the binary and runs the browser tests; it takes Playwright arguments, for example a single spec file.

## Out of scope

- Changing the workflows that call shoryo ([server.md 作らないもの](../spec/server.md#作らないもの)).
- A server clock override for tests ([server.md 作らないもの](../spec/server.md#作らないもの)).
- OS notifications, a keyboard shortcut to the newest toast, and moving diagrams with arrow keys ([interaction.md 作らないもの](../spec/interaction.md#作らないもの)).
- The visual details of stamps, the header and toasts beyond a first form (未決; the person decides in Step 12).
- Migrating stored data written before this plan.

## Steps

## Step 1 — Times, per-round snapshots and the review window in the data and the server

Purpose: store what [状態データ](../spec/server.md#状態データ) now lists, refuse review requests in the window [待つ](../spec/server.md#待つ) closes, and pin which commands [LLM の状態](../spec/server.md#llm-の状態) counts. Specification: [状態データ](../spec/server.md#状態データ), [待つ](../spec/server.md#待つ), [LLM の状態](../spec/server.md#llm-の状態), [見直す](../spec/screen.md#見直す), [作らないもの](../spec/server.md#作らないもの) (no server clock override).

Prerequisites: none.

May change: `crates/shoryo-core/src/`, `crates/shoryo-core/tests/domain/`, `crates/shoryo-server/src/` (and its `Cargo.toml` only if a time crate is adopted), `scripts/check-domain-purity.sh`, `tests/round_trip/`, `Cargo.lock`, the existing web tests' fixtures only where the stored shape they read changed. Not `web/` otherwise, and not `crates/shoryo-core/Cargo.toml`.

Done when:
- A person's stamp stores the time the server accepted the press (provisional answers 1 and 2).
  - Lifting the stamp removes the time. Pressing again stores the new time.
  - A pre-approved stamp has no time of its own.
- Sending a round stores its sent time beside `submitted`, which stays.
- Every round stores the finished picture and the four non-decision records as they stand after it was applied.
- A sent result round stores `proceeded` or `review_requested`.
- `request_review` is refused while the current round is sent, with its own refusal kind (not `round_submitted`; provisional answer 3). The refusal leaves no event, no in-review mark and no change to the state file.
- `stop_review` succeeds for any decision in review, also after the next round arrived. Once a round's review conclusion has cleared the mark, it is refused (`NotInReview`).
- The `NextRoundArrived` refusal no longer exists.
- The domain crate reads no clock:
  - `scripts/check-domain-purity.sh` also fails on `std::time`, `Instant`, `now_utc` and `Utc::now` in `crates/shoryo-core/src`, and on a `time`, `chrono` or `jiff` dependency in `crates/shoryo-core/Cargo.toml`;
  - the domain tests pass every time in as an argument.

Shown by: test.

1. RED: write these failing tests.
   - Domain (`crates/shoryo-core/tests/domain/events.rs`, `rounds.rs`), with times passed in:
     - `pressing_a_stamp_records_the_time_and_lifting_it_clears_it`
     - `submitting_records_the_sent_time`
     - `result_round_records_whether_it_proceeded_or_asked_for_review`
     - `round_keeps_the_picture_and_records_it_was_applied_with`
     - `review_request_is_refused_while_the_round_is_sent`
     - `review_request_can_be_stopped_after_the_next_round_until_its_conclusion`. It replaces `review_request_cannot_be_stopped_after_next_round`; delete that one.
   - Round trip (`tests/round_trip/main.rs`), reading times with the helper of provisional answer 15:
     - `stamp_time_is_kept_in_utc_and_cleared_when_the_stamp_is_lifted`. Check that the stored instant lies between the test's own clock readings before and after the request, and that any offset it carries is zero. Do not assert the textual form (RFC 3339 is this plan's choice, not a contract). Do not replace the server's clock.
     - `sent_round_keeps_its_sent_time_in_utc`
     - `past_result_round_reads_back_the_picture_and_records_it_was_sent_with`. Send a result round, then a round with a different picture and undecided list, and read `shoryo result`.
     - `review_request_between_sending_and_the_next_round_is_refused_and_leaves_no_trace`. Check the HTTP status, that `wait` returns no `review_requested`, and that the state has no in-review mark. Do not assert the refusal's kind name.
     - `counted_commands_restart_the_quiet_time_and_result_does_not`. Let `agent.quiet_ms` in the view grow past a few hundred milliseconds. After each of `round`, `reply` and `end`, it drops near zero. After `result`, it keeps growing. `stop` ends the server, so it cannot be observed this way and is left to the code. This test is expected to pass at once (the behaviour exists); record that.
2. GREEN: make the others pass.
3. REFACTOR, then run `scripts/check.sh` (it runs the extended purity check).

Left to implementer:
- JSON field names (delegated by [server.md 任せたこと](../spec/server.md#任せたこと)). Step 11 documents whichever names are chosen.
- Whether RFC 3339 is produced in `shoryo-server` with the `time` crate (already in `Cargo.lock` as a transitive dependency; follow ba0918-reuse and ba0918-rust before adopting it) or with standard-library arithmetic.
- Whether old state files still load. Compatibility is not required, because nothing is released. Old files may fail with the existing corrupt-state error.

Stop and hand back if: recording a time needs the server clock to be replaceable in tests ([作らないもの](../spec/server.md#作らないもの) forbids that entry).

## Step 2 — The glossary's words on the screen and in the code

Purpose: the screen says 質問 and 見直す, as [CONTEXT.md](../../CONTEXT.md) and [言語](../spec/screen.md#言語) require. Specification: [言語](../spec/screen.md#言語), [質問](../spec/screen.md#質問), [見直す](../spec/screen.md#見直す).

Prerequisites: Step 1 (its refusal kind `next_round_arrived` is gone, so its strings can go).

May change: `web/strings.js`, comments and the section comment in `web/style.css`, comments in `web/components/` and `web/view-data.js`, the doc comment in `crates/shoryo-core/src/state.rs`, `web/tests/` (the new test, comments, and assertions on renamed strings such as `asks.spec.js`'s `"Ask back (1)"`), `skills/shoryo/references/round.md` line 53.

Done when:
- The ja table has these declared texts:
  - the thread heading 「LLM への質問（{count}）」;
  - the follow-up button 「この返事に続けて質問」;
  - 「見直す」 and 「見直しをやめる」 for the review controls.
- No ja string contains 聞き返し or 見直したい.
- The other rows of the wording table are set, except those added with their features in Steps 4, 5, 8 and 9.
- The en strings for those keys follow the table.
- `refusal.next_round_arrived` is deleted in both languages.
- Code comments use 質問 and 見直す where they meant the renamed operations.

Shown by: test — `ja_screen_says_question_and_review_in_the_glossary_words` (`web/tests/header.spec.js`).
- Switch to ja on a round with an ask and a decision.
- Assert that the thread heading starts with 「LLM への質問」, and that the follow-up button and the review button carry the declared words.
- Assert that the page's text contains neither 聞き返し nor 見直したい. The round, ask and reply the test sends must not contain those words themselves (for example, do not reuse `longLabel` from `web/tests/diagrams.spec.js`).
- The rule is [言語](../spec/screen.md#言語) ("用語集と違う言葉が出ない"); the words are declared in [CONTEXT.md](../../CONTEXT.md) and [質問](../spec/screen.md#質問).

Existing tests that pin the old English or Japanese of these keys change with it.

Left to implementer: none.

Stop and hand back if: none.

## Step 3 — Dated stamps

Purpose: stamps show when they were pressed or sent, now and in past rounds. Specification: [判子](../spec/screen.md#判子), [過去のラウンド](../spec/screen.md#過去のラウンド), [状態データ](../spec/server.md#状態データ).

Prerequisites: Steps 1 and 2.

May change: `web/components/question-parts.js`, `web/components/card.js`, `web/components/provisional.js`, `web/components/past-rounds.js`, `web/view-data.js`, `web/style.css`, `web/strings.js`, `web/tests/`.

Done when:
- A person's stamp shows `M/D` of its stored time in the browser's time zone, in the current round (before and after sending) and in past rounds. Hover and focus show a tooltip with the year and time (provisional answer 11).
- A pre-approved stamp shows no date before its round is sent. After sending, in the current round and in past rounds, it shows `M/D` of the round's sent time, with the year and time on hover and focus.
- Every stamp that can no longer be pressed (past rounds, and the current round once sent; the person's and 代決 alike) also shows the year and time on tap.
- Tapping a stamp of the current, unsent round still presses or lifts it and shows no time.
- Past rounds show every question's stamp as it was sent, dated and not pressable. The past-round "代決" badge on questions is replaced by the pre-approved stamp.

Shown by: test (`web/tests/stamps.spec.js`, `web/tests/look-back.spec.js`).
- Set the page's time zone with `test.use({ timezoneId: "Pacific/Kiritimati" })` (UTC+14, so the local date differs from the UTC date for 14 hours a day). Read the stored UTC time from `api/view`, and compute the expected `M/D`, hour and minute in that zone. Never replace the server's clock.
- Assert the `M/D` on the stamp. Assert that the tooltip contains the four-digit year, then the zone's hour and the two-digit minute in that order (for example a pattern like `H\D+MM`), without pinning separators or layout. The date check alone is partial: it passes in UTC for the 10 hours a day when both dates agree, so the hour is what proves the zone is used.
- Tests:
  - `pressed_stamp_shows_its_month_and_day_and_the_year_and_time_on_hover_and_focus`
  - `pre_approved_stamp_has_no_date_until_sent_and_then_the_sent_date`
  - `stamps_of_a_sent_current_round_keep_their_dates`
  - `past_round_shows_each_stamp_with_its_date`. It replaces the question half of `pre_approved_answers_are_marked_in_past_rounds_and_decisions`; keep that test's decisions-tab half.
  - `clicking_a_past_round_stamp_shows_its_year_and_time`

Tapping on a real touch device is still the person's check in Step 12 ([判子](../spec/screen.md#判子) says so).

Left to implementer: the tooltip's form and the format of the year and time.

Stop and hand back if: none.

## Step 4 — Past result rounds as they were sent

Purpose: a past result round shows its own picture, records and how it was sent, and the stored "how sent" replaces the page's guess. Specification: [過去のラウンド](../spec/screen.md#過去のラウンド), [結果](../spec/screen.md#結果), [送ったあとの知らせ](../spec/screen.md#送ったあとの知らせ), [LLM の状態](../spec/server.md#llm-の状態).

Prerequisites: Steps 1 and 2.

May change: `web/view-data.js`, `web/components/past-rounds.js`, `web/components/result.js`, `web/strings.js`, `web/style.css`, `web/tests/`.

Done when:
- Choosing a sent result round in 過去のラウンド shows:
  - the round's stored finished picture;
  - its decisions as of that round, with 代決 per provisional answer 12;
  - its not-building, rejected, undecided and delegated lists;
  - how it was sent.
- It shows no send button, and "見直す" sits beside each decision.
- `proceeded()` in `web/view-data.js` reads the stored "how sent" of the current round instead of deriving it from today's in-review marks. This affects the after-send notice and the status that stays "working".

Shown by: test (`web/tests/look-back.spec.js`).
- `past_result_round_shows_its_picture_records_and_how_it_was_sent`. Change the picture and the undecided list in a later round, then check that the past result still shows the old ones, the how-sent mark, a "見直す" control beside each decision, and no send button.

Pressing "見直す" from a past result is tested in Step 5, where the confirmation exists. The existing `after-send.spec.js` tests keep passing.

Left to implementer: whether the past result reuses `ResultView` or a sibling component.

Stop and hand back if: none.

## Step 5 — 見直す with a confirmation, closed after sending, withdrawable until the conclusion

Purpose: a mistaken 見直す never reaches the LLM, and the window in which it cannot be sent shows on the screen. Specification: [見直す](../spec/screen.md#見直す), [まとめて送る](../spec/screen.md#まとめて送る), [結果](../spec/screen.md#結果), [直したこと](../spec/screen.md#直したこと), [過去のラウンド](../spec/screen.md#過去のラウンド).

Prerequisites: Steps 1, 2 and 4 (the past result round must exist for the last two tests).

May change: `web/components/decision-item.js`, `web/components/dialog.js`, `web/app.js`, `web/view-data.js`, `web/components/result.js`, `web/components/fixes.js`, `web/components/map.js` (its details panel's review control), `web/strings.js`, `web/style.css`, `web/tests/`.

Done when:
- Every "見直す" opens one confirmation dialog, held as the single open dialog in `app.js`, with 見直す and 見直さない.
  - 見直さない (or Escape) changes nothing.
  - 見直す sends `request_review`.
- The result wording is used only when 見直す is pressed inside the current, unsent result. Everywhere else, including a past result, the ordinary wording is used.
- While the current round is sent and the topic has not ended, every "見直す" is disabled on every tab (provisional answer 13).
- "見直しをやめる" shows for every decision in review until a round concludes it. It has no confirmation.
- The refusal kind Step 1 added for a review request in the sent window has its text in both languages (the "refused review request" row of the wording table).

Shown by: test (`web/tests/look-back.spec.js`, `web/tests/stamps.spec.js`).
- `review_asks_for_confirmation_and_dont_review_changes_nothing`
- `confirming_review_marks_in_review_and_reaches_the_waiting_llm`
- `review_cannot_be_pressed_after_sending_while_stop_review_still_works`. Assert that "見直す" cannot be pressed (disabled or absent), so the test survives an overturn of provisional answer 13.
- `stop_review_works_after_the_next_round_until_its_conclusion`. It replaces `review_request_can_be_stopped_before_next_round`.
- `current_result_uses_the_result_confirmation_and_a_past_result_the_ordinary_one`. In ja, assert the declared sentence 「見直しを頼むで送ると、次のラウンドで問い直されます」 in the first case and its absence in the second. Do not pin the ordinary wording.
- `review_from_a_past_result_marks_the_current_content`. A decision revised later, reviewed (and confirmed) from the past result, becomes in review under its current content.

Existing tests that click `[data-action=review]` add the confirmation step:
- `decision_in_review_is_marked_in_chains`
- `fixed_item_that_changed_a_decision_offers_review_request`
- `selected_decision_can_be_put_in_review_and_stopped_from_the_map`
- `result_with_pending_review_request_sends_review_requests_instead_of_proceed`

Left to implementer: how the dialog body is laid out.

Stop and hand back if: none.

## Step 6 — Header order, the finished picture in the tab row, the back stack, the page title

Purpose: the header holds exactly what [ヘッダーと切替](../spec/screen.md#ヘッダーと切替) lists in its order, and "戻る" follows [移動と現在地](../spec/screen.md#移動と現在地). Specification: [ヘッダーと切替](../spec/screen.md#ヘッダーと切替), [切替は設定に残る](../spec/screen.md#切替は設定に残る), [画面の構成](../spec/screen.md#画面の構成), [移動と現在地](../spec/screen.md#移動と現在地).

Prerequisites: Step 2.

May change: `web/components/header.js`, `web/app.js`, `web/view-data.js`, `web/style.css`, `web/strings.js`, `web/tests/`.

Done when:
- The header reads, left to right:
  - 戻る (only with a reachable place), then the title;
  - then a slot for the notification icon (filled in Step 9; it may already render the icon with no badge);
  - the LLM status;
  - the unreadable-config notice (only when unreadable);
  - the theme icon;
  - the language switch.
- The finished-picture button sits at the right end of the tab row on every tab (provisional answer 5).
- The back stack follows provisional answer 6.
  - Jumps and the "first question without a stamp" button push.
  - Tab switches and map range, selection, path and zoom changes do not push.
  - Two jumps and two backs return in reverse order.
  - Gone places are pruned.
- `document.title` is the topic's title (provisional answer 7).

Shown by: test (`web/tests/header.spec.js`, `web/tests/map.spec.js`, `web/tests/map-navigation.spec.js`).
- `header_lists_back_title_status_config_notice_theme_and_language_in_order`. Use the broken-config fixture so the notice shows. Check relative order only. Step 9 extends it with the notification icon.
- `finished_picture_opens_from_the_tab_row_on_every_tab`. It replaces `finished_picture_and_original_request_are_reachable_from_the_header_on_every_tab`; the original request keeps its header check.
- `two_jumps_then_two_backs_return_in_reverse_order`
- `switching_tabs_or_changing_the_map_does_not_show_back`
- `unstamped_button_can_be_returned_from_with_back`
- `back_skips_a_place_whose_card_is_gone`. Jump from a card of the current round, let the next round arrive, then check that "戻る" no longer offers that place.

Existing tests that change:
- Delete `map_back_restores_previous_range_selection_and_zoom`: its rule (map changes push) is no longer in the specification. `back_from_past_round_restores_map_range_and_selection` keeps covering the success condition.
- `back_control_is_in_the_header_and_only_when_there_is_history` creates its history by clicking a map node, which no longer pushes. Make it create history with a jump (for example a decision's source link), and keep its other assertions.
- `a_change_of_the_llm_status_leaves_the_opened_request_open_and_the_focus_in_place` focuses another header control instead of the moved finished-picture button.
- Keep the `data-action="finished-picture"` hook on the moved button. `finished_picture_opens_from_every_tab` and `finished_picture_draws_empty_slots_dashed` in `web/tests/map.spec.js` find it page-wide and need no change.

Left to implementer: how the anchor is found and stored.

Stop and hand back if: none.

## Step 7 — Map points: details on focus, panned into view

Purpose: keyboard users get a point's details, and a focused point off the view comes into view. Specification: [詳しい情報の出し方](../spec/screen.md#詳しい情報の出し方).

Prerequisites: Step 6.

May change: `web/components/map.js`, `web/pan-zoom.js`, `web/tests/map-navigation.spec.js`.

Done when:
- Focusing a map point shows the same details as hovering it, without moving other parts of the page. Blurring hides them.
- Focusing a point outside the visible area pans the map until the point is visible.

Shown by: test.
- `focusing_a_point_shows_its_details`
- `focusing_a_point_outside_the_view_pans_it_into_view`

Left to implementer: how far the pan moves past the edge.

Stop and hand back if: none.

## Step 8 — Wheel, Ctrl/⌘ and touch over diagrams

Purpose: diagrams stop swallowing the page's scroll. Specification: [地図と完成図を動かす](../spec/interaction.md#地図と完成図を動かす).

Prerequisites: Step 2.

May change: `web/pan-zoom.js`, `web/components/map.js`, `web/components/diagram-view.js`, `web/components/result.js`, `web/app.js` (the finished-picture dialog), `web/strings.js`, `web/style.css`, `web/tests/`.

Done when:
- A plain wheel over the map or a finished picture (the result's and the dialog's) scrolls the page, leaves the zoom unchanged, and shows the hint (provisional answer 10).
- Ctrl+wheel and ⌘+wheel over them zoom the diagram and call `preventDefault`.
- A reply's diagram reacts to neither: a plain wheel and Ctrl+wheel over it leave it unchanged, and the event's default is not prevented.
- `PanZoom` takes a mode:
  - the map tab and the dialog's finished picture keep one-finger dragging;
  - the finished picture inside the result (an in-page diagram) leaves one finger to the page (CSS `touch-action` allowing panning) and moves with two fingers;
  - pinch zooms in both.
- `map.how-to-move` and its ja text describe Ctrl/⌘+wheel instead of the wheel.

Shown by: test (`web/tests/map-navigation.spec.js`, `web/tests/diagrams.spec.js`). Record `defaultPrevented` with a listener the test adds.
- `plain_wheel_over_the_map_scrolls_the_page_and_shows_the_hint`. In ja, assert the declared hint 「Ctrl＋ホイールで拡大」.
- `plain_wheel_over_the_result_picture_leaves_its_zoom_unchanged`
- `ctrl_or_meta_wheel_zooms_the_diagram_and_prevents_the_default`. Use the map and the result's finished picture.
- `wheel_and_ctrl_wheel_over_a_reply_diagram_leave_it_unchanged_and_not_prevented`

Update `map_zooms_and_moves_inside_its_area` and `finished_picture_fits_the_view_and_can_be_zoomed` to zoom with Ctrl+wheel.

Real-browser behaviour (the page itself not zooming, touch, trackpad pinch, the ⌘ hint on a Mac) is the person's check in Step 12.

Left to implementer: how the hint is drawn and positioned over the diagram.

Stop and hand back if: on a real touch device, a one-finger page scroll over the in-page picture blocks two-finger moves (or the reverse) in a way `touch-action` cannot resolve. The specification's split would then need the person.

## Step 9 — Arrivals: toasts, the notification icon, the tab title

Purpose: nothing the LLM sends goes unnoticed, and every arrival can be reached by keyboard. Specification: [届いたものの知らせ](../spec/interaction.md#届いたものの知らせ), [ヘッダーと切替](../spec/screen.md#ヘッダーと切替), [移動と現在地](../spec/screen.md#移動と現在地).

Prerequisites: Step 6.

May change: `web/app.js`, `web/view-data.js` (the pure diff of two views into arrivals, and the excerpt), a new `web/components/toasts.js`, `web/components/header.js`, `web/components/thread.js` (opening a thread and expanding it on request), `web/components/card.js`, `web/components/provisional.js`, `web/components/current-round.js`, `web/components/past-rounds.js`, `web/components/question-parts.js` (opening the card or row that 見る lands on), `web/strings.js`, `web/style.css`, `web/tests/`.

Done when:
- Arrivals are found as in provisional answer 4.
  - An arrival already seen (provisional answer 8) gets no toast and does not count in the badge.
  - Any other arrival gets a toast with the table's text and 見る.
- 見る pushes the current place, then moves as in provisional answer 9.
  - A reply that arrived after sending lands in its past round.
- Toasts:
  - vanish after 6 seconds, but not while a pointer or the focus is on them;
  - stack vertically;
  - on the 4th, the oldest without pointer or focus goes;
  - when every toast is held, the new one stacks and the oldest goes once released.
- The notification icon sits right before the LLM status and shows the unseen count as a badge.
  - Opening its list (the last 10 arrivals) clears the badge.
  - Choosing an entry moves like 見る, and pushes onto the back stack (provisional answer 6).
  - An arrival moved to through its toast's 見る or a list entry counts as seen: it leaves the badge count.
  - The list and the count live only in the page's memory.
- While `document.hidden`, the title carries the arrival count. On return the count clears, and toasts appear only for arrivals not visible then. No OS notification is used.
- Toast and list state is screen-wide state held in `app.js` (ba0918-gui-structure). The toast component only draws it.

Shown by: test (`web/tests/arrivals.spec.js`, new; `web/tests/header.spec.js`). Time is driven with `page.clock`.
- `reply_to_an_unseen_card_shows_a_toast_whose_view_lands_on_the_reply_and_back_returns`. Also assert that after 見る the arrival no longer counts in the badge.
- `reply_visible_on_arrival_shows_no_toast_and_is_not_counted`
- `reply_in_another_tab_shows_a_toast`
- `view_on_a_reply_inside_an_elided_middle_expands_and_shows_it`
- `next_round_and_result_show_their_toasts`. In ja, assert the declared texts.
- `toast_vanishes_after_six_seconds_but_not_while_hovered_or_focused`
- `fourth_toast_removes_the_oldest_unheld_one`
- `notification_badge_counts_unseen_arrivals_and_clears_when_the_list_opens`
- `choosing_from_the_notification_list_moves_like_view`. Also assert that the chosen arrival leaves the badge count and that "戻る" returns.
- Extend `header_lists_back_title_status_config_notice_theme_and_language_in_order` (Step 6) to assert the notification icon comes right before the LLM status.

The background-tab title count and the toasts on return are the person's check in Step 12 ([届いたものの知らせ](../spec/interaction.md#届いたものの知らせ) says so).

Left to implementer: the toast's and icon's look (未決, [未決](../spec/interaction.md#未決)), and how the excerpt counts characters (code points or graphemes).

Stop and hand back if: a reply or round is found that arrived but was never announced, because comparing accepted views cannot see it. Provisional answer 4 is then overturned, and the server must send the kind.

## Step 10 — The operation promises

Purpose: keyboard, focus and reduced motion behave as [操作の約束](../spec/interaction.md#操作の約束) promises across everything added above. Specification: [操作の約束](../spec/interaction.md#操作の約束), [質問](../spec/screen.md#質問).

Prerequisites: Steps 5, 7 and 9.

May change: `web/` (only where a test finds a gap), `web/tests/interaction.spec.js` (new).

Done when:
- Using only the keyboard, the person can:
  - choose an answer, stamp it and ask;
  - confirm a review;
  - open the notification list and choose an entry;
  - send the round.
- A reply arriving while the person types in an ask field leaves the focus and the typed text in place.
- With reduced motion, the working status dot, the typing dots and the toasts do not animate.

Shown by: test.
- `keyboard_alone_answers_stamps_asks_reviews_and_sends`. Drive every bullet of the first Done item with `page.keyboard` only, including opening the notification list and choosing an entry.
- `reply_arriving_while_typing_keeps_the_focus_in_the_field`
- `reduced_motion_stops_the_status_dot_the_typing_dots_and_the_toasts`. Use `page.emulateMedia({ reducedMotion: "reduce" })` and check the computed `animation-name` (and transition duration) of the status dot, the typing dots and a toast.

The last two cover behaviour that may already hold. If they pass at once, record that, and change no product code for them. They are kept because [操作の約束](../spec/interaction.md#操作の約束) names these automated checks and no test covers them yet.

Whether every operation is reachable by keyboard is the person's check in Step 12.

Left to implementer: none.

Stop and hand back if: an operation the specification names cannot be reached by keyboard without a new shortcut. [見送った案](../spec/interaction.md#見送った案) rejects shortcuts.

## Step 11 — Skill, README and changelog

Purpose: the agent's instructions and the user's documents describe the revised behaviour. Specification: [skill.md](../spec/skill.md) ([待ち方](../spec/skill.md#待ち方), [見直しの問い直し](../spec/skill.md#見直しの問い直し), [収束したとき](../spec/skill.md#収束したとき)), [LLM の状態](../spec/server.md#llm-の状態), [状態データ](../spec/server.md#状態データ).

Prerequisites: Steps 1–10.

May change: `skills/shoryo/SKILL.md`, `skills/shoryo/references/events-and-replies.md`, `skills/shoryo/references/round.md`, `README.md`, `CHANGELOG.md`.

Done when:
- Every 見直したい in `skills/shoryo/` (including `SKILL.md`'s "Review requests" heading and `events-and-replies.md`'s `review_requested` row) becomes 見直す.
- `SKILL.md` tells the agent how to handle a late `review_stopped` (provisional answer 14).
- `events-and-replies.md`:
  - says that `review_requested` only arrives while a round is open;
  - says that `review_stopped` may arrive until the agent sends that decision's conclusion;
  - updates "The result" table to the stored shape Step 1 left: adds, renames or removes whichever fields Step 1 changed (the stamp time, the round's sent time, the per-round picture and records, how a result was sent).
- README:
  - says "ask the LLM about a question" instead of "ask back";
  - names the counted commands for "not responding" (`round`, `wait`, `reply`, `end`; not `result` or `stop`);
  - mentions the toasts and the notification icon;
  - lists `docs/spec/interaction.md`.
- `CHANGELOG.md` `## [Unreleased]` describes the user-visible changes.

Shown by: check — `PATH=$HOME/.cargo/bin:$PATH scripts/check.sh`, then `rg 見直したい skills/shoryo` finds nothing. `skill_examples_are_accepted` feeds every example in the references to the binary, so any JSON example added must be accepted.

Left to implementer: wording.

Stop and hand back if: none.

## Step 12 — The person's check

Purpose: the checks the specification gives to the person. Specification: [判子](../spec/screen.md#判子), [質問](../spec/screen.md#質問), [地図と完成図を動かす](../spec/interaction.md#地図と完成図を動かす), [届いたものの知らせ](../spec/interaction.md#届いたものの知らせ), [操作の約束](../spec/interaction.md#操作の約束), [未決](../spec/screen.md#未決).

Prerequisites: Steps 1–11.

May change: nothing.

Done when: the person has confirmed each item below on a real browser, or turned one into a finding.
- Tapping a stamp that can no longer be pressed on a touch device shows its year and time.
- A screen reader announces that a reply is being written.
- Ctrl+wheel over a diagram never zooms the page.
- Touch and trackpad pinch work.
- On a Mac, ⌘+wheel zooms and the hint shows ⌘.
- With the browser tab in the background, the title shows "(n) 議題名", and returning clears it and shows toasts.
- Every operation is reachable by keyboard.
- The look of stamps, the header and toasts.

Shown by: external. The person runs `cargo run -- start <topic>`, drives a few rounds through the agent commands, and observes each item above. Each item passes when it behaves as its linked heading says.

Left to implementer: none.

Stop and hand back if: none.
