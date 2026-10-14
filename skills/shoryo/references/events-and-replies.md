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

Start with `shoryo result <topic> --current` when writing a specification or resuming. It returns
`title`, `original_request`, `ended`, `latest_round`, `pending_events` (the unacknowledged count),
and `records`. Decision entries contain `id`, `name`, `text`, `origin`, `origin_round` and `pre_approved`; their
text is the exact latest content, not a generated summary. The other records are `not_building`,
`undecided`, `delegated`, `rejected` and `in_review`, unchanged from stored data.

`latest_round` is `null` before any round, otherwise `{number, submitted, sent_as, question_count}`.
It reports state, not approval: an unsent round, a question round or a result sent with
`sent_as: "review_requested"` is not permission to proceed. Check the workflow's convergence
conditions and outstanding in-review marks. Current decisions never include unsent answer drafts.
The event count is not proof of unhandled work (handled events may still await acknowledgment);
use the normal wait/ack loop, rather than treating a read as acknowledgment.

For an original answer or reason, use `shoryo result <topic> --answer QID`, taking QID from the
decision's `origin.question`. The receipt contains `question`, `text`, `round`, `submitted`,
`sent_at`, `answer`, `selected_option` and `asks`. `answer` has the same shape and interpretation
as one submitted-event answer. `selected_option` contains only `text` and `consequence` for a
sent choice; it is `null` for custom answers and deferrals. Both are `null` before submission,
even if the person stamped a draft. Do not treat an unsent answer as agreement.

The receipt omits backgrounds, option descriptions, other options and replies. `asks` contains
only `{id, follows, status}` for this question, with status `waiting`, `replied` or `no_reply`.
Read `--ask ID` for the specific ask and reply you need; it returns the stored ask object without
automatically including its preceding or following conversations. Read `--question QID` only
when the complete question context is needed. It returns the stored question including retained
drafts, so consult the receipt's send state before interpreting those drafts as answers.

Use `--decision ID` for the decision's complete history, and `--round N` when the entire round
or picture is needed. Do not retrieve a whole round merely to read one answer's reason.
Use `origin_round` in the current records to locate the source; it can differ from the history's
first effective round. It is `null` if the source question is absent from the stored topic;
do not infer a round or agreement in that case. Selectors return the corresponding stored object
without other rounds or decisions.
All result forms work after the server stops. Unknown selectors fail, never broaden the output.

`shoryo result` prints the topic's whole data: the same JSON that is stored. The parts a
workflow can read when a selective result is insufficient:

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
