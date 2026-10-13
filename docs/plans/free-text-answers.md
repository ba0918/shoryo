# Free-text answers

Implementation plan against approved specification commit `11d015f`.
This document proposes future work; it is not implementation evidence or approval.

## Goal

Let a person answer any question in their own words, retain drafts, and send exactly the answer they confirmed, including when another tab changes it.

## Specification

Read these existing sections before their dependent steps; the specifications remain authoritative.

- [Screen](../spec/screen.md): [答え欄の初期値](../spec/screen.md#答え欄の初期値), [その他の回答](../spec/screen.md#その他の回答), [この答えだと](../spec/screen.md#この答えだと), [仮決めの一覧](../spec/screen.md#仮決めの一覧), [先送りのラベル](../spec/screen.md#先送りのラベル), [判子](../spec/screen.md#判子), [まとめて送る](../spec/screen.md#まとめて送る), [過去のラウンド](../spec/screen.md#過去のラウンド), [詳しい情報の出し方](../spec/screen.md#詳しい情報の出し方).
- [Server](../spec/server.md): [待つ](../spec/server.md#待つ), [開き直し](../spec/server.md#開き直し), [状態データ](../spec/server.md#状態データ), [自由記述回答の保存と送信](../spec/server.md#自由記述回答の保存と送信), [旧形式への対応範囲](../spec/server.md#旧形式への対応範囲).
- [Skill](../spec/skill.md): [問いの書き方](../spec/skill.md#問いの書き方), [送られた答えの扱い](../spec/skill.md#送られた答えの扱い).
- [Interaction](../spec/interaction.md): [操作の約束](../spec/interaction.md#操作の約束).

Use [PROJECT.md](../../PROJECT.md), [AGENTS.md](../../AGENTS.md), and [CONTEXT.md](../../CONTEXT.md) for commands, rules, and terminology.
The old explanation-parts exemption does not waive compatibility for old answers or pending events.

## Approach and why

Extend the existing domain and browser components rather than introduce another saved answer owner.
The server remains authoritative; components hold only local unsaved editing buffers.
Build the sent-answer projection once in the core and reuse it for submission, guarded comparison, and submitted-answer display data.
Browser formatting may turn an option index into its label, but must not independently decide which draft is sent.

### Reuse layers

The repository search found an existing implementation at the first applicable reuse rung for each layer.

| Layer | Adopt or build | Reason |
|---|---|---|
| Input | Adopt native textarea/radios and `web/components/question-parts.js`; build the additional method controls | Existing card and provisional-row inputs already share these pieces. |
| State | Adopt serde, `Answer`, and domain operations; build minimal method and projection extensions | These already own transitions and persisted answers; a separate model would duplicate ownership. |
| Storage/network | Adopt server atomic change/store, browser queue, and SSE; build guarded submission and awaited flushing | Existing infrastructure supplies persistence and serialization, but not confirmation identity. |
| Display | Adopt `web/view-data.js` and existing components; extend their answer view data | Existing confirmation, history, and map rendering can receive method-aware data. |
| Tests | Adopt Cargo `domain`/`round_trip` and Playwright fixtures; add behavioral cases | These exercise the pure core and real binary without a new harness or dependency. |

### Provisional answers and overturn conditions

These are proposed contract decisions for upstream review, not choices silently delegated to implementation.

1. **Stored draft contract:** retain `selected: usize`, `note`, `deferred`, `stamp`, and `stamped_at`; add `method: "choice" | "custom"` with serde default `"choice"`, and `free_text: String` with default `""`.
   Keep the retained selected index valid even in custom mode. Initial answers remain choice/recommendation.
   This preserves old data and both drafts without destructive migration.
   **Overturn if:** the owner requests a different persisted contract, or a demonstrated compatibility problem requires a superior representation; fix the replacement contract in the plan before coding it.
2. **Editing operations:** add `{"op":"answer_method","question":id,"method":"choice"|"custom"}` and `{"op":"free_text","question":id,"text":string}`.
   Existing `choose` also activates choice mode; selecting choice through `answer_method` restores the retained index.
   Actual method, option, free-text, or deferral changes clear stamp/time; identical saves do not. Note changes retain stamps.
   Drafts survive switches. Custom answers moved between classes remain unstamped, including when deferred; no custom answer gains LLM preapproval.
   **Overturn if:** an existing consumer requires other operation names/shapes, while retaining these transitions; custom preapproval would conflict with the specification and must be handed back.
3. **Submitted event contract:** retain all existing `SentAnswer` fields; add nullable `free_text`, serde-default absent to `None`, omit it when absent on serialization.
   Choice: `choice:index`, no `free_text`, `deferred:false`, retained note.
   Custom: `choice:null`, nonempty `free_text`, `deferred:false`, `note:""`, person stamp.
   Deferred: `choice:null`, no `free_text`, `deferred:true`; keep existing note only for choice-mode deferral, and use `note:""` for custom-mode deferral.
   Thus `deferred` plus `free_text`, never `choice:null` alone, distinguishes variants. Hidden selected/custom drafts never enter events.
   Validate accepted variants at the canonical core boundary, including stored pending events and guarded input; reject contradictory variants instead of guessing.
   Old choice/deferred pending events without `free_text` remain valid and retain identifiers/acknowledgment behavior.
   **Overturn if:** the owner requires another additive discriminator or different defer-note semantics; do not weaken old-event compatibility or mix unused drafts into answers.
4. **Guarded submission:** extend Submit to `{"op":"submit","round":n,"expected_answers":[SentAnswer,...]}`.
   Add top-level `expected_answers` to the page view: canonical round-order projection, or `null` when an open round cannot currently be submitted; result rounds use `[]`.
   Freeze this array and its display data when confirmation opens. Compare the exact ordered array, including question/class/choice/free_text/note/deferred/stamp but not stamp timestamps, to the current canonical projection inside the existing state lock, before any mutation.
   Return a structured `answers_changed` refusal (HTTP 409) for a mismatch; invalid custom input uses `empty_free_text` with question identifiers. Missing stamps keep the existing refusal.
   A malformed expected variant is rejected without state changes. Omission/null of `expected_answers` provisionally retains legacy unguarded submission, with normal validity checks; every new-browser Submit includes an array.
   This catches note/class/text changes without mistaking asks, replies, or wait activity for answer changes. It cannot be replaced by a check of the whole watch version.
   **Overturn if:** the owner requires rejection of legacy unguarded clients, or a different answer-specific guard demonstrably preserves the same atomic comparison; declare that contract before implementation.
5. **Draft validity:** saving an empty draft is allowed. Server emptiness uses Rust `trim`; preserve the original nonempty string exactly, including newlines and leading/trailing whitespace.
   JS `trim` agrees for ordinary/full-width spaces; use Unicode `White_Space` matching for the final UI emptiness predicate to match Rust, including empty U+0085 and nonempty U+FEFF. Test these cases rather than normalize stored text.
   Do not invent a text size cap beyond existing HTTP transport bounds, and never truncate silently.
   **Overturn if:** the owner requires a different whitespace definition or explicit limit; a new acceptance boundary requires specification resolution.
6. **Local feedback and failures:** show the empty-answer reason beside a chosen/touched custom field or after an attempted critical action, not on untouched choice pages.
   Await actual save success before stamping, opening confirmation, or sending; an HTTP refusal or network failure stops the dependent action and keeps the unsaved buffer available.
   Reuse existing failure presentation; do not infer successful submission from a disconnected request or automatically resend it.
   On changed answers, fetch latest state and reopen a latest confirmation requiring another explicit send; if stamps are missing, return to answering/restamping before reconfirmation.
   Never silently replace a confirmed snapshot during rendering or send updated text under the old confirmation.
   **Overturn if:** the owner requests another feedback/reconfirmation interaction that still prevents unconfirmed submission and preserves recoverable drafts.

## Scope of change

- Core: `crates/shoryo-core/src/{state,operations,lib}.rs`, a focused answer/projection module if needed, and `crates/shoryo-core/tests/domain/`.
- Server: `crates/shoryo-server/src/{app,store}.rs`; tests/helpers under `tests/round_trip/` for HTTP, wait/result, and restart evidence.
- Browser: `web/app.js`, `web/view-data.js`, `web/strings.js`, existing styles, and `web/components/{question-parts,card,provisional,current-round,confirm-send,past-rounds,map}.js` as needed.
- Browser tests: `web/tests/free-text-answers.spec.js` plus relevant existing tests and fixtures/helpers only where the changed contract requires it.
- Agent contract: `skills/shoryo/SKILL.md`, `skills/shoryo/references/events-and-replies.md`, and `skills/shoryo/references/round.md` only if its instructions/examples need clarification.
- Update existing Rust Operation/Answer/SentAnswer literals and test helpers narrowly for new fields, preserving their original behavior checks.

No other plan or specification is editable under this plan, especially `docs/plans/spec-revision-2.md`.

## Step order and prerequisites

Run steps 1–7 in order. Step 1 fixes the wire/projection boundary; step 2 proves persistence and atomic HTTP handling; steps 3–5 consume that contract; step 6 documents it; step 7 collects final evidence.
Before production work, read routed implementation, Rust, GUI, testing, and skill-authoring rules as applicable.
Use the pinned existing toolchain and installed Cargo/Playwright environment. Missing prerequisites are a blocker to report, not permission to change manifests/toolchains or CI.
Use temporary isolated topic data supplied by the existing harnesses, never a person's real topic files.

## Verification map

| Requirement sections | Evidence steps |
|---|---|
| Screen initial answer, Other, consequences, provisional list | 1 and 3: state transitions plus real browser controls/draft restoration |
| Screen stamps/deferral; server draft validation | 1–4: core rejection/no mutation, API rejection, immediate-input ordering |
| Screen send; server save/send and reopening | 2 and 4: real HTTP guard, restart, two-page race/reconfirmation |
| Server state, wait, old-format scope | 1 and 2: old answers and unacknowledged events through real load/restart/wait/result |
| Screen history and map detail | 5: submitted read-only text in both map node contexts |
| Interaction operation promises | 3–5: keyboard path, focus across saves/SSE/folded rows; 6: human completeness review |
| Skill question writing and sent-answer handling | 6: document/semantic review, not exact-wording assertions |
| Screen Other/send confirmation meaning | 6: implemented confirmation reviewed at final acceptance for choice+note, custom with inactive drafts, and custom+deferred |
| Whole integration regression | 7: existing browser suite and final repository gates |

## Left to the implementer

Internal helper names/module extraction, behavior-neutral layout spacing, and English/Japanese wording that preserves the specified meaning.
Choose tests by meaningful observable behavior, not by matching their count to fields or specification paragraphs.
Preserve existing tests; adjust setup/literals rather than weaken established behavior assertions.

## Stop conditions

Hand back any conflict with commit `11d015f`, missing behavioral oracle, need for new dependencies/toolchain/CI, or evidence that the chosen contract cannot preserve old answers/events.
Do not modify specs to resolve a conflict. Report the section, evidence, and proposed resolution.
Report unrelated failures separately; findings do not widen this scope. Acceptance belongs upstream, not to the implementer.

## Verification commands and TDD evidence

Run from the repository root. Cargo metadata confirms the integration targets are `domain` (core) and `round_trip` (root binary), not a guessed server integration target.
For every production behavior, write a meaningful failing test first, execute its scoped command, and record the observed failure before changing production code.
Repeat that command for GREEN; run the corresponding complete affected suite, then refactor only while green and rerun it (or record why no refactor is warranted).
Keep shell command/output evidence for RED/GREEN/REFACTOR; do not assert that phases happened without observed output.

```sh
cargo test -p shoryo-core --test domain --locked
cargo test -p shoryo-server --lib --locked
cargo test -p shoryo --test round_trip --locked
scripts/test-web.sh web/tests/free-text-answers.spec.js
scripts/test-web.sh web/tests/stamps.spec.js web/tests/current-round.spec.js web/tests/look-back.spec.js web/tests/map.spec.js web/tests/interaction.spec.js web/tests/after-send.spec.js
scripts/test-web.sh
scripts/check.sh
```

Use Cargo's trailing test-name filter or Playwright's `--grep` for RED of a newly named behavior; `scripts/test-web.sh` forwards Playwright arguments.
The server lib command applies when server-local tests are added; real HTTP coverage remains in `round_trip`.
Run affected suites during TDD; run the complete browser suite and `scripts/check.sh` once at the final gate after scoped tests pass. Repeat a failed gate only after a relevant fix.
Do not test document wording or count tests mechanically. Step 6 uses document/human review instead.

## Out of scope

Dependencies, toolchain, manifests, CI, releases, version bumps, changelog requirements, new documentation trees, explanation-parts migration, other brainstorm workflows, automatic retries, and real-data cleanup.
This plan-writing task creates only this file; the following steps describe a later implementation, after upstream approval.

## Step 1 — Compatible core answers and one sent-answer projection

Purpose: establish the method-aware state and event contract without losing old answers.
Specification: [状態データ](../spec/server.md#状態データ), [自由記述回答の保存と送信](../spec/server.md#自由記述回答の保存と送信), [待つ](../spec/server.md#待つ), [旧形式への対応範囲](../spec/server.md#旧形式への対応範囲), [判子](../spec/screen.md#判子), [先送りのラベル](../spec/screen.md#先送りのラベル), [答え欄の初期値](../spec/screen.md#答え欄の初期値).
Prerequisites: approved plan/provisional contracts; read the existing state, operation, domain-test, and load paths.
May change: scoped core files/module and `crates/shoryo-core/tests/domain/`; affected Rust literals/helpers elsewhere only for compilation.
Done when: both drafts survive method switches; initial recommendation remains; method/text changes clear stamps while identical saves and note edits do not; class swaps never preapprove custom answers.
Empty/ordinary-space/full-width-space custom stamping and sending are rejected without changing state/events/times; empty saves and deferred custom sends succeed.
The canonical projection emits only the three declared variants, retains exact multiline text, and excludes inactive drafts; old answers and pending choice/deferred events deserialize unchanged in meaning.
Construct and validate all projected answers before setting `submitted`, `sent_at`, or allocating an event; direct core refusal must leave the entire Topic unchanged, not merely rely on server copy-on-write.
Shown by: test — `cargo test -p shoryo-core --test domain --locked`; add behavior-named tests such as `custom_drafts_survive_method_switches`, `changed_custom_text_requires_a_new_stamp`, `empty_custom_confirmation_leaves_topic_unchanged`, and `old_pending_answers_keep_their_meaning` as applicable.
Include malformed variant rejection, invalid choice index, same-value stamp/time preservation, and deferred payload exclusion using real state and serialization, not mock-call assertions.
Left to the implementer: enum/helper/module names and case organization; wire names and validity rules are fixed above.
Stop and hand back if: a pre-feature answer/event fixture cannot load with current explanation-parts data without destructive migration, or a projection cannot be reused without duplicate saved ownership.

## Step 2 — Atomic guarded submission and restart compatibility

Purpose: expose the core snapshot and enforce its comparison through real persistence and HTTP.
Specification: [自由記述回答の保存と送信](../spec/server.md#自由記述回答の保存と送信), [開き直し](../spec/server.md#開き直し), [状態データ](../spec/server.md#状態データ), [待つ](../spec/server.md#待つ), [旧形式への対応範囲](../spec/server.md#旧形式への対応範囲).
Prerequisites: step 1; existing isolated binary/HTTP round-trip harness.
May change: scoped core guarded Submit/refusals/tests, server `app.rs`/`store.rs`, `tests/round_trip/`, and contract-required literals/helpers.
Done when: `expected_answers` is compared to the canonical projection under the same Mutex held by `change_or` for change/save/commit, never before acquiring it or after releasing it.
Stale/invalid/empty rejection changes neither stored bytes, submitted state, timestamps, pending events, nor published mutation notifications; unrelated asks/waits do not falsely invalidate matching answers.
Old pending events remain readable through the actual `StoredTopic` serde/load path, server restart, wait/ack, and result; both new drafts and hidden retained notes survive reload/restart without entering custom sends.
Shown by: test — domain and `round_trip` commands above; server lib command only for meaningful server-local tests.
Add a deterministic HTTP scenario: capture confirmation, commit another client's note/text change (restamp text), then submit the old snapshot and observe HTTP 409 and unchanged persisted submission/event state.
Also coordinate concurrent change/Submit requests without sleeps: only a send matching the state at its locked acceptance may succeed; if change wins, stale send is refused. A successful send makes the later edit refuse.
Use real request completion/barriers in test code, not a production clock/race-control API; prove a matching guarded send and legacy omission still succeed.
Left to the implementer: test coordination helpers and refusal display wording preserving the declared structured kinds.
Stop and hand back if: storage validation would waive answer compatibility under the explanation exemption, or race evidence needs a new production-only testing interface.

## Step 3 — Shared Other editor with local drafts and accessible feedback

Purpose: make custom answering available in cards and folded provisional rows.
Specification: [その他の回答](../spec/screen.md#その他の回答), [答え欄の初期値](../spec/screen.md#答え欄の初期値), [この答えだと](../spec/screen.md#この答えだと), [仮決めの一覧](../spec/screen.md#仮決めの一覧), [判子](../spec/screen.md#判子), [操作の約束](../spec/interaction.md#操作の約束).
Prerequisites: steps 1–2; existing Playwright fixture and component event flow.
May change: scoped question/card/provisional/current-round components, `view-data.js`, `app.js`, strings/styles, and browser tests/helpers.
Done when: every question has a system-provided Other control after its real options, without altering LLM options/recommendation; selecting it reveals a multiline field even while folded and hides consequence/note.
Switching radios restores both drafts and the retained option; folding/unfolding preserves the local buffer and saved content. Blank custom cannot be stamped unless deferred; feedback follows provisional answer 6.
Drawing only reads prepared view data; edits emit events upward. Domain/top-level owns saved state, field/region owns only its unsaved buffer, and another region never writes either.
Keyboard selection/editing/deferral/stamping works; retained controls keep focus/caret through save/SSE redraws. No custom class swap receives preapproval.
Shown by: test — `scripts/test-web.sh web/tests/free-text-answers.spec.js` plus affected existing scoped suites; add observable cases for folded custom input, radio draft restoration, blank/deferred validity, and keyboard focus retention.
Exercise component drawing/events from supplied view data through the existing browser harness as well as the real top-level path; assert effects, not internal debounce call counts.
Left to the implementer: behavior-neutral component extraction and layout/labels in both languages.
Stop and hand back if: a second saved answer owner or draw-time mutation is required, or editing would force unrelated GUI restructuring.

## Step 4 — Awaited flushing and frozen confirmation

Purpose: keep immediate-input and cross-tab send actions faithful to the confirmed answer.
Specification: [自由記述回答の保存と送信](../spec/server.md#自由記述回答の保存と送信), [まとめて送る](../spec/screen.md#まとめて送る), [判子](../spec/screen.md#判子), [開き直し](../spec/server.md#開き直し), [操作の約束](../spec/interaction.md#操作の約束).
Prerequisites: steps 1–3 and their green suites; guarded page-view contract.
May change: `app.js`, `view-data.js`, question draft-saving pieces, `confirm-send.js`, strings, and scoped browser tests/helpers.
Done when: the queue returns an awaitable real success/failure result; merely enqueued operations or swallowed rejections cannot authorize the next action.
Replace/extend the existing 400 ms note saver carefully: cancel/drain pending draft timers, flush dirty buffers, await saves, and derive fresh server data before stamp, confirmation, and final Submit. Do the same before switching methods/classes or folding away an edited field.
Do not rewrite a clean field from this tab over another tab's newer saved answer. Ensure delayed callbacks cannot overwrite a later edit or remove a newly applied stamp.
Freeze confirmation's display/snapshot in top-level dialog state; rendering never recomputes it from changing live answers. Final send flushes again and submits that frozen snapshot, not a newly substituted one.
Refused/failed saves stop stamp/confirm/send. A changed snapshot requires latest explicit reconfirmation (and restamping where necessary); unrelated SSE updates leave the confirmation meaningful.
Shown by: test — scoped free-text and stamps/after-send/interaction browser commands above.
Test input followed immediately by stamp, confirmation, and send before debounce expiry; delay/intercept save completion and inject refusal/network failure to prove dependent actions do not occur.
Use two real pages: A confirms, B edits and restamps, A sends; also hold A's Submit request until B's save completes to exercise the server safeguard despite delayed SSE. Observe no stale event and a new explicit latest confirmation before the exact latest text reaches wait.
Left to the implementer: buffer/flush helper organization and test interception mechanics, preserving real server acceptance checks.
Stop and hand back if: queue changes require automatic resubmission, or latest state cannot be confirmed without silent replacement of confirmed content.

## Step 5 — Read-only sent answers in history and both map contexts

Purpose: display what was sent rather than unused retained drafts or an invalid option lookup.
Specification: [過去のラウンド](../spec/screen.md#過去のラウンド), [詳しい情報の出し方](../spec/screen.md#詳しい情報の出し方), [その他の回答](../spec/screen.md#その他の回答), [状態データ](../spec/server.md#状態データ), [操作の約束](../spec/interaction.md#操作の約束).
Prerequisites: steps 1–4; submitted answer projection, not pending events that may already be acknowledged.
May change: scoped view-data/history/map/card/provisional/confirmation components, server view projection if needed, strings/styles, and browser tests.
Done when: confirmation, sent current round, past rounds, current-question map details, and decision-origin map details distinguish choice/custom/defer safely.
Submitted custom bodies are read-only plain text preserving multiline whitespace; no hidden choice/note/draft appears as a sent answer. Reconstruct from immutable submitted round state through the canonical projection, not today's unsaved buffer.
XSS-looking text is literal text, never HTML or explanation parts; both map contexts remain readable after reload, restart, next round, and event acknowledgment.
Shown by: test — scoped free-text/look-back/map/after-send browser suites with a real custom submission containing multiline and HTML-looking text, a next round/decision origin, and acknowledged events.
Inspect rendered text, absence of executable injected nodes, read-only controls, and payload exclusion; do not rely on a screenshot or textarea value alone.
Left to the implementer: display helper names and whitespace-preserving CSS within existing styles.
Stop and hand back if: history requires editable drafts or pending events as its sole source, or both map node contexts cannot receive the same answer interpretation.

## Step 6 — Agent contract and semantic handoff

Purpose: teach the skill to interpret the fixed wire variants and prepare final acceptance of custom-answer meaning, including the implemented confirmation display.
Specification: [問いの書き方](../spec/skill.md#問いの書き方), [送られた答えの扱い](../spec/skill.md#送られた答えの扱い), [待つ](../spec/server.md#待つ), [状態データ](../spec/server.md#状態データ), [その他の回答](../spec/screen.md#その他の回答), [まとめて送る](../spec/screen.md#まとめて送る), [操作の約束](../spec/interaction.md#操作の約束).
Prerequisites: steps 1–5; actual wire contract and passing compatibility examples; read skill-authoring rules before editing skill files.
May change: the scoped skill files and their existing example-driven round-trip tests only if examples require execution coverage.
Done when: event/result instructions explain method/free_text, inactive retained drafts, and the three variants; examples distinguish custom from defer and do not require LLM-authored Other options or consequences.
Instructions treat clear custom text as the answer, never force it onto the recommendation, and ask next-round clarification for conflicting or missing conditions rather than invent them.
Shown by: external — the existing upstream final acceptance review, presented once at cycle end after step 7 automation, compares these instructions and actual wait/result examples to the linked sections, including ambiguous/conflicting custom cases; record findings without asserting model compliance from wording tests.
Review the IMPLEMENTED confirmation display and its actual labels/meaning for choice+note, custom with inactive choice/note drafts retained, and custom+deferred with a retained custom draft.
Pass means the reviewer can distinguish the sent answer body, optional note, and deferral without assuming hidden drafts are sent: choice has its answer and separate note, custom has only its body, and custom+deferred shows deferral without presenting retained drafts as answers or notes.
The human also checks completeness of the added keyboard path on the implemented screen under 操作の約束; automated focus evidence remains in steps 3–5.
Prepare these cases without an intermediate human approval gate, additional workflow, live LLM evaluation, new tests, or exact-text assertions. Continue to step 7; capture the final review result or explicit pending status at handback.
Left to the implementer: explanatory prose/examples with unchanged semantics; `round.md` edits only when needed to remove misleading guidance.
Stop and hand back if: review exposes a specification conflict or requests behavioral changes outside this plan; pending human checks remain unverified, not self-approved, and are not product implementation failures requiring iteration.

## Step 7 — Final regression evidence and hand-back

Purpose: deliver the bounded change with observed automated results and explicit remaining human judgments.
Specification: [その他の回答](../spec/screen.md#その他の回答), [自由記述回答の保存と送信](../spec/server.md#自由記述回答の保存と送信), [送られた答えの扱い](../spec/skill.md#送られた答えの扱い), [操作の約束](../spec/interaction.md#操作の約束), and the remaining sections linked in the verification map's preceding specification inventory.
Prerequisites: steps 1–5 green; step 6 documents and final acceptance cases prepared. Human review is not a prerequisite for automation.
May change: only fixes within earlier steps' scopes; no additional deliverable trees.
Done when: complete existing browser regression and final repository gates pass; TDD phase outputs, compatibility/race results, changed paths, provisional-decision deviations, and remaining human checks are reported.
After automation, hand the implemented confirmation cases and other step 6 checks to the existing upstream human acceptance review once at cycle end; report its result or explicit pending status separately from automated implementation results.
Shown by: check — run `scripts/test-web.sh`, then `scripts/check.sh` once after scoped tests pass; inspect `git diff --check` and `git status --short` for bounded changes.
Left to the implementer: concise evidence organization; no authority to approve their own result.
Stop and hand back if: any required automated gate cannot run/pass or unrelated failures prevent a complete automated verdict; distinguish automated success from unverified human acceptance. Pending step 6 human checks do not block automation or trigger implementation iteration by themselves.
