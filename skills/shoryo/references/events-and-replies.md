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
| `submitted` | `round`, `answers` | The person sent the whole round. Each answer: `question`, `class` (as sent), `choice` (index into the question's options, from 0; `null` when deferred), `note`, `deferred`, `sent_unseen`. |
| `review_requested` | `decision` | The person wants this decision asked again (見直したい). |
| `review_stopped` | `decision` | The person withdrew that request. |
| `class_swapped` | `question`, `class` | The person moved a question between `human` and `provisional`. |

An example of what `wait` prints after an ask, a swap and a send:

```json output
{"events":[
  {"id":1,"kind":"ask","ask":1,"round":1,"question":"q1","text":"Explain more","follows":null},
  {"id":2,"kind":"class_swapped","question":"q2","class":"human"},
  {"id":3,"kind":"submitted","round":1,"answers":[
    {"question":"q1","class":"human","choice":0,"note":"","deferred":false,"sent_unseen":false},
    {"question":"q2","class":"human","choice":0,"note":"","deferred":false,"sent_unseen":true}
  ]}
]}
```

## A reply

`{ "text": "...", "diagram": "..." }`. `diagram` is optional diagram text (see the diagram
reference). Unknown fields are refused.

```json reply
{
  "text": "A JSON file can be opened with any editor, so the records stay readable without shoryo.",
  "diagram": "file = One JSON file\neditor = Any editor\nshoryo = shoryo\n| shoryo | file | editor |\nshoryo -> file : writes\neditor -> file : opens"
}
```

## The result

`shoryo result` prints the topic's whole data: the same JSON that is stored. The parts a
workflow reads to write a specification:

| Field | Meaning |
|---|---|
| `title`, `original_request`, `ended` | The topic. |
| `rounds[]` | Each round: `number`, `subject`, `review`, `fixes[]` (`text`, `change` with `decision`, `before`, `after`), `questions[]`, `asks[]`, `review_conclusions`, `confirmed`, `submitted`. |
| `rounds[].questions[]` | As sent in the round, with `class` as it stands after any swap, and `answer`: `selected` (option index), `note`, `deferred`, `opened`, `touched`, `sent_unseen`. |
| `rounds[].asks[]` | `id`, `question`, `text`, `follows`, `state` (`status`: `waiting`, `replied` with `text` and `diagram`, or `no_reply`). |
| `records.decisions[]` | `id`, `origin` (`{"question": id}` or `{"fix_round": n}`), `history[]` of `{round, content: {name, text}}`, oldest first; the last entry is the current content. |
| `records.not_building`, `undecided`, `delegated`, `rejected` | As sent. |
| `records.revisions[]` | 改めたこと: `decision`, `round`, `before`, `after`. |
| `records.in_review[]` | Decisions the person asked to review and no round has concluded yet: `decision`, `since_round`. |
| `finished_picture` | The latest finished picture. |
