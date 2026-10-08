# Plan: fixes from the person's screen review

## Goal

After this plan:

- the ask-and-reply thread reads like a chat;
- diagrams (the finished picture and replies) are never clipped and their labels stay inside their boxes;
- the stamp looks like a stamp;
- the card's secondary controls say what they do and sit where they belong;
- "back" is always reachable;
- after sending, the person can see what is happening and what the LLM is doing.

## Specification

- [画面](../spec/screen.md), with the revision "docs: 聞き返しを、開け閉めと時系列の会話として出すように仕様を改める" on this branch
- [起動と往復と記録](../spec/server.md)
- [CONTEXT.md](../../CONTEXT.md)

Read the specification in Japanese. The links below point at its headings.

## Approach and why

The person reviewed the screen built by `docs/plans/screen-polish.md` through a kemi live review. The findings, in substance:

1. **Finished picture.** The whole picture is clipped and text runs out of the shapes. It has the same problem the map had.
2. **Stamp.** A pressed stamp should look more like a real seal.
3. **Ask buttons.** A preset ask button sends to the LLM the moment it is pressed. Put one step in between, so a mistaken press does not send.
4. **Fold toggle.** "たたむ" does not say what it folds.
5. **Note field.** It is not clear that the note is sent with the answer.
6. **Defer switch.** It sits in an odd place and would read better next to the heading.
7. **Decision item.** The heading, the full text, and "決まった問い: …" seem to say the same thing twice.
8. **Tab row.** It shows a stray scrollbar.
9. **Back.** If "back" stays on screen, it must be in the fixed header. Otherwise the person scrolls to reach it every time.
10. **Chat.** The ask/reply thread does not feel like a chat. The specification was revised for this ([聞き返し](../spec/screen.md#聞き返し)).
11. **After sending.** The waiting state is hard to read. After "Proceed with this result" the screen wrongly says it is waiting for the next round.

Item 10 and the ask buttons (3) changed the specification. The rest fit inside it. Visual details remain the person's call ([未決](../spec/screen.md#未決)), and the person checks them again at the end.

Order of the work:

- The thread and the diagrams come first, because they are the largest visible problems.
- The card polish comes next.
- The header and the waiting state come last, because they touch the server (the LLM's status).

### Provisional answers

Each answer has an overturn condition: what the person would say to reverse it.

1. **The LLM's status is shown in the header**, the way kemi shows it.
   - States: waiting (a `wait` command is in progress), working (no `wait` in progress since the last event was delivered), not responding (no `wait` and no reply for 10 minutes).
   - The server records when a `wait` starts and ends. The page receives the status with its other live updates.
   - Overturn if: the person wants no status, or other wording.
2. **After sending, a clear notice replaces the faint text in the send bar.** It sits at the top of the current round and says what was sent and what happens next.
   - For an ordinary round: "Sent. The LLM is reading your answers; the next round will appear here."
   - After "Proceed with this result": "You proceeded with this result. The LLM now writes the specification; nothing more is needed on this screen."
   - The Japanese strings say the same.
   - Overturn if: the person rewrites the wording.
3. **Stamp look.** A pressed stamp is a vermilion double-ring seal with "Approve" / 「確認」 and the date, slightly rotated, with a faint ink texture. A pre-approved stamp has the same shape, drawn dashed and grey-blue, with "Pre-approved" / 「代決」. Overturn if: the person wants another design.
4. **Fold toggles name their contents.** For example "Show premises and option details" / 「前提と選択肢の説明を開く」, and the matching "hide". Overturn if: the person rewrites the wording.
5. **Note field.** It is labelled "Note sent with this answer (optional)" / 「この答えに添えて送る補足（任意）」 and placed directly under the options. Overturn if: the person rewrites the wording.
6. **Defer switch.** It moves to the card head, next to the class badge. Overturn if: the person prefers it next to the options.
7. **Decision item.** It shows the name, the full text, and one small line "← <question text>" that links to the question. The "決まった問い" label goes. Overturn if: never.
8. **Thread.**
   - An exchange is one ask together with its reply.
   - The elision rule ([聞き返し](../spec/screen.md#聞き返し)) counts exchanges.
   - A follow-up quotes the first 40 characters of the reply it continues, and clicking the quote scrolls to that reply.
   - The send button sits inside the input as an icon button (paper plane) with an accessible name.
   - Preset ask buttons appear as small suggestions above the input.
   - Overturn if: the person wants other limits or placement.
9. **Diagrams.** Diagrams use the same text measuring as the map:
   - Node labels wrap inside the box. A box grows in height up to three lines, and longer text is cut with "…" and shown in full on hover or tap.
   - The finished-picture view fits the whole picture by default, and can zoom and pan like the map.
   - A diagram inside a reply is sized to its content, never wider than the reply.
   - Overturn if: the person wants a different maximum.
10. **Back in the header.** The back control moves into the fixed header and appears only while there is somewhere to go back to. Overturn if: never.

## Scope of change

`web/`, `web/tests/`, `crates/shoryo-server/` (the LLM's status only), `crates/shoryo-core/` (only if the status needs a state field), `tests/`, `README.md`.

## Step order and prerequisites

1. The chat thread
2. Diagrams
3. Card and decision polish
4. The header's back control, the post-send notice, and the LLM's status
5. The person reviews the result

Steps 2 and 3 need step 1 only because they share the card. Step 4 is independent of steps 2 and 3. Step 5 needs all of them.

## Steps

## Step 1 — The thread reads like a chat

Purpose: [聞き返し](../spec/screen.md#聞き返し) as revised, and finding 3. Specification: [聞き返し](../spec/screen.md#聞き返し).
Prerequisites: none.
May change: `web/`, `web/tests/`.
Done when:
- The thread opens and closes under its heading "Ask back (n)" / 「聞き返し（n）」, and starts open when the question has asks.
- Exchanges appear in time order in one flow.
- Follow-ups carry a quote of the reply they continue (provisional answer 8).
- More than 5 exchanges collapse the middle into a "… n exchanges …" control that expands everything.
- Preset ask buttons only fill the input.
- Sending happens through the send button or Enter.
- The old latest/all/hide control and the nested reply tree are gone.

Shown by: test — browser tests:
- `thread_opens_and_closes_and_starts_open_when_there_are_asks`
- `follow_up_is_placed_in_time_order_with_a_quote_of_the_reply_it_continues`
- `more_than_five_exchanges_collapse_the_middle_and_expand_on_click`
- `preset_ask_button_fills_the_input_without_sending`
- `send_button_and_enter_send_the_ask`

Tests for the removed three states are deleted.

Left to the implementer: the thread's styling.
Stop and hand back if: none.

## Step 2 — Diagrams are never clipped

Purpose: finding 1 (provisional answer 9). Specification: [図](../spec/screen.md#図), [完成図](../spec/screen.md#完成図).
Prerequisites: step 1.
May change: `web/`, `web/tests/`.
Done when:
- In both the finished-picture view and replies, no node label extends past its box, whatever the length of the Japanese text.
- The finished picture fits the view by default, and can be zoomed and panned.
- A reply's diagram is no wider than the reply and has no large empty frame.
- Grid positions are still honoured ([図](../spec/screen.md#図)).

Shown by: test — browser tests:
- `long_japanese_diagram_label_stays_inside_its_box`
- `finished_picture_fits_the_view_and_can_be_zoomed`
- `reply_diagram_is_sized_to_its_content`

The existing grid-position tests keep passing.

Left to the implementer: none.
Stop and hand back if: none.

## Step 3 — Card and decision polish

Purpose: findings 2, 4, 5, 6, 7 and 8 (provisional answers 3–7). Specification: [たたんだカード](../spec/screen.md#たたんだカード), [判子](../spec/screen.md#判子), [先送りのラベル](../spec/screen.md#先送りのラベル), [決まったこと](../spec/screen.md#決まったこと), [画面の構成](../spec/screen.md#画面の構成).
Prerequisites: step 1.
May change: `web/`, `web/tests/`.
Done when: each of provisional answers 3–7 holds, and the tab row has no scrollbar at 1280 and 390 px wide.
Shown by: check — `scripts/test-web.sh` passes. Existing tests may change only by new `data-*` hooks where the structure moved. The look is the person's check in step 5.
Left to the implementer: exact styling.
Stop and hand back if: none.

## Step 4 — Back in the header, a clear post-send notice, and the LLM's status

Purpose: findings 9 and 11 (provisional answers 1, 2, 10). Specification: [移動と現在地](../spec/screen.md#移動と現在地), [まとめて送る](../spec/screen.md#まとめて送る), [結果](../spec/screen.md#結果), [待つ](../spec/server.md#待つ).
Prerequisites: none.
May change: `web/`, `web/tests/`, `crates/shoryo-server/`, `crates/shoryo-core/` (status field only, if needed), `tests/`, `README.md`.
Done when:
- The back control is in the fixed header and shows only when there is history.
- After sending, the notice of provisional answer 2 appears, with the right text for an ordinary round and for a result.
- The header shows the LLM's status, which changes when a `wait` starts and ends and turns "not responding" after 10 minutes without either.

Shown by: test —
- browser tests:
  - `back_control_is_in_the_header_and_only_when_there_is_history`
  - `after_send_the_notice_says_what_happens_next`
  - `after_proceeding_with_a_result_the_notice_does_not_promise_a_next_round`
  - `header_shows_waiting_while_the_agent_waits_and_working_after_an_event_is_delivered`
- round-trip test: `status_reports_whether_a_wait_is_in_progress`

The 10-minute boundary is made testable through an injected clock, never by sleeping.

Left to the implementer: none.
Stop and hand back if: none.

## Step 5 — The person reviews the result

Purpose: the visual check ([未決](../spec/screen.md#未決)).
Prerequisites: steps 1–4.
May change: nothing unless the person asks.
Done when: the person has reviewed the running screen through `kemi --live` and said whether these findings are resolved.
Shown by: external — the person runs it.
Left to the implementer: none.
Stop and hand back if: none.

## Verification map

| Specification section | Steps |
|---|---|
| [聞き返し](../spec/screen.md#聞き返し) | 1 |
| [図](../spec/screen.md#図), [完成図](../spec/screen.md#完成図) | 2 |
| [判子](../spec/screen.md#判子), [たたんだカード](../spec/screen.md#たたんだカード), [先送りのラベル](../spec/screen.md#先送りのラベル), [決まったこと](../spec/screen.md#決まったこと) | 3 |
| [移動と現在地](../spec/screen.md#移動と現在地), [まとめて送る](../spec/screen.md#まとめて送る), [結果](../spec/screen.md#結果), [待つ](../spec/server.md#待つ) | 4 |
| [未決](../spec/screen.md#未決) | 3, 5 |

## Left to the implementer

Names, internal structure, and styling within the provisional answers.

## Stop conditions

Beyond the general ones (skill `ba0918-cycle`): a step needs a change to `docs/spec/` or `CONTEXT.md`.

## Test command

`scripts/check.sh` and `scripts/test-web.sh`. If `cargo --version` is not 1.99.0, prefix with `PATH=$HOME/.cargo/bin:$PATH`. Never use `git stash` in this worktree; the stash is shared with other worktrees. Use a temporary commit or a scratch copy instead.

## Out of scope

Anything the person did not raise in the review. Release.
