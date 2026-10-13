# Events, replies and the result

Read this when handling what `shoryo wait` prints, writing a reply for `shoryo reply`, or reading
the output of `shoryo result`.

## Events

`shoryo wait` prints `{"events": [...]}`. Every event has an `id` and a `kind`; acknowledge the
ids with `--ack` on the next `wait`. An event not acknowledged is printed again, so a crash
between receiving and handling loses nothing.

| `kind` | Fields | Meaning |
|---|---|---|
| `ask` | `ask`, `round`, `question`, `text`, `follows` | The person asked about a question. Reply with `shoryo reply <topic> <ask>`. `follows` is the ask whose reply this one continues, or `null`. |
| `submitted` | `round`, `answers` | The person sent the whole round. Each answer: `question`, `class` (as sent), `choice` (option index from 0, or `null` for custom/deferral), optional `free_text` (custom body), `note`, `deferred`, `stamp` (判子: `"person"` when the person stamped it, `"pre_approved"` for a choice left pre-approved, 代決). Interpret these together as below. `answers` is empty for a result round. |
| `review_requested` | `decision` | The person wants this decision asked again (見直す). Arrives only while the current round is open, never between sending it and the next round (or ending after proceeding with the result). |
| `review_stopped` | `decision` | The person withdrew that request. May arrive after the next round, until you send that decision's review conclusion. |
| `class_swapped` | `question`, `class` | The person moved a question between `human` and `provisional`. |

An example of what `wait` prints after an ask, a swap, a review request withdrawn, another
review request, and a send:

```json output
{"events":[
  {"id":1,"kind":"ask","ask":1,"round":1,"question":"q1","text":"Explain more","follows":null},
  {"id":2,"kind":"class_swapped","question":"q2","class":"human"},
  {"id":3,"kind":"review_requested","decision":"d1"},
  {"id":4,"kind":"review_stopped","decision":"d1"},
  {"id":5,"kind":"review_requested","decision":"d2"},
  {"id":6,"kind":"submitted","round":1,"answers":[
    {"question":"q1","class":"human","choice":0,"note":"","deferred":false,"stamp":"person"},
    {"question":"q2","class":"human","choice":0,"note":"","deferred":false,"stamp":"person"}
  ]}
]}
```

### Sent answer variants

The custom variant extends the old choice/deferral contract; it changes the meaning of
`choice: null`, which is no longer sufficient evidence of deferral.

| Variant | Sent fields | Interpretation |
|---|---|---|
| Choice | `choice: index`, no `free_text`, `deferred: false`, optional content in `note` | The indexed option is the answer; `note` is separate supporting text. |
| Custom | `choice: null`, nonblank `free_text`, `deferred: false`, `note: ""`, `stamp: "person"` | `free_text` is the answer body, exactly as written, including whitespace/newlines. Do not treat it as HTML or explanation parts. |
| Deferred | `choice: null`, no `free_text`, `deferred: true` | No answer was sent. Choice-mode deferral retains its note; custom-mode deferral sends an empty note. No retained draft is a decision. |

For example, the following answers mean a custom storage policy and a question to ask again,
respectively; neither is consent to the recommendation:

```json
[
  {"question":"q1","class":"human","choice":null,"free_text":"Keep an export alongside the file.\nExport only when I request it.","note":"","deferred":false,"stamp":"person"},
  {"question":"q2","class":"provisional","choice":null,"note":"","deferred":true,"stamp":"person"}
]
```

Clear custom text updates the records directly. If "keep it local" conflicts with an existing
cloud-sync decision, ask which policy supersedes the other. If "export regularly" lacks a
required schedule, ask for that schedule; do not invent one or substitute an existing option.
Old pending choice and deferred events omit `free_text` and retain their original meaning.

## A reply

`{ "parts": [...] }`. The ordered explanation schema, roles, bounds and representation choices
are in `explanations.md`. The CLI ask argument identifies the reply; titles are not identity.
Unknown fields and old `text`/`diagram` reply inputs are refused.

```json reply
{
  "parts": [
    {"type":"text","body":"A JSON file can be opened with any editor, so the records stay readable without shoryo."},
    {"type":"diagram","title":"Reading the file","role":"example","source":"file = One JSON file\neditor = Any editor\nshoryo = shoryo\n| shoryo | file | editor |\nshoryo -> file : writes\neditor -> file : opens"}
  ]
}
```

## The result

`shoryo result` prints the topic's whole data: the same JSON that is stored. The parts a
workflow reads to write a specification:

| Field | Meaning |
|---|---|
| `title`, `original_request`, `ended` | The topic. |
| `rounds[]` | Each round: `number`, `subject`, `review`, `fixes[]` (`text`, `change` with `decision`, `before`, `after`), `questions[]`, `asks[]`, `review_conclusions`, `confirmed`, `submitted`, and the snapshots and send information below. |
| `rounds[].questions[]` | As sent in the round, with `class` as it stands after any swap, and `answer`: `method` (`"choice"` or `"custom"`), `selected` (retained option index), `free_text` (retained custom body), `note`, `deferred`, `stamp` (`null` while unstamped, `"person"` or `"pre_approved"`; every sent answer has one), `stamped_at` (the last time the person pressed the stamp, UTC; `null` when unstamped or pre-approved). Old answers without `method`/`free_text` mean choice/empty custom draft. |
| `rounds[].sent_at` | The server's UTC time when the round was sent; `null` before sending. A pre-approved stamp uses this date on the screen. |
| `rounds[].sent_as` | For a sent result round: `"proceeded"` or `"review_requested"`, according to whether any decision was in review when sent. `null` for other rounds and before sending. |
| `rounds[].finished_picture`, `rounds[].records` | The picture and the `not_building`, `undecided`, `delegated`, `rejected` lists as they stood after this round was applied; an omitted field keeps its preceding value. Past results use these snapshots and the decisions' history as of that round, not today's records. |
| `rounds[].asks[]` | `id`, `question`, `text`, `follows`, `state` (`status`: `waiting`, `replied` with `parts`, or `no_reply`). |
| `records.decisions[]` | `id`, `origin` (`{"question": id}` or `{"fix_round": n}`), `history[]` of `{round, content: {name, text}}`, oldest first; the last entry is the current content. |
| `records.not_building`, `undecided`, `delegated`, `rejected` | As sent. |
| `records.revisions[]` | 改めたこと: `decision`, `round`, `before`, `after`. |
| `records.in_review[]` | Decisions the person asked to review and no round has concluded yet: `decision`, `since_round`. |
| `finished_picture` | The latest finished picture. |

Times are RFC 3339 UTC strings. Lifting a person's stamp clears `stamped_at`; pressing it
again records a new server time. The `submitted` event's `stamp` field is unchanged; times
are read from the result data, not that event.

The result deliberately retains both drafts. Only a `submitted` round contains sent answers:
first check `deferred`; when true neither `selected` nor `free_text` was sent. Otherwise,
`method: "choice"` sends the indexed option with `note`, while `method: "custom"` sends only
`free_text`. An inactive selected option, custom draft or hidden note is not an additional
answer. This interpretation remains available after the event is acknowledged.
