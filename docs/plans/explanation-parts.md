# Plan: ordered explanation parts

This plan implements the approved specifications in commit `adc41f2` on branch `explanation-parts`.
It is a draft until the person approves and commits this file; implementation starts only after that approval.

## Goal

People can read ordered text, code, legacy diagrams, sequence diagrams and flowcharts in question backgrounds, option details and replies, including historical rounds, without executing supplied content or losing unreadable parts silently.

## Specification

Read these specifications and the exact sections linked by each step; this plan supplies implementation decisions, not a replacement specification.

- [説明パーツ](../spec/explanation-parts.md)
- [画面](../spec/screen.md)
- [起動と往復と記録](../spec/server.md)
- [操作と知らせ](../spec/interaction.md)
- [shoryo skill](../spec/skill.md)

Use [CONTEXT.md](../../CONTEXT.md) for terminology and [PROJECT.md](../../PROJECT.md) for project conventions and commands.
Do not follow specification indexes or recursively explore linked documents.

## Approach and why

### Boundaries and reuse record

The core owns the canonical explanation value and input validation, shared by round input, persisted questions and replies.
Add a focused module under `crates/shoryo-core/src/`, rather than defining another representation in the HTTP handler.
`Topic::check_round` must validate before mutation, and `Topic::reply` must validate before changing an ask.
Keep the server's existing clone, save, then publish transaction in `Shared::change`.
The CLI already inserts the ask identifier into reply JSON; no new command is needed.

The browser uses one ordered explanation component at all three locations.
Separate text/code/legacy presentation, sequence geometry, flow geometry, lossless measurement and geometry validation into focused modules.
Geometry computation takes explicit measurements and returns either geometry or a reason, without DOM mutation, network access or topic-state changes.
Only the measurement and SVG adapters touch the DOM.
`app.js` remains the owner of the single dialog selection and cross-region focus; viewer transform and clipboard feedback are local component state.
No new global keyboard dispatcher or persisted viewer setting is needed.

The following decisions walk the reuse ladder independently for each layer.
The delegated investigation's ecosystem search was bounded to about five minutes; its candidate assessment is not proof of impossibility.
If implementation reveals a dependency need, stop rather than add one silently.

| Layer | Decision and reason |
|---|---|
| Markdown parsing, editing, highlighting, execution | Omit: none is needed for the approved contract. |
| Serialization, transport, persistence | Adopt existing Serde/serde_json, CLI client and clone-save-publish helpers; they already carry structured state. |
| Explanation validation | Build a small pure contract-specific validator over typed values; standard collection/string operations cover checks but not this contract. |
| Text, code, scrolling, copying | Adopt platform text nodes, `pre`/`code`, CSS overflow and Clipboard API; no runtime dependency is needed. |
| Legacy diagram drawing | Adopt `web/diagram.js` unchanged; new validation wraps it without changing accepted valid grid drawing semantics. |
| Font measurement | Adopt the SVG measurement pattern in `web/measure.js`, not its truncating `fitLines`; build lossless wrapping with resolved-font cache keys. d3plus-text was considered but still needs a verified no-loss adapter. |
| Sequence and flow geometry | Build bounded fixed-rule geometry. Dagre's usual automatic layout is unnecessary; ELK documents fixed-layout support, but its suitability here is untested and an adapter remains necessary. Neither is claimed incapable of explicit layout. Mermaid is excluded by the specification. |
| Collision validation | Build shape-aware checks over bounded lists; RBush indexes rectangles but does not supply ownership exceptions, diamond containment or arrow/shape semantics. |
| Dialogs and gestures | Adopt `DialogLayer`, existing focus restoration and `PanZoom`; add keyboard movement only in the new viewer. |
| Tests and embedding | Adopt domain tests, round-trip `Env`/`Server`, Playwright helpers and rust-embed's automatic `web/` embedding. No new asset manifest or web build pipeline. |

Custom geometry and collision handling are substantial code to implement, maintain and security-review, not a few-line utility.
The approval judgment below explicitly accepts that cost instead of hiding it behind “no dependencies”.
Do not copy library source or examples: if reuse later involves copying, establish its license and required notices before writing it.

### Provisional answers and judgment points

All behavior left by the specifications to the plan is settled below.
These answers are approval targets together with the plan, not questions to pause on between implementation steps.
An overturn condition means what the person would request to change this draft before implementation; it is not permission to change an approved contract mid-cycle.

#### J1. Wire and stored representation

Use `Part[]` for `background` (omission defaults to `[]`) and for every `options[].description` (required, may be `[]`).
CLI reply input is `{ "parts": [...] }`; HTTP reply input adds `ask`.
Stored replied ask state is `{ "status": "replied", "parts": [...] }`.
Reply lists contain 1–32 parts; background and description lists contain 0–32.
Persist precisely the supplied order and optional layout fields; do not persist browser-derived sizes or default coordinates as though supplied.

Each part has an internally tagged `type`:

| Type | Fields besides `type` |
|---|---|
| `text` | `body` |
| `code` | `body`, `language`, `role`, optional `title` |
| `diagram` | `source`, `title`, `role` (legacy grid text) |
| `sequence` | `title`, `role`, `participants`, `events`, optional `layout`, optional `canvas` |
| `flow` | `title`, `role`, `nodes`, `edges`, optional `canvas` |

`role` is exactly `proposal`, `example` or `confirmed`.
`language: "pseudocode"` means pseudocode; any other nonblank bounded language string is accepted, including unknown languages.
Optional fields may be absent but not `null`; no fields besides those declared for their enclosing object are accepted, including nested objects.
Reject unknown tags, roles, port names, non-finite/fractional numeric values, bad references and duplicate IDs before any state change.
IDs of new participants/nodes use ASCII `[A-Za-z][A-Za-z0-9_-]{0,63}` and are unique within their diagram.
References must resolve within that diagram, not to a topic question or decision.
Reject lone JSON surrogate escapes rather than letting JavaScript and Rust count different values.

Required strings must be present.
Titles, languages, IDs, participant/node labels, message labels, frame conditions and supplied edge-label text must be nonblank under Unicode whitespace trimming; the original string is retained.
Text and code bodies may be empty or whitespace-only: content-field presence, not editorial usefulness, is the input boundary.
Multiline labels are permitted; preserve explicit newlines and all characters in lossless wrapping.
Strings discussing `<script>`, HTML or SVG remain legitimate inert content; reject a part type or field supplying executable markup, not the spelling of markup inside text.

Why: [対象と読み順](../spec/explanation-parts.md#対象と読み順), [パーツの内容と立場](../spec/explanation-parts.md#パーツの内容と立場) and [説明パーツの受理と保存](../spec/server.md#説明パーツの受理と保存) need one serializable contract with no second old reply field.
Overturn if: the person requests different identifiers, stricter empty-body treatment, or a different optional-field convention while retaining the approved meanings.

#### J2. Sequence structure and fixed geometry

Participants are `{id,label,x?}`, in left-to-right array order.
Events are one of:

- `{type:"message",from,to,label,kind,gap_after?}`, with `kind: "call" | "return"` and self messages allowed.
- `{type:"alt",branches:[{condition,messages:[Message]}],gap_after?}`, with 2–8 branches.
- `{type:"loop",condition,messages:[Message],gap_after?}`.

`Message` in a frame has exactly the same shape as a top-level message, including `type:"message"`.
Frames contain messages only; nested frames are rejected.
There are 1–12 participants and 1–96 top-level events, at most 12 frames and at most 96 messages in total, counting all branch/loop messages.
Each frame/branch contains at least one message.
The top-level ceiling is derived from the message ceiling: replacing a top-level message with a nonempty loop cannot increase the event count, and an alt consumes at least two messages for one event.
Calls are solid, returns dashed, and every message has an arrowhead pointing to `to`.
Use the same 1.5-unit stroke and 8-by-6 arrowhead as flow edges; return dashes are 6 units on, 4 off.

`layout` may contain only `participant_gap`, `event_gap`, `self_loop_width`.
Defaults are 220, 72 and 48 drawing units respectively.
The first default participant center is x=124; center i is `124 + i * participant_gap` (zero-based).
An explicit x replaces only that participant's default, and effective centers must be strictly increasing at input validation.
Centers are never moved to cure overlap.
Headers are width 160, with top y=24 and height `max(48, wrappedTextHeight + 24)`; all header bottoms share the largest header height.
Labels wrap in 136 units; lifelines run vertically from the header bottom to the final event's occupied bottom plus 24.

The first event row starts 48 units below the common header bottom.
For each message, place its label at the row's top, centered between endpoints, wrapping to `max(1, min(280, abs(to.x-from.x)-24))` units.
For self messages use `max(1,self_loop_width-16)` as wrap width and center the label over the loop.
The arrow's horizontal segment starts 12 units below the label's measured bottom.
A self loop goes right by `self_loop_width`, down 32, then back to its lifeline.
A normal message ends the occupied row at the horizontal arrow; a self message ends it at the lower arrow.
The next row starts at occupied bottom plus `gap_after`, otherwise `layout.event_gap`.
Honor every supplied gap, including the last message in a frame and each top-level event.

Compute a frame's horizontal bounds without using condition-band placement: first find `minContentX` and `maxContentX` from its participating lifelines, message labels and self-loop geometry, excluding condition bands.
Use all participant lifelines if necessary for a nonzero content width.
Set `left=minContentX-16`, then place each band's text at `left+16`.
Wrap each condition at 280 units and measure the literal `alt` or `loop` marker using the same font and measurement adapter; each band contains its marker on a separate 18-unit line, an 8-unit gap, then the condition's lossless lines.
Use that marker line in every alt branch's band as well as the loop band; the part title stays outside the SVG in the common wrapper and is not part of band geometry.
Let `maxConditionWidth` be the maximum actual measured width of any marker or condition line in these bands, not the nominal 280-unit wrap width.
Set `right=max(maxContentX+16,left+16+maxConditionWidth+16)`.
This is one-way extent calculation: extending right does not change left, wrapping or lifeline positions.
A band's text block has height `18+8+conditionTextHeight`, with 16-unit top and bottom inset, so `bandHeight=16+18+8+conditionTextHeight+16` at baseline; J5 scales text-derived measurements and padding with resolved font size.
The first band starts at frame top; later bands start 8 units below their divider, and each marker block starts 16 units below its band top.
Start its messages 16 units below the band's bottom.
An alt has one such band per branch, in branch order, with a horizontal divider before each later band; the divider precedes the band by 8 units.
Use 16 units between the preceding branch's occupied content bottom (including its last message gap) and the next divider.
Frame bottom is 16 below its last message's occupied bottom including that message's gap.
The following top-level event starts at frame bottom plus the frame's supplied/default gap.
These content-derived bounds enclose content; they do not reorder participants or events.

Why: [シーケンス図の表現範囲](../spec/explanation-parts.md#シーケンス図の表現範囲) and [明示配置と固定規則](../spec/explanation-parts.md#明示配置と固定規則) require a finite, predictable ordering without a general graph layouter.
The consistent 16-unit band inset and content-first bounds avoid circular measurement, while measuring the marker alongside the condition prevents an unaccounted label from escaping its frame.
The 18-unit marker line and 8-unit text gap separate frame kind from condition without changing the supplied condition; the 96-event ceiling describes a reachable maximum under the total-message bound.
Overturn if: the person needs another bounded spacing interface, different band/marker spacing or different default geometry/event limits; nested frames or new sequence semantics would require specification revision.

#### J3. Flow structure and fixed routes

Nodes are `{id,kind,label,position?,width?,height?}`.
`kind` is `start`, `end`, `process` or `decision`; `position` is the center `{x,y}`.
Start/end use a horizontal capsule, process a rectangle and decision a diamond.
Distinguish terminal kinds with localized visible markers inside the capsule: `Start`/`End` in English and 「開始」/「終了」 in Japanese, separately above the supplied node label even when the labels are equal.
These markers are derived presentation, not input fields; preserve the supplied label/body string and stored JSON unchanged.
Edges are `{from,to,from_port?,to_port?,via?,label?}`.
Ports are `north`, `east`, `south`, `west`, at actual shape extrema, not at a bounding-box corner.
`label` is `{text,position?}` with position its text-box center.
Every decision-origin edge requires a nonblank label, including a return edge.
Self edges are allowed; an omitted route that cannot form a visible valid self edge fails rendering rather than acquiring an invented route.

There are 1–32 nodes and 0–64 edges.
Default node center i is `(160,100+i*160)`; explicit centers replace only their own default.
Default width is 180 for capsules/rectangles, 220 for diamonds.
Padding is 12; line advance is 18 at the baseline font.
Wrap capsule/rectangle labels to `width-24`, and diamond labels to `width/2-24`.
For terminals, measure the marker with the same font at an 18-unit line advance, wrap it losslessly within `width-24`, and place an 8-unit gap between its measured line block and the supplied label block.
Center both blocks horizontally at the node center and center their combined block vertically: `contentTop=centerY-(markerTextHeight+8+textHeight)/2`; marker top is `contentTop`, supplied label top is `contentTop+markerTextHeight+8`.
Use 12-unit top/bottom padding around that combined block, yielding derived terminal height `max(64,markerTextHeight+8+textHeight+24)`; a one-line marker and one-line label give height 68 at baseline.
Derived process height is `max(64,textHeight+24)` and decision height is `max(96,2*(textHeight+24))`.
J5 scales marker line height, gap and padding with resolved font size; measurement and capsule containment include both marker and supplied-label bounding boxes, and their mutual overlap is a failure.
An explicit height is used unchanged and subsequently checked for real shape containment.
The derived height exceeding 640 is a rendering failure; do not cap it and hide text.

Omitted ports mean south to north.
Resolve the actual shape-boundary coordinates as `sourcePort` and `destinationPort`; the route is `sourcePort`, each `via` point in supplied order, then `destinationPort`.
If `via` is omitted, insert `(sourcePort.x,midY)`, `(destinationPort.x,midY)`, where `midY=(sourcePort.y+destinationPort.y)/2`, not the midpoint of node centers.
For a default-height decision at `(160,100)` followed by a process at `(160,260)`, south/north port y values are 148 and 228, so this midpoint is 188, not 180.
Remove only adjacent identical points for drawing; keep stored input unchanged.
Do not reroute around obstacles or treat a back edge differently: examples needing a return route supply `via` explicitly.
Explicit routes may be diagonal and are preserved; orthogonality is not an input requirement.
Arrowheads have length 8 and width 6; line stroke is 1.5 units.

For an edge label without position, choose the longest nonzero segment of the computed port-based route by Euclidean length, the first on ties; never use node-center segments for this choice.
Wrap its text in 160 units and use the actual wrapped box dimensions.
For a predominantly vertical segment (`abs(dy)>abs(dx)`), put the label box's left edge at `midpoint.x+8` and center its y coordinate at `midpoint.y`.
Otherwise (including equal-component diagonal segments), center the box's x coordinate at `midpoint.x` and place its bottom at `midpoint.y-8`.
Explicit label position fixes the center of that same wrapped box.
A label on a route with no nonzero segment fails rendering.
Do not shift labels to avoid other shapes.
At baseline, default process centers `(160,100)` and `(160,260)` with height 64 have port y values 132 and 228, midpoint 180, and equal vertical segments whose first midpoint is `(160,156)`.
A two-line 36-unit-high label therefore starts at x=168 and spans y=138–174, clear of source bottom 132, destination top 228 and the route at x=160.
This example is legal, not a promise that every long label fits: colliding longer labels still fail and may require an explicit label position or different input geometry.

Why: [フローチャートの表現範囲](../spec/explanation-parts.md#フローチャートの表現範囲) permits merges, returns and explicit routes; [明示配置と固定規則](../spec/explanation-parts.md#明示配置と固定規則) does not permit automatic repair.
Terminal markers distinguish start/end even with identical supplied labels while retaining the chosen capsule shape; the 18-unit line, 8-unit separation and 12-unit padding reuse the baseline typography and leave each text block independently measurable.
Using actual ports makes routes well-defined for unequal-height shapes; the fixed 8-unit side clearance for vertical labels avoids placing ordinary two-line labels over their source without introducing collision avoidance.
Overturn if: the person prefers different terminal marker wording/placement/spacing, ports, dimensions, orientation-based label placement/clearance or deterministic default routes; auto-repositioning would contradict the approved specification.

#### J4. Finite limits and structural rejection

Count strings in Unicode scalar values (`chars().count()` in Rust, iteration over code points in JavaScript), except the aggregate UTF-8 budget.
Check counts and dimensions before allocating geometry, loops or measurement probes.
Reject excess, never truncate.

| Input | Inclusive boundary |
|---|---|
| Parts per explanation | J1's 0–32 or 1–32 |
| Text body / code body / legacy source | 0–16,384 / 0–32,768 / 1–32,768 scalar values |
| Part title / language / new ID and reference | 1–120 / 1–64 / 1–64 |
| Every new node/participant/message/edge label and frame/branch condition | 1–512 |
| Sequence participants / total messages / frames / alt branches / top-level events | J2's bounds |
| Flow nodes / edges | J3's bounds |
| Supplied via points per edge / total per flow | 0–12 / 0–768 |
| Canvas width and height | Integers 64–8192 |
| Supplied x/y (node, participant, route and label position) | Integers 0–8192 |
| Supplied node width / height | Integers 80–640 / 32–640 |
| Every supplied sequence gap and self-loop width | Integers 16–512 |
| Legacy nodes / edges / rows / columns per row | 1–64 / 0–128 / 0–64 / 0–32 |
| Legacy node IDs and references / node and edge labels | 1–64 / 0–512 |
| Sum of explanation strings per submitted round or reply | 0–524,288 UTF-8 bytes |

The aggregate covers every string leaf inside every explanation, including tags, roles, languages, IDs/references and port names; count each occurrence, not unique strings, before mutation.
It excludes unrelated question text, options' short text/consequence, records and finished picture; do not introduce new question/choice-count restrictions for this work.
Tags, roles and ports are finite enums as above; there are no other explanation string fields.
All fixed defaults are checked against the same derived drawing extent ceiling, not exempted because no coordinate was supplied.
Boundary fixtures must be reachable complete inputs satisfying all other independently variable constraints.
Test the derived 96-event ceiling with 96 top-level messages; do not invent 108-event acceptance or a 97-event input satisfying the 96-total-message ceiling.
For a derived/redundant ceiling with no isolated overflow (also the 768-total-via ceiling under 64 edges times 12 points), prove the reachable maximum and refusal at the contributing independent bound instead of fabricating an impossible isolated case.
For example, test 97-message refusal inside one loop, 13-frame refusal with 13 one-message loops, and 13-via-point refusal on one edge; these cases isolate the respective independently variable limits.

Legacy source uses the current `id = label`, `id = ? label`, grid row and `from -> to : label` grammar.
Blank lines remain ignored; reject unrecognized nonblank lines, duplicate definitions, dangling edge/grid references and unequal row column counts.
Keep the parser's first-cell placement for repeated grid IDs, its omitted-node row and optional empty labels; do not apply new diagram collision rules to legacy source.
IDs use non-whitespace tokens compatible with the existing grammar, not the new-ID ASCII restriction.
No bounds are newly imposed on the unrelated finished-picture grammar.

Keep the existing HTTP status split: JSON/schema decoding failures use 422, typed domain refusals (including explanation-limit or reference validation) use 409, and the CLI reports either as exit 1.
Return an English explanation locating the question/option/part (or reply part), field and failed rule/limit.
Do not echo entire bodies in errors.
Save failure retains the existing HTTP 500 path and leaves prior accepted content intact.
On typed state loading, apply the same explanation validator to historical question lists and each replied ask before serving them; validate each round's question-explanation aggregate and each reply's aggregate separately, not the cumulative topic history against a submission budget.
Invalid persisted explanation values produce `CorruptState` without writing anything.
Browser geometry adapters also guard these finite bounds before measurement, so a malformed view cannot initiate unbounded geometry work.
`heard_from_agent` can change the view version on a rejected request; compare topic content, saved bytes and visible explanations, not an opaque version counter.

Why: [説明パーツの受理と保存](../spec/server.md#説明パーツの受理と保存) and [任せたこと](../spec/server.md#任せたこと) require finite limits with whole-request refusal.
Overturn if: representative small explanations cannot fit these values, or the person requests different finite ceilings/legacy syntax acceptance; change this plan before implementation, not silently in a validator.

#### J5. Measured wrapping, canvas and collision failure

For new diagrams, use browser font metrics after fonts are ready, not character-count widths.
At baseline, diagram text is 14px-equivalent in relative units (`0.875rem` with the project's baseline 16px root), line advance 18px-equivalent (`1.125rem`).
Scale text padding and text-derived height calculations with the resolved font size; fixed supplied coordinates remain unchanged.
Break explicit newline-delimited runs losslessly at the longest fitting prefix, preferring a whitespace boundary without dropping that whitespace; split a long unbroken run at code-point boundaries when necessary.
Retain blank lines and trailing spaces in the text data and SVG's whitespace-preserving text treatment.
Use no ellipsis or maximum line count.
If a single scalar cannot fit, report a rendering failure.
Cache metrics by text and resolved font family, size, weight, style, spacing and line-height, or invalidate for these conditions.
Recompute after fonts load, theme/font changes and viewport/zoom conditions that affect resolved metrics.
Do not reuse `fitLines`, whose trimming and ellipsis violate the new contract.

`canvas` is `{width,height}` with origin `(0,0)`.
When supplied, it is fixed.
When omitted, derive each dimension as `max(64,ceil(maxExtent+24))`, without translating any node.
Negative extents or dimensions above 8192 fail rendering; no shifting into the canvas or unbounded enlargement.
Bounds include strokes, arrowheads, frames, and actual text bounding boxes.
Card scrolling outside the visible viewport is not clipping; test containment against the full drawing canvas.

Build geometry with ownership metadata, then check actual SVG `getBBox()` text bounds and shape geometry before showing the diagram as valid.
Reject independent node/header box interior overlap, text-label overlap, label overlap with an unrelated shape, text outside its owning shape or canvas, and arrow segments entering unrelated node/header interiors.
Use a 0.5 drawing-unit numerical tolerance for containment/contact calculations, not a spacing repair.
Diamond/capsule checks use polygons/curves or equivalent exact containment/intersection calculations, not just AABBs (axis-aligned bounding boxes).
Test rectangle text corners against diamond/capsule containment.
Positive interior penetration beyond tolerance is overlap; boundary contact alone is legal.
Sibling labels still collide even when they share a frame.
An owning shape may contain its own text; connected endpoint shapes may meet arrows only at their declared ports, not admit a route through their interior.
Sequence lifelines and an enclosing frame are not independent filled node boxes; frame enclosure and the corresponding self loop are legal.
Line crossings alone are not errors.

On post-acceptance layout or measurement failure, display an inline reason and two-space pretty-printed JSON for that part as inert text.
Suppress that part's incomplete SVG and enlarge button, not its siblings/card.
The error must name the failing relation or extent (with affected IDs when available); an unavailable measurement is a failure, not approval of unmeasured geometry.
Revalidation may recover a previously failed part after display conditions change.

Why: [配置の失敗の検出](../spec/explanation-parts.md#配置の失敗の検出) and [描画失敗の表示](../spec/explanation-parts.md#描画失敗の表示) distinguish structural refusal from browser-measured failure.
Overturn if: the person requests a different bounded tolerance or canvas convention, or evidence shows this geometry cannot distinguish required legal/illegal pairs.
Custom checks carry maintenance cost; approval accepts that cost, and inability to detect the specified failures is a stop condition, not grounds to weaken the check.

#### J6. Copying, quotes, viewports and viewer controls

Use relative 14px-equivalent code text without wrapping or automatic width-fit scaling.
Code blocks have intrinsic short height and `max-height: min(20rem,50vh)` (320px at baseline); new diagram scroll regions use `max-height: min(22.5rem,50vh)` (360px at baseline), not a minimum height.
Keep width at natural diagram scale and use two-axis region scrolling.
New classes must not inherit the legacy SVG shrink-to-width or 12px text styles.
Keep legacy diagram drawing/gestures unchanged and add only its title/role wrapper.

Copy sends exactly the original code `body` to `navigator.clipboard.writeText`, including indentation, newlines and trailing whitespace; never include title, language or role.
Success feedback appears only after resolution.
Unavailable API or denial shows readable failure feedback and makes the existing readonly `pre` body selectable for manual copying (a “Select code” button selects it using Range/Selection).
No hidden `execCommand`, permission escalation or automatic retry.
Feedback belongs to that code component and remains keyboard reachable.
Place option detail controls outside the radio's label and outside a disabled fieldset; lock only answer controls, not read-only copy/enlarge controls.
Activating them must not change choice or stamp.

Follow-up quote generation reads the first part only: text uses the existing 40-code-point excerpt with ellipsis if longer; diagrams use title; code uses supplied title or language plus role.
Do not quote bodies/source JSON or use quote/title as identity.
Keep `follows` bound to the original ask ID, including equal-title replies.
Display roles with existing language switching: English “Proposal”, “Example”, “Confirmed current state”, Japanese 「案」「説明用の例」「確認済みの現状」.
Language metadata is shown literally except reserved `pseudocode`, displayed as “Pseudocode”/「疑似コード」.
Other fixed UI strings use existing translation keys with equivalent meaning, not literal-prose acceptance tests.

A new viewer opens at natural scale `k=1` with drawing origin at viewport `(24,24)`; reset returns exactly that transform.
Provide focusable Left/Right/Up/Down buttons and arrow keys on the focused viewer surface to move the drawing by 40 screen CSS pixels per press in the indicated direction.
Provide zoom in/out buttons and `+`/`-` keys with factors 1.2 and 1/1.2, zooming about viewer center and clamped to 0.2–3, plus a Reset button and `0` key.
Do not intercept these keys in editable fields or outside the viewer.
Reuse the existing modifier-wheel exponential factor `exp(-deltaY*0.0015)`, pointer-centered zoom, dialog single-finger movement and two-pointer midpoint pinch; ordinary wheel leaves scale unchanged and scrolls the page.
Gesture scale uses the same 0.2–3 clamps because initial/reset scale is 1 and there is no fit-below-minimum action in this viewer.
No new animation is needed, including under reduced motion.

Store the opener identity by round number, question ID, location (`background`, option index, or ask ID), and part index, never title.
Use that same stable identity for component/focus keys.
Open focuses the viewer surface; trap Tab within the dialog using the existing mechanism.
Close and Escape restore the surviving opener through top-level focus handling, even after a live redraw; if it disappeared, do not invent a focus destination.
Only one dialog may be open; retain existing explicit-dialog replacement policy and do not let clipboard failure replace a dialog.

Why: [コードの表示](../spec/explanation-parts.md#コードの表示), [新しい図の表示](../spec/explanation-parts.md#新しい図の表示), [返事への続きの引用](../spec/screen.md#返事への続きの引用) and [新しい説明図の拡大表示](../spec/interaction.md#新しい説明図の拡大表示) need concrete non-shrinking controls and safe clipboard failure behavior.
Overturn if: the person prefers a different initial viewer transform, bounded zoom/movement increments, metadata wording or clipboard fallback; code-body fidelity and keyboard/focus promises remain fixed.

#### J7. Old-format rejection and user guidance

Reject old string backgrounds/descriptions and old `{text,diagram}` reply input, rather than migrate them.
Persisted old questions/replied asks fail typed loading with the existing `CorruptState` error; never substitute an empty topic, save defaults, delete files or auto-migrate.
Old topics without an affected field can continue if they deserialize; do not blanket-reject unrelated valid records by adding a version gate.
Offline `result` continues its current raw read-only behavior, so old bytes can be extracted without startup or conversion; do not promise new-format UI compatibility for that output.
User guidance identifies the affected CLI fields and stored questions/replies, explains refusal without deletion, and advises retaining/exporting old data and using a separately named topic for new work.
Any requested individual migration is handed back for separately authorized work.
Add a focused breaking-format note to README and `CHANGELOG.md`'s Unreleased section, but do not bump a version, tag, push or release in this implementation.

Why: [旧形式への対応範囲](../spec/server.md#旧形式への対応範囲) expressly permits this break but not destruction of real data.
Overturn if: the person requests compatibility or a separately approved migration; preserving original bytes is not negotiable within this plan.

## Scope of change

Implementation may change explanation-related code in `crates/shoryo-core/`, the round/reply boundary and load validation in `crates/shoryo-server/`, necessary reply adapter code in `src/main.rs`, affected tests under `tests/round_trip/` and `web/tests/`, and presentation modules under `web/`.
It may add focused explanation modules and tests in those directories.
It may update `skills/shoryo/SKILL.md`, its references, README and the Unreleased changelog to describe this contract.
No specification, CI, dependency, canonical version or worktree-layout changes are part of implementation.
The drafting task creates only this plan; the preceding scope is for the later approved implementation, not permission to implement while drafting.

## Step order and prerequisites

1. Add canonical contract validation and convert affected synthetic fixtures together, so later layers have a stable schema.
2. Prove wire/persistence transactions and explicit old-data refusal.
3. Render ordered text/code/legacy parts and update quotes.
4. Implement fixed sequence geometry.
5. Implement fixed flow geometry.
6. Add measured containment/collision validation and isolated failure presentation for both new diagrams.
7. Complete scrolling, dialog gestures and keyboard/focus controls.
8. Update agent guidance, executable examples and user-facing break notice.
9. Run full integration and structural gates.
10. Inspect real browser/device behavior and explanation meaning with synthetic data.

Read routed skills before the work they govern (including Rust, TDD/testing, GUI, skill-authoring and release rules when writing their respective artifacts).
Use the pinned toolchain in `rust-toolchain.toml` and the locked dependencies.
Install browser test dependencies using `npm ci` and `npx playwright install chromium` if absent.
If toolchain, network or package-age policy prevents setup, report the actual blocker; do not add a permanent policy exception or substitute a different toolchain silently.

All running binaries and browser tests use existing isolated `Env`/`Server` and browser fixture helpers, temporary repository roots and temporary XDG data/config homes outside the repository.
Never start an implementation check against the person's actual topic directory or change real old bytes.
Do not create an alternate worktree as a side effect of following this plan; branch/work location belongs to the invoking workflow.

## Verification map

| Specification sections | Evidence steps |
|---|---|
| Explanation 対象と読み順, パーツの内容と立場 | 1–3, 9 |
| Explanation 表示だけの安全境界 | 1–3, 6, 9 |
| Explanation シーケンス図の表現範囲 | 1, 4, 6 |
| Explanation フローチャートの表現範囲 | 1, 5, 6 |
| Explanation 明示配置と固定規則, 任せたこと | 1, 4–6, 9 |
| Explanation 配置の失敗の検出, 描画失敗の表示 | 6, 10 |
| Explanation コードの表示, 新しい図の表示 | 3, 7, 10 |
| Explanation 作らないもの, 見送った案 | 9–10 (scoped diff and unchanged legacy features) |
| Server 説明パーツの受理と保存, 状態データ, 返す | 1–2, 9 |
| Server 旧形式への対応範囲, 記録の置き場所と寿命 | 2, 8–10 |
| Screen たたんだカード, 質問, 過去のラウンド, 返事への続きの引用 | 2–3, 7, 9 |
| Screen 図; Interaction 地図と完成図を動かす | 3, 7, 9–10 |
| Interaction 新しい説明図の拡大表示, 操作の約束 | 7, 9–10 |
| Skill 説明の表現を選ぶ, 説明の意味を確かめる | 8, 10 |

## Left to the implementer

Internal helper names, module splits, typed error names, test-file placement within the allowed directories, equivalent collision algorithms and CSS decoration may be selected without changing J1–J7.
Do not leave schema, optional defaults, numerical bounds, routing, failure/copy behavior, persistence or viewer behavior to implementation intuition.

## Stop conditions

Stop and hand back if meeting an approved success condition requires a changed specification, a new dependency, unbounded work, automatic repair/shrinking, a weaker collision oracle, real-data migration, or an unsafe/privileged/destructive operation.
Report unsupported environment checks as unverified, not passed.
No phase-specific human approval is required for safe isolated implementation; approval of the actual result is at the cycle's end.
No destructive or externally visible action is authorized here.

## Test command

Use project commands, from the repository root:

- Domain: `cargo test -p shoryo-core --locked` (append a behavior-name filter for RED).
- CLI/server round trip: `cargo test --locked --test round_trip` (append a behavior-name filter for RED).
- Targeted browser tests: `scripts/test-web.sh web/tests/explanation-parts.spec.js`; add `web/tests/explanation-geometry.spec.js` for the geometry cases.
- Existing regression browser suites: `scripts/test-web.sh`.
- Full Rust gates: `scripts/check.sh` (Rust gates only, not browser proof).

Every step marked `test` follows RED → GREEN → REFACTOR, one observable behavior at a time.
Run the new named test before production changes and retain its real failure output, then run the affected whole suite after GREEN and again after REFACTOR.
A missing symbol compile/import failure is valid RED; a fixture typo or an unrelated legacy compilation failure is not evidence of the behavior.
When no refactor is warranted, record what was examined and why, with the still-green suite output.
Fixture conversion can precede a new test run when needed to express the new input; do not change an assertion to make a rejection disappear.
No tests pin prose counts, exact skill wording or private module layout.

## Out of scope

Release/version/tag/push, real-data conversion, deletion of this plan, new CLI commands, new runtime dependencies, Markdown extraction, syntax highlighting, editing/execution, automatic graph layout, Mermaid, comparison/diff parts, new map/finished-picture geometry, dedicated nonvisual diagram listings and required alternative prose.

## Step 1: Canonical parts and input validation

Purpose: establish one bounded serializable explanation contract before adapters depend on it.
Specification: [対象と読み順](../spec/explanation-parts.md#対象と読み順), [パーツの内容と立場](../spec/explanation-parts.md#パーツの内容と立場), [表示だけの安全境界](../spec/explanation-parts.md#表示だけの安全境界), [シーケンス図の表現範囲](../spec/explanation-parts.md#シーケンス図の表現範囲), [フローチャートの表現範囲](../spec/explanation-parts.md#フローチャートの表現範囲), [説明パーツの受理と保存](../spec/server.md#説明パーツの受理と保存), [任せたこと](../spec/server.md#任せたこと).
Prerequisites: committed approval of this plan, pinned Rust available; J1–J4 are the contract.
May change: explanation-related modules and domain tests in `crates/shoryo-core/`; affected synthetic fixtures/constructors in `crates/shoryo-core/tests/domain/`, `tests/round_trip/` and `web/tests/rounds.js`, `web/tests/fixtures.js`; minimal server reply adapter adaptation needed to compile the workspace.
Done when: input and stored state share the new types; all nested unknown fields and invalid kinds/roles/references/nesting are rejected; each independently variable min/max count, string/aggregate budget and integer bound has a feasible acceptance/refusal pair, while derived ceilings are covered as J4 declares; failed round/reply validation leaves domain content unchanged.
Shown by: test — add behavior tests `ordered_parts_preserve_all_metadata`, `invalid_part_refuses_entire_round`, `invalid_reply_leaves_ask_waiting`, `explanation_limits_accept_boundary_and_refuse_excess`, `sequence_frames_reject_nesting`, `decision_branches_require_conditions`; use parameterized cases per J4 boundary and missing/unknown fields. Drive `check_round` and `Topic::reply`, not just helper return values; run the domain command through RED/GREEN/REFACTOR. Convert old string fixtures to explicit text parts rather than accepting old format to rescue tests.
Left to the implementer: focused module names and typed refusal names; none of J1–J4.
Stop and hand back if: the canonical value requires a browser/infrastructure dependency, or the fixture conversion reveals a real persisted-data target.

## Step 2: Transactional transport, persistence and old-data refusal

Purpose: connect the new contract without weakening state-save transactions or touching old real records.
Specification: [説明パーツの受理と保存](../spec/server.md#説明パーツの受理と保存), [旧形式への対応範囲](../spec/server.md#旧形式への対応範囲), [状態データ](../spec/server.md#状態データ), [返す](../spec/server.md#返す), [記録の置き場所と寿命](../spec/server.md#記録の置き場所と寿命), [返事への続きの引用](../spec/screen.md#返事への続きの引用).
Prerequisites: Step 1; isolated round-trip `Env` and a new synthetic topic.
May change: `crates/shoryo-server/src/app.rs`, `store.rs` and load/error plumbing if needed, `src/main.rs` only where reply parsing requires it; `tests/round_trip/` and affected domain tests.
Done when: CLI and direct HTTP round/reply inputs enforce the same bounds; save refusal and invalid requests preserve prior saved bytes and topic content; new parts survive restart, result output and late replies to submitted rounds; equal-title follow-ups retain correct ask IDs; old synthetic state causes explicit startup refusal with unchanged bytes and raw offline result still readable.
Shown by: test — add `parts_survive_restart_and_result`, `invalid_round_or_reply_keeps_saved_bytes`, `refused_part_save_keeps_previous_content`, `late_parts_reply_stays_in_original_round`, `same_title_follow_up_targets_original_ask`, `old_explanation_state_refuses_start_without_overwrite`, `old_reply_input_is_refused`; run `cargo test --locked --test round_trip` through RED/GREEN/REFACTOR. Cover J4 boundary refusal through real requests, using helpers rather than one server per case where practical. Keep existing refusal tests; compare content/bytes rather than view version.
Left to the implementer: test helper extraction and whether parsing error context is added in the adapter or canonical decoder without duplicating validation.
Stop and hand back if: refusal would create/save an empty topic, successful tests need actual user data, or atomic publication cannot be retained.

## Step 3: Ordered text, code, legacy wrappers and follow-up quotes

Purpose: make the common explanation reader work in every target location before adding new geometry.
Specification: [対象と読み順](../spec/explanation-parts.md#対象と読み順), [パーツの内容と立場](../spec/explanation-parts.md#パーツの内容と立場), [表示だけの安全境界](../spec/explanation-parts.md#表示だけの安全境界), [コードの表示](../spec/explanation-parts.md#コードの表示), [たたんだカード](../spec/screen.md#たたんだカード), [質問](../spec/screen.md#質問), [返事への続きの引用](../spec/screen.md#返事への続きの引用), [過去のラウンド](../spec/screen.md#過去のラウンド), [図](../spec/screen.md#図).
Prerequisites: Steps 1–2; browser dependencies and synthetic mixed-part fixtures.
May change: `web/view-data.js`, common/new explanation components, `web/components/question-parts.js`, `thread.js`, affected card/provisional callers, `web/strings.js`, `web/style.css`, affected tests plus `web/tests/explanation-parts.spec.js` and helpers.
Done when: text/code/text and legacy-first lists preserve order in backgrounds, option details, replies and past rounds; folded cards do not leak details; metadata and all roles show; copying preserves exact body and denial/unavailable fallback works; quote content distinguishes first-part kinds without source dumps; option controls remain usable after submission/history and never select radios or remove stamps; supplied markup is inert with no part-derived network requests.
Shown by: test — add `parts_read_in_order_in_each_location_and_history`, `roles_and_languages_are_visible`, `copy_preserves_code_body_and_reports_denial`, `option_detail_controls_do_not_change_answers`, `first_part_identifies_follow_up_without_source_dump`, `part_content_does_not_execute_or_fetch`; run targeted Playwright RED/GREEN/REFACTOR. Use real DOM and Clipboard API where supported; simulate only the unavailable/permission-denied browser boundary. Observe browser requests and script execution, not forbidden-word searches.
Left to the implementer: equivalent component split, visual decoration and translation phrasing consistent with J6.
Stop and hand back if: details cannot be made independent of disabled radio labels without changing answer behavior, or code rendering requires executing supplied markup.

## Step 4: Fixed sequence geometry

Purpose: draw supported sequence events deterministically with measured, lossless labels.
Specification: [シーケンス図の表現範囲](../spec/explanation-parts.md#シーケンス図の表現範囲), [明示配置と固定規則](../spec/explanation-parts.md#明示配置と固定規則), [任せたこと](../spec/explanation-parts.md#任せたこと).
Prerequisites: Steps 1–3; J2 and J5 geometry rules; real browser font measurements.
May change: new sequence geometry/SVG and lossless text-measurement modules under `web/`, explanation component integration, new scoped CSS, `web/tests/explanation-geometry.spec.js` and helpers.
Done when: participants, ordinary/self/return messages, alt branches and loops use J2's exact rules; overrides and multiline/long-unbroken labels are retained; viewport width does not reorder or reposition; no truncation is introduced in new diagrams.
Shown by: test — add `sequence_order_and_explicit_spacing_are_preserved`, `sequence_returns_self_calls_and_frames_are_distinct`, `sequence_defaults_are_stable_across_viewport_widths`, `sequence_labels_wrap_without_loss`; run targeted geometry Playwright tests through RED/GREEN/REFACTOR. Test actual submitted data and rendered positions under fixed display conditions, including default and explicit spacing, not source-code constants alone. Include a long wrapped condition and a self call in the same legal frame, asserting the 16-unit inset and content-first horizontal bounds with measured marker/condition extents.
Left to the implementer: pure geometry module boundaries and SVG helper names.
Stop and hand back if: legal frames/self calls require nesting or automatic rearrangement, or browser metrics cannot be obtained reliably in the supported test environment.

## Step 5: Fixed flow geometry and routes

Purpose: draw fixed flow shapes, conditions, merges and returns without auto-layout.
Specification: [フローチャートの表現範囲](../spec/explanation-parts.md#フローチャートの表現範囲), [明示配置と固定規則](../spec/explanation-parts.md#明示配置と固定規則), [任せたこと](../spec/explanation-parts.md#任せたこと).
Prerequisites: Steps 1–4; J3 rules and shared lossless measurement.
May change: new flow geometry/SVG modules under `web/`, explanation integration/CSS, `web/tests/explanation-geometry.spec.js` and helpers.
Done when: all four node kinds are visually distinguishable (start/end retain capsules with distinct markers); explicit positions, ports, route points and label centers remain unchanged; omitted routes/labels follow J3; decision conditions, merge and explicit back edges remain readable and connected; short default diagrams fit without moving nodes.
Shown by: test — add `flow_shapes_and_conditions_preserve_connections`, `flow_explicit_positions_ports_and_routes_are_preserved`, `flow_defaults_and_label_ties_are_deterministic`, `flow_return_and_merge_keep_supplied_routes`; use real flow input with output geometry and endpoint assertions under multiple widths, through targeted Playwright RED/GREEN/REFACTOR. In the shape/kind test include equal supplied labels on start/end, both localized markers, preserved original labels and containment of marker plus label. In the existing defaults/ties test include the two-process two-line label case (ports 132→228, first midpoint y=156, label left x=168 and y=138–174 at baseline) and the unequal-height decision/process case (ports 148→228, route midpoint y=188), deriving label segment choice from that computed route.
Left to the implementer: equivalent shape/intersection algorithms and internal decomposition.
Stop and hand back if: a desired return/merge example needs obstacle avoidance instead of an explicit route, or a new runtime layouter dependency appears necessary.

## Step 6: Measured layout validation and per-part failure

Purpose: distinguish readable diagrams from detected layout failures without repairing supplied geometry.
Specification: [配置の失敗の検出](../spec/explanation-parts.md#配置の失敗の検出), [描画失敗の表示](../spec/explanation-parts.md#描画失敗の表示), [表示だけの安全境界](../spec/explanation-parts.md#表示だけの安全境界), [説明パーツの受理と保存](../spec/server.md#説明パーツの受理と保存).
Prerequisites: Steps 4–5; J5 ownership, containment and canvas conventions.
May change: new diagram geometry validators and measurement adapters, explanation component/error presentation, scoped CSS, geometry and explanation Playwright tests.
Done when: every specified overlap/clip/arrow-through-shape failure has a legal counterpart accepted; actual long-text bounds work in both themes; supplied canvas clipping, negative extent and derived size overflow fail only the affected part; errors show reason and original structured data; font/display changes trigger valid remeasurement without shrinking/moving nodes.
Shown by: test — add `independent_boxes_and_labels_fail_on_overlap`, `owned_labels_ports_self_calls_and_frame_enclosure_are_legal`, `arrows_entering_unrelated_shapes_fail_but_crossings_are_legal`, `diamond_and_capsule_containment_use_actual_shapes`, `canvas_and_text_clipping_fail_only_the_affected_part`, `font_and_theme_changes_revalidate_lossless_labels`; create paired examples through submitted data, real `getBBox` metrics and visible failure output. Run targeted Playwright RED/GREEN/REFACTOR; include source text containing executable-looking strings in pretty JSON, adjacent readable parts and other operable cards. The legal pairs include Step 4's long condition/self call frame and Step 5's equal-label terminal markers and two-line vertical-edge label; a deliberately undersized terminal must report marker/label containment failure rather than dropping either text block.
Left to the implementer: equivalent bounded math implementation and error wording that identifies the observed relation.
Stop and hand back if: testing requires only mocked text sizes for a required real-font guarantee, legal containment must be rejected to catch an illegal pair, or checks need unbounded retry/auto-repair.

## Step 7: Scroll regions and accessible enlargement

Purpose: finish non-shrinking reading regions and the separate keyboard-operated viewer.
Specification: [コードの表示](../spec/explanation-parts.md#コードの表示), [新しい図の表示](../spec/explanation-parts.md#新しい図の表示), [新しい説明図の拡大表示](../spec/interaction.md#新しい説明図の拡大表示), [地図と完成図を動かす](../spec/interaction.md#地図と完成図を動かす), [操作の約束](../spec/interaction.md#操作の約束).
Prerequisites: Steps 3–6; valid wide/short diagrams and code; existing dialog/focus mechanisms.
May change: explanation viewer components, `web/app.js` dialog event/view-data plumbing, `web/components/dialog.js` integration, `web/pan-zoom.js` only for tested reuse needs without altering existing behavior, strings/CSS and browser tests.
Done when: code and diagrams use intrinsic short heights and both declared caps; wide content scrolls at natural text size; new diagrams alone have enlargement; all J6 keyboard/button transforms, gestures, reset and closes work; focus survives redraw and returns to the correct equal-title opener; map, finished picture and legacy parts keep existing gesture promises.
Shown by: test — add `code_and_diagram_regions_obey_both_height_caps`, `wide_parts_scroll_without_text_shrinking`, `viewer_keyboard_pan_zoom_reset_and_close_restore_opener`, `viewer_focus_survives_live_redraw_with_equal_titles`, `viewer_modifier_wheel_zooms_but_plain_wheel_does_not`, `legacy_parts_keep_noninteractive_diagrams`; run targeted Playwright then the existing interaction/diagram/ask suites through RED/GREEN/REFACTOR. Inspect dimensions, overflow, computed text size, real focused element and prevented events, not merely button presence.
Left to the implementer: helper extraction and accessible fixed-control wording; behavior and numerical transforms are J6.
Stop and hand back if: adding keyboard movement changes the map/finished-picture contract, or focus identity cannot survive a legitimate live update.

## Step 8: Agent references, executable examples and break notice

Purpose: teach producers the new contract and the limits of successful rendering without treating examples as new agreement.
Specification: [説明の表現を選ぶ](../spec/skill.md#説明の表現を選ぶ), [説明の意味を確かめる](../spec/skill.md#説明の意味を確かめる), [旧形式への対応範囲](../spec/server.md#旧形式への対応範囲), [任せたこと](../spec/explanation-parts.md#任せたこと).
Prerequisites: Steps 1–7; J1–J7 verified; read skill-authoring rules before editing the skill.
May change: `skills/shoryo/SKILL.md`, `skills/shoryo/references/round.md`, `events-and-replies.md`, `commands.md` where stale, an explanation reference under that directory if useful; `tests/round_trip/main.rs` example scanner, README, `CHANGELOG.md` Unreleased only.
Done when: references describe schema/defaults/limits once with links instead of conflicting copies; examples cover text-only, mixed ordering, code, supported sequence/flow and legacy wrappers; selecting/avoiding representations and checking meaning/role/unknown details are explained; failure handling distinguishes structural refusal, browser failure and unseen rendering; user notice identifies the exact break and non-destructive alternatives; every executable input example is accepted independently.
Shown by: test — extend the existing `skill_examples_are_accepted` behavior test to create a fresh ask for each `json reply` and legacy diagram example, and wrap `text diagram` examples in the new legacy part. Ensure a new explanation reference is included in scanner inputs if added. Run `cargo test --locked --test round_trip skill_examples_are_accepted` with RED/GREEN/REFACTOR for scanner behavior, then the whole round-trip suite. Documentation wording/meaning is assessed in Step 10, not asserted by prose-count or exact-string tests.
Left to the implementer: English phrasing, example domain with clearly invented content, and whether one focused reference reduces duplication.
Stop and hand back if: examples require unspecified fields or new supported features, guidance promises meaning correctness from rendering alone, or third-party text/source would be copied without a permissive license and required notices.

## Step 9: Full gates and scoped regression audit

Purpose: verify integrated behavior and inspect that unrelated display/storage features remain outside the change.
Specification: [作らないもの](../spec/explanation-parts.md#作らないもの), [見送った案](../spec/explanation-parts.md#見送った案), [対象と読み順](../spec/explanation-parts.md#対象と読み順), [説明パーツの受理と保存](../spec/server.md#説明パーツの受理と保存), [過去のラウンド](../spec/screen.md#過去のラウンド), [操作の約束](../spec/interaction.md#操作の約束).
Prerequisites: Steps 1–8 complete, isolated tests; no real topic server.
May change: no production changes in this step; return regressions to the owning prior step and its RED/GREEN/REFACTOR cycle.
Done when: all project gates pass; new embedded modules work with the single built binary; no dependency, spec, version, old-data deletion, map/finished-picture geometry or executable-part changes appear in the scoped diff; no confidential content/local machine paths are introduced.
Shown by: check — run in order `cargo fmt --all --check`, `scripts/check-domain-purity.sh`, `cargo test --workspace --locked`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `scripts/check.sh`, `scripts/test-web.sh`, `git diff --check`, `git diff --stat`, then inspect the scoped diff and search added content for credentials and machine-specific paths. Preserve each exit status/output and explain any environment blocker; `scripts/check.sh` is not a substitute for the separate browser run.
Left to the implementer: none; rerun affected/full gates after prior-step fixes.
Stop and hand back if: gates cannot run under supported policy/toolchain, a touched unrelated behavior needs a redesign, or isolation leaks writes into actual topic data.

## Step 10: Real-device and explanation-meaning inspection

Purpose: check browser/device behavior and editorial meaning that automated syntax/layout tests cannot establish.
Specification: [配置の失敗の検出](../spec/explanation-parts.md#配置の失敗の検出), [コードの表示](../spec/explanation-parts.md#コードの表示), [新しい図の表示](../spec/explanation-parts.md#新しい図の表示), [新しい説明図の拡大表示](../spec/interaction.md#新しい説明図の拡大表示), [操作の約束](../spec/interaction.md#操作の約束), [説明の表現を選ぶ](../spec/skill.md#説明の表現を選ぶ), [説明の意味を確かめる](../spec/skill.md#説明の意味を確かめる), [旧形式への対応範囲](../spec/server.md#旧形式への対応範囲).
Prerequisites: Step 9; synthetic topic in temporary data/config roots, supported desktop browser and an actual touch/trackpad device when available.
May change: none here; feed findings back to the appropriate prior step; retain inspection notes outside tracked files.
Done when: a person can read long/short parts in both themes and narrow screens with browser text enlargement; enlarged viewers preserve browser page zoom behavior, touch movement and pinch; all controls can be traversed without a mouse; paired legal/failed diagrams make sense visually; reference examples use the minimum appropriate representation and do not turn proposed/example details into separate agreements; break notice clearly preserves old records. Missing device checks are explicitly unverified.
Shown by: external — inspect the built binary's screen with isolated data at baseline and 200% text enlargement, narrow/wide widths and both themes; try code selection/copy and blocked clipboard, actual Ctrl/⌘ wheel with page-zoom observation, touch/trackpad pinch and one-finger dialog movement, keyboard open/move/zoom/reset/close after a reply redraw. Compare invented questions/agreements with the skill's choose/avoid examples and repair instructions; pass means no lost content, correct ownership/focus and no unsupported semantic claims. This is result inspection, not an intermediate signoff or a release/migration operation.
Left to the implementer: safe inspection order and synthetic topic labels; platform gaps must be reported.
Stop and hand back if: the only available inspection target is real old topic data, device observations contradict a required guarantee, or correction would change an approved specification.
