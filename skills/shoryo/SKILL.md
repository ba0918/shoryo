---
name: shoryo
description: Run a grill-me style brainstorm on the shoryo screen instead of in the chat — put each round of questions on a local browser page, wait for the person's asks and answers, reply, and keep every record in shoryo's data. Use when a brainstorm workflow (such as the ba0918 or kotowari brainstorm) hands its rounds to shoryo, or when the person says "do this brainstorm in shoryo", "use shoryo", "put the questions on the screen", or "壁打ちを shoryo でやって" / "画面で聞いて". Starts a local server and writes topic data only when one of those asked for it.
---

# shoryo

## Scope

This skill tells you how to run brainstorm rounds through shoryo: starting a topic, writing
rounds as data, classifying questions, the waiting loop, replying to asks, handling sent answers,
deferred questions, review requests and review rounds, the finished picture, and ending.

It does not decide how the question tree is explored, when the brainstorm is finished, or how a
specification is written, placed or approved from the result: that is the calling workflow's job
(for example the ba0918 or kotowari brainstorm). It does not change those workflows.

Side effects: this skill starts a local server and writes the topic's data in the per-user data
directory. Do this only when the person asked for the brainstorm to run on shoryo, or a workflow
the person started hands its rounds to shoryo. Never start shoryo as a by-product of other work.

Requires the `shoryo` binary on the PATH (`shoryo --version` prints its version). The command
reference is `references/commands.md`; read it before the first command in a session.

## The records live in shoryo only

While a topic runs on shoryo, write the records (decisions, not building, undecided, delegated,
rejected options, revisions) only into shoryo's data, through the rounds you send. Do not keep
a second copy in a workflow progress file. The workflow reads them back with `shoryo result`.

## Starting a topic

1. Choose a topic name (letters, digits, `.`, `_`, `-`) and run `shoryo start <topic>` in the
   background from the repository; it keeps running. It prints the page URL. A topic with data
   resumes where it stopped. If it says the topic is already running, use that server.
2. Tell the person the URL in one line.

## Never write the questions into the conversation

After each round, write one line in the conversation: "Round N is on the screen" and the URL
(what `shoryo round` prints). Never write questions, options, recommendations or record contents
into the conversation, and never write HTML: you send structured data and the screen draws it.

## Classifying questions

Put each question in one of two classes. The test: would changing the answer later be costly?

- `human` (人が決める): purpose, policy, behaviour a user can see, trade-offs — anything whose
  later change would undo other decisions.
- `provisional` (仮決め): names, small defaults and the like — any option keeps the decided
  behaviour the same, and it is cheap to change later.

Do not put a question that shapes behaviour or design in `provisional`. A provisional question
arrives on the screen already stamped for the person (代決, pre-approved); the person reviews the
list and sends back only what looks wrong, so a misplaced question can be sent without being
looked at. When unsure, make it `human`.

When the person swaps a question's class (a `class_swapped` event), treat it in that class from
then on, including when you ask it again; use the swap as a hint for classifying similar
questions. A provisional question becomes an answer only when it is sent. Silence is not consent.

## Writing a round

Write every question with: its text; one line on why it is decided now; the decisions it rests
on; the background needed to read it; options, exactly one recommended, each with a description
and its consequence (この答えだと). Give every decision a short name — it is what chains and the
map show — and every round a subject, which heads its column on the map. Send the updated
records and the redrawn finished picture with every round.

The format, its fields and three complete examples are in `references/round.md`; read it before
writing the first round. `shoryo round` checks the round and refuses it with the reason if it
breaks a rule; fix it and send it again.

Before writing backgrounds, option descriptions or replies, read `references/explanations.md`.
This changes the former string backgrounds/descriptions and text/diagram replies into ordered
parts. That reference owns representation selection, role/meaning checks, bounded schema and
repair. Successful acceptance or drawing alone does not establish meaning or agreement.

## The waiting loop

After sending a round, keep running `shoryo wait <topic> --ack <ids received last time>` until
the round is sent. When it returns with no events (after `--timeout`), wait again. Handle each
event:

- `ask`: reply to it (below), in the order received, then wait again.
- `review_requested`, `review_stopped`, `class_swapped`: remember it for the next round and wait
  again. Do not go back to the conversation.
- `submitted`: update the records from the answers and send the next round. For a result round
  see "When the topic converges".

Always pass the ids you received on the next `wait`; unacknowledged events come back. The event
formats are in `references/events-and-replies.md`.

## Replying to an ask

Answer the ask about its question with `shoryo reply <topic> <ask id>` using ordered parts from
`references/explanations.md`; choose the smallest representation that clarifies the answer. Never rewrite the
question in a round already sent: if the question was badly put, offer the rephrasing inside the
reply. Asks may arrive after the round was sent; reply to them too.

## Handling sent answers

Every answer in a `submitted` event is an answer — including ones left at the recommendation and
provisional ones sent with your pre-approval (`"stamp": "pre_approved"`, 代決) — except a
deferred one (`deferred`, `choice` null). Every question carries a stamp (判子) before the round
can be sent; `stamp` says whether the person pressed it or left yours. Record the answers as
decisions, not building, undecided, delegated or rejected options before sending the next
round. The screen marks decisions made from pre-approved answers as 代決 until they are revised.

### Deferred questions

A deferred question stays undecided. Ask it again in the next round in a different way: split it
into smaller questions, make it concrete with a scenario, or add background. Give the new
question a new id and set `reasks` to the old one. Never decide it by the recommendation.

### Review requests (見直す)

Ask each decision in review again in the next round. Then:

- If the answer changed, send the decision with its new content (the old content is kept as a
  revision by shoryo) and the conclusion `changed`.
- If it did not, send the conclusion `unchanged`.

Either way send a `review_conclusions` entry for it. After a change, check in the next round
whether each question and decision resting directly on it is still right, and send those you
confirm in `confirmed`.

A `review_stopped` can arrive even after you have asked that decision again, until you send
its conclusion. Withdrawal returns the decision to its original content: keep that content,
ignore the answer to the re-asked question, and send the conclusion `unchanged`. This extends
withdrawal beyond the arrival of the next round; it is not limited to the round in which the
review was requested.

## When the topic converges

When the question tree is exhausted and the only undecided items left are the person's to
decide later, send a round with no questions, carrying the last records and finished picture.
The screen shows it as the result; the format is in `references/round.md` ("The result round").

- An empty `submitted` for that round, with no `review_requested` pending (none received since,
  or each withdrawn by `review_stopped`), means "proceed with this result": return to the
  workflow, which writes the specification from `shoryo result`.
- With review requests pending, the person pressed "Send review requests": ask those decisions
  again in the next round, as under "Review requests" above, and send a result round again once
  they are settled.

## Review rounds

When the workflow's review produces findings, do not ask them in the conversation. Send them as
the next round of the same topic (`"review": true`), sorted three ways:

- Fixes you can derive from the records: fix them without asking and list them in `fixes`; when
  a fix changes a decision, name it and send the decision's new content.
- Questions the person decides: `human` questions, by the classification above.
- Provisional ones: `provisional` questions.

If the topic was ended, sending the round reopens it.

## The finished picture

Redraw the finished picture with every round: one diagram of what the topic will end up as.
Put each decision at the place it belongs, in a few words; leave places not decided yet as empty
slots (`id = ? label`). It is a picture, not a list of decisions. The format is in
`references/diagram.md`.

## Ending

When the workflow has finished up to approval, run `shoryo end <topic>`. Sending the last round
does not end the topic, and `end` is refused while a round is unsent. After ending, the page
stays readable; `shoryo result <topic>` gives the workflow the whole data, and works after the
server is stopped too. Run `shoryo stop <topic>` when the topic's server is no longer needed. Do
not delete the topic's data; that is the person's decision.

## Evidence

- After each round: the line `shoryo round` printed, and nothing else about the round in the
  conversation.
- Before the next round: the `wait` output whose `submitted` event the records were updated from.
- At the end: the exit status of `shoryo end`, and `shoryo result` output handed to the workflow.
