# Explanation parts

Read this before writing question backgrounds, option descriptions or replies.
These fields now contain ordered parts, not strings or a separate reply diagram.
The order is the reading order; nothing is extracted from Markdown.

## Choose the smallest useful representation

- Use text for a definition, premise or short reason. A text-only list is normal.
- Use code when literal syntax, indentation or a small interface is what the person needs to see.
  Code is readonly and copyable, never executed; syntax highlighting is not provided.
- Use a sequence for who communicates with whom, in what order, with call/return/self messages,
  alternative conditions or repetition. Do not use it for free-form graph nesting.
- Use a flow for conditioned branches, merges and return paths. Supply explicit routes when a
  return would otherwise run through a node. Do not use it as an automatic graph layouter.
- Use the existing grid diagram for a small arrangement of related things. Its grammar is in
  `diagram.md`; finished pictures continue to use that grammar directly.

Avoid a diagram when one sentence answers the question, and avoid code when exact syntax is
unknown or irrelevant. Do not invent APIs, message order, conditions or current-state details
to fill a drawing. State what is unknown in text and ask about it if it affects the decision.

Before sending, compare every label, direction, branch condition and code detail with the
question and existing records. `proposal` is an option under discussion, `example` illustrates
a point, and `confirmed` describes verified current state. An example or proposal is not an
additional agreement. A successful parse or drawing does not verify this meaning.

## Common schema

Background and every option description are arrays of 0–32 parts. A reply is
`{"parts":[...]}` with 1–32 parts; the CLI argument names the ask ID, including late replies.
Optional fields are omitted, not `null`. Unknown fields at any depth are refused.

| `type` | Required fields | Optional fields |
|---|---|---|
| `text` | `body` | none |
| `code` | `body`, `language`, `role` | `title` |
| `diagram` | `source`, `title`, `role` | none |
| `sequence` | `title`, `role`, `participants`, `events` | `layout`, `canvas` |
| `flow` | `title`, `role`, `nodes`, `edges` | `canvas` |

`role` is `proposal`, `example` or `confirmed`. Language is shown literally except reserved
`pseudocode`, localized as Pseudocode. Body/source strings are inert text, not HTML, SVG or URLs
to load. Text/code bodies may be empty; titles, language and new labels/conditions must be nonblank.
Follow-up quotes use the first part only: a text excerpt, a diagram title, or code title (otherwise
language plus role). Ask identity is always its ID, never that quote or a shared title.

## Sequence

Participants: `{id,label,x?}` in left-to-right order. Default centers are `124+i*participant_gap`;
supplied centers replace only their own defaults and must remain strictly increasing.
Events are:

- `{type:"message",from,to,label,kind,gap_after?}`, `kind` is `call` or `return`.
- `{type:"alt",branches:[{condition,messages:[Message]}],gap_after?}`, with at least two branches.
- `{type:"loop",condition,messages:[Message],gap_after?}`.

Every frame message has the same tagged message shape. Frames cannot nest. Calls are solid,
returns dashed; `to` is the arrow target, including self calls. `layout` has only
`participant_gap`, `event_gap`, `self_loop_width`, defaulting to 220, 72, 48 drawing units.
Headers are 160 wide, labels wrap within 136. The first message starts 48 below the common header
bottom. Message labels wrap losslessly, then arrows start 12 below their text; self calls go
right by the self-loop width and down 32. The next row starts after the occupied arrow bottom
plus the supplied/default gap. Frame bands retain every condition, marker and message gap;
content determines the enclosing frame, never participant movement.

## Flow

Nodes: `{id,kind,label,position?,width?,height?}`, where kind is `start`, `end`, `process` or
`decision`. Position `{x,y}` is the center. Defaults are `(160,100+i*160)`, width 180 (220 for
decisions); heights derive from wrapped text with minimum 64 (96 for decisions). Terminals
are capsules with separate localized Start/End markers above the supplied label; processes
are rectangles and decisions diamonds. Supplied dimensions/centers are not repaired.

Edges: `{from,to,from_port?,to_port?,via?,label?}`. Ports are `north`, `east`, `south`, `west` at
actual shape extrema, default south→north. Every decision-origin edge needs a nonblank label,
including returns. Label: `{text,position?}`, where position is the wrapped label-box center.
`via` is an ordered array of `{x,y}`; explicit diagonal routes are allowed. Omitted routes use
the midpoint of actual port y values; only adjacent duplicate points are removed for drawing.
An empty supplied `via` means a direct segment, not the omitted default.

Without an explicit label position, the longest nonzero port-route segment is used, first on
ties. Vertical segments put the box 8 to the right, centered vertically; other segments put it
8 above, centered horizontally. Labels wrap within 160 drawing units. No obstacle avoidance,
automatic repositioning or special return-edge repair is performed.

## Finite limits

All boundaries are inclusive. String lengths use Unicode scalar values; aggregate size uses
UTF-8 bytes of every explanation string leaf, including repeated tags, roles, IDs and ports.
Numbers below are integers. New IDs start with an ASCII letter, followed by letters/digits/`_`/`-`.

| Field or structure | Limit |
|---|---|
| Text body / code body / legacy source | 0–16,384 / 0–32,768 / 1–32,768 characters |
| Title / language / new IDs and references | 1–120 / 1–64 / 1–64 characters |
| New labels and conditions | 1–512 characters |
| Sequence participants / total messages / frames / top-level events | 1–12 / 1–96 / 0–12 / 1–96 |
| Alt branches / messages per branch or loop | 2–8 / at least 1 (within total message limit) |
| Flow nodes / edges | 1–32 / 0–64 |
| Via points per edge / per flow | 0–12 / 0–768 |
| Supplied canvas width/height | 64–8192 |
| Supplied x/y coordinates | 0–8192 |
| Supplied node width/height | 80–640 / 32–640 |
| Every sequence gap and self-loop width | 16–512 |
| Legacy nodes / edges / rows / columns per row | 1–64 / 0–128 / 0–64 / 0–32 |
| Legacy IDs/references / node and edge labels | 1–64 / 0–512 characters |
| Explanation strings per round or reply | 0–524,288 bytes |

The round budget includes all question backgrounds and option descriptions together, not
question short text, consequences, records or finished picture. Each reply has its own budget.
Legacy IDs are non-whitespace tokens under the existing grammar, not the new ASCII-ID rule.
Repeated grid cells retain first-cell placement; unequal rows, duplicates, dangling references
and unrecognized nonblank lines are refused. Finished-picture limits are unchanged.

## Rendering and repair

New diagram text is measured with actual browser fonts and wraps without ellipsis or a line
ceiling. Supplied `canvas:{width,height}` fixes the drawing area at origin `(0,0)`. If omitted,
the area derives from content plus 24, bounded at 8192 per dimension; negative extents fail.
Fonts/text enlargement may make accepted geometry fail. Nodes, labels, arrows, strokes and
arrowheads must fit; shape/text overlap, text outside its own shape, and routes entering node
interiors fail. Contact alone and line crossings are legal. Positions never move to fix a failure.

- Structural refusal: the CLI exits 1, explaining the field/rule. The whole request is refused;
  correct the input before resending. Schema failures are HTTP 422; domain refusals are 409.
  When a limit is exceeded, split the explanation into smaller explanations that fit the
  per-part, part-count and whole-request limits. Never silently omit excess text, conditions,
  branches or routes. Splitting a body across parts does not increase the round/reply byte budget.
- Browser layout failure: only that part shows its reason and original pretty JSON, with no
  incomplete diagram or enlarge control. Check IDs, labels, positions, sizes, gaps and canvas;
  revise the explanation, not an already sent question. Keep the intended meaning unchanged.
  Depending on the cause, use smaller diagram parts or a simpler text/code explanation instead
  of only adjusting geometry. Preserve the decision-relevant conditions, order, connections and
  role; do not remove content merely to make the layout pass.
- Unseen rendering: report it as unverified. Input acceptance is not visual or semantic proof.

New diagrams scroll at natural text size and can open separately with movement, zoom and reset
controls. Existing grid explanation diagrams remain noninteractive. None executes supplied content.

## Executable invented examples

Text alone is enough for a definition:

```json reply
{"parts":[{"type":"text","body":"A retry means attempting the same request again; it does not create a new agreement."}]}
```

Keep context before and after literal syntax; this pseudocode is only an illustration:

```json reply
{"parts":[{"type":"text","body":"This example shows a possible retry boundary, not a confirmed implementation."},{"type":"code","title":"Retry sketch","language":"pseudocode","role":"example","body":"  if temporary_failure:\n    retry_once()  \n"},{"type":"text","body":"Whether retrying is appropriate still depends on the request's effects."}]}
```

Use a sequence when the question is about communication order:

```json reply
{"parts":[{"type":"sequence","title":"Illustrated read","role":"example","participants":[{"id":"reader","label":"Reader"},{"id":"store","label":"Store"}],"events":[{"type":"message","from":"reader","to":"store","label":"Read record","kind":"call"},{"type":"message","from":"store","to":"reader","label":"Return record","kind":"return"}]}]}
```

Use a flow when the question is about a condition and its next action; the Yes label is the
branch condition, not a new decision made by the drawing:

```json reply
{"parts":[{"type":"flow","title":"Proposed ready path","role":"proposal","nodes":[{"id":"ready","kind":"decision","label":"Ready?"},{"id":"work","kind":"process","label":"Do the work"},{"id":"done","kind":"end","label":"Done"}],"edges":[{"from":"ready","to":"work","label":{"text":"Yes"}},{"from":"work","to":"done"}]}]}
```
