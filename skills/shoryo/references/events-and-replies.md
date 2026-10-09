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
| `submitted` | `round`, `answers` | The person sent the whole round. Each answer: `question`, `class` (as sent), `choice` (index into the question's options, from 0; `null` when deferred), `note`, `deferred`, `stamp` (判子: `"person"` when the person stamped it, `"pre_approved"` when a provisional answer was sent with the stamp you put on it, 代決). `answers` is empty for a result round. |
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
| `rounds[].questions[]` | As sent in the round, with `class` as it stands after any swap, and `answer`: `selected` (option index), `note`, `deferred`, `stamp` (`null` while unstamped, `"person"` or `"pre_approved"`; every sent answer has one), `stamped_at` (the last time the person pressed the stamp, UTC; `null` when unstamped or pre-approved). |
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
