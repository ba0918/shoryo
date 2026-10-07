# The round format

Read this when writing a round for `shoryo round`. A round is one JSON object. Unknown fields are
refused, so a misspelt field name fails loudly instead of being ignored.

## Fields

| Field | Required | Meaning |
|---|---|---|
| `title` | first round | The topic's title, shown at the top of the screen. A later round may change it. |
| `original_request` | first round | The topic in the person's own words. A later round may change it. |
| `subject` | yes | The round's subject; it heads the round's column on the map. |
| `review` | no (`false`) | `true` for a review round. |
| `fixes` | no | Review rounds: what you fixed from the records without asking. Each is `{ "text", "decision"? }`; name `decision` when the fix changed that decision, and send its new content in `records.decisions`. |
| `questions` | no | The questions of this round, in the order to show them. |
| `records` | no | The records, updated with the previous round's answers (below). |
| `review_conclusions` | no | For each decision the person asked to review: `{ "decision", "outcome": "unchanged" \| "changed" }`. |
| `confirmed` | no | Points you checked are still right after a decision they rest on changed: `{ "decision": id }` or `{ "question": id }`. |
| `finished_picture` | no | The finished picture as diagram text (see the diagram reference). Send it with every round. |

### A question

| Field | Meaning |
|---|---|
| `id` | Your identifier, unique in the topic. A question asked again gets a new id. |
| `text` | The question. |
| `class` | `"human"` (人が決める) or `"provisional"` (仮決め). |
| `why_now` | One line: why this is decided now. |
| `premises` | Ids of the decisions this question rests on, most important first. Each must be in the records, or in this round's `records.decisions`. |
| `background` | Terms and background needed to read the question. |
| `options` | The options, each `{ "text", "description", "recommended", "consequence" }`. Exactly one is `"recommended": true`; every `consequence` (この答えだと) is filled in. |
| `reasks` | Optional: the id of the deferred question this one asks again. |

### Records

| Field | Meaning |
|---|---|
| `decisions` | 決まったこと: `{ "id", "name", "text", "decided_by" }`. `name` is the short name used in chains and on the map. `decided_by` is `{ "question": id }`, or `"fix"` for a decision made by a fix in this review round. Decisions are added or updated by `id` and never removed: send only new or changed ones. Changing a decision's `name` or `text` records the old content as 改めたこと by itself. |
| `not_building` | 作らないもの: a list of strings. |
| `undecided` | 未決: `{ "text", "decider" }`. |
| `delegated` | 任せたこと: `{ "text", "reason" }`. |
| `rejected` | 見送った案: `{ "text", "reason", "question" }`, `question` being the id of the question it was rejected in. |
| `in_review` | Ignored: the in-review marks belong to the person. |

Each list other than `decisions` replaces the stored list when present, and leaves it as it was
when absent. Send the whole list whenever one entry changes.

## Refusals

`shoryo round` refuses the round, prints the reason and exits non-zero, leaving the screen as it
was, when a question has not exactly one recommended option, an option has no consequence, a
premise names an unknown decision, a question id was used before, or the previous round has not
been sent. Fix the round and send it again.

## Examples

A first round:

```json round
{
  "title": "Where the topic data lives",
  "original_request": "I want the brainstorm records kept somewhere I can read later.",
  "subject": "Storage",
  "questions": [
    {
      "id": "q1",
      "text": "Where should a topic's data be stored?",
      "class": "human",
      "why_now": "Every later question about reading and deleting rests on it.",
      "premises": [],
      "background": "A topic is one brainstorm. Its data is the rounds, the answers and the records.",
      "options": [
        {
          "text": "One JSON file per topic",
          "description": "A plain file in the per-user data directory.",
          "recommended": true,
          "consequence": "The data can be read and copied with ordinary tools."
        },
        {
          "text": "An embedded database",
          "description": "A small database file managed by a library.",
          "recommended": false,
          "consequence": "Reading the data needs that library or its tools."
        }
      ]
    },
    {
      "id": "q2",
      "text": "What is the data file called?",
      "class": "provisional",
      "why_now": "The file is created when the first round is sent.",
      "premises": [],
      "background": "",
      "options": [
        {
          "text": "state.json",
          "description": "Says what it holds.",
          "recommended": true,
          "consequence": "The file is state.json in the topic's directory."
        },
        {
          "text": "topic.json",
          "description": "Names the unit.",
          "recommended": false,
          "consequence": "The file is topic.json in the topic's directory."
        }
      ]
    }
  ],
  "finished_picture": "agent = Agent\nshoryo = shoryo\nstore = ? Where the data lives\n| agent | shoryo | store |\nagent -> shoryo : rounds\nshoryo -> agent : answers\nshoryo -> store : saves"
}
```

The next round, after the person sent the first one: the answers become records, and a new
question rests on a decision made from them.

```json round
{
  "subject": "Reading and deleting",
  "questions": [
    {
      "id": "q3",
      "text": "Who may delete a topic's data?",
      "class": "human",
      "why_now": "Deleting cannot be undone, and the file now exists.",
      "premises": ["d1"],
      "background": "The data stays after the topic ends unless someone deletes it.",
      "options": [
        {
          "text": "Only the person, by hand",
          "description": "shoryo never deletes anything.",
          "recommended": true,
          "consequence": "Old topics stay until the person removes them."
        },
        {
          "text": "shoryo, after a month",
          "description": "Old topics are cleaned up automatically.",
          "recommended": false,
          "consequence": "A topic not opened for a month is gone."
        }
      ]
    }
  ],
  "records": {
    "decisions": [
      { "id": "d1", "name": "One JSON file", "text": "Each topic's data is one JSON file in the per-user data directory.", "decided_by": { "question": "q1" } },
      { "id": "d2", "name": "state.json", "text": "The file is called state.json.", "decided_by": { "question": "q2" } }
    ],
    "rejected": [
      { "text": "An embedded database", "reason": "Reading the data would need extra tools.", "question": "q1" }
    ],
    "not_building": ["A web service that stores topics for others"]
  },
  "finished_picture": "agent = Agent\nshoryo = shoryo\nstore = One JSON file\nwho = ? Who deletes\n| agent | shoryo | store |\n| .     | .      | who   |\nagent -> shoryo : rounds\nshoryo -> agent : answers\nshoryo -> store : saves\nwho -> store : deletes"
}
```

A review round, after the person sent the second round and asked to review `d2`: one fix
changed a decision, the review changed `d2`, and a point resting on `d1` is confirmed.

```json round
{
  "subject": "Review",
  "review": true,
  "fixes": [
    { "text": "Stated where the per-user data directory is on Linux", "decision": "d1" }
  ],
  "questions": [
    {
      "id": "q4",
      "text": "Is the review complete?",
      "class": "human",
      "why_now": "The specification is written from these records next.",
      "premises": ["d1", "d3"],
      "background": "",
      "options": [
        { "text": "Yes", "description": "Nothing else to change.", "recommended": true, "consequence": "The workflow writes the specification." },
        { "text": "No", "description": "Something is still wrong.", "recommended": false, "consequence": "Another review round follows." }
      ]
    }
  ],
  "records": {
    "decisions": [
      { "id": "d1", "name": "One JSON file", "text": "Each topic's data is one JSON file under ~/.local/share/shoryo/ on Linux.", "decided_by": { "question": "q1" } },
      { "id": "d2", "name": "topic.json", "text": "The file is called topic.json.", "decided_by": { "question": "q2" } },
      { "id": "d3", "name": "Person deletes", "text": "Only the person deletes a topic's data.", "decided_by": { "question": "q3" } }
    ],
    "delegated": [
      { "text": "The JSON field names", "reason": "Any names keep the same behaviour." }
    ],
    "undecided": [
      { "text": "Whether to compress old topics", "decider": "the person" }
    ]
  },
  "review_conclusions": [ { "decision": "d2", "outcome": "changed" } ],
  "confirmed": [ { "decision": "d3" } ],
  "finished_picture": "agent = Agent\nshoryo = shoryo\nstore = topic.json\nwho = Person deletes\n| agent | shoryo | store |\n| .     | .      | who   |\nagent -> shoryo : rounds\nshoryo -> agent : answers\nshoryo -> store : saves\nwho -> store : deletes"
}
```
