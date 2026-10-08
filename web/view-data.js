// Turns the server's view into the read-only data each component draws. Decisions about
// what is shown are made here, once, not inside the components. `view.lang` is the screen's
// language, carried into every piece of view data so a switch redraws it.
import { translator } from "./strings.js";

export function decisionContent(decision, round = Infinity) {
  const versions = decision.history.filter((version) => version.round <= round);
  return versions.length > 0 ? versions[versions.length - 1].content : null;
}

export function findQuestion(topic, id) {
  for (const round of topic.rounds) {
    const question = round.questions.find((q) => q.id === id);
    if (question) return { question, round };
  }
  return null;
}

export function findDecision(topic, id) {
  return topic.records.decisions.find((decision) => decision.id === id) ?? null;
}

/// The question a decision was decided in, as text, or the fix round it came from.
export function decisionSource(view, decision) {
  if (decision.origin.question !== undefined) {
    return findQuestion(view.topic, decision.origin.question)?.question.text ?? decision.origin.question;
  }
  return translator(view.lang)("decision.fixed-in", { round: decision.origin.fix_round });
}

/// When a stamp was put on: the person's when they pressed it; the LLM's 代決 when the round
/// carrying it was sent, and no time before that.
function stampedAt(round, answer) {
  switch (answer.stamp) {
    case "person":
      return answer.stamped_at;
    case "pre_approved":
      return round.sent_at;
    default:
      return null;
  }
}

/// A question as its card or row draws it. `asOf` is the round whose content the
/// prerequisites are shown with; by default their current content.
export function questionData(view, round, question, locked, landed = null, asOf = Infinity) {
  const topic = view.topic;
  return {
    lang: view.lang,
    id: question.id,
    text: question.text,
    why_now: question.why_now,
    background: question.background,
    chains: view.chains[question.id] ?? [],
    premises: question.premises.map((id) => {
      const decision = findDecision(topic, id);
      const content = decision ? decisionContent(decision, asOf) : null;
      return { id, name: content?.name ?? id, text: content?.text ?? "" };
    }),
    options: question.options.map((option, index) => ({ index, ...option })),
    selected: question.answer.selected,
    note: question.answer.note,
    deferred: question.answer.deferred,
    stamp: question.answer.stamp,
    stampedAt: stampedAt(round, question.answer),
    asks: questionAsks(round, question.id),
    locked,
    landed: question.id === landed,
  };
}

export function currentRoundData(view, landed = null) {
  const topic = view.topic;
  const round = topic.rounds[topic.rounds.length - 1];
  if (!round) return null;
  const locked = round.submitted || topic.ended;
  const questions = round.questions.map((question) => ({
    cls: question.class,
    data: questionData(view, round, question, locked, landed),
  }));
  const result = round.questions.length === 0;
  const reviewing = topic.records.in_review.length > 0;
  return {
    lang: view.lang,
    result: result ? resultData(view, locked) : null,
    human: questions.filter((q) => q.cls === "human").map((q) => q.data),
    provisional: questions.filter((q) => q.cls === "provisional").map((q) => q.data),
    send: {
      round: round.number,
      locked,
      unstamped: locked ? 0 : view.unstamped.length,
      result,
      label: result ? (reviewing ? "send.review-requests" : "send.proceed") : "send.all",
      lang: view.lang,
    },
    fixes: round.review ? fixesData(view, round) : [],
    notice: round.submitted && !topic.ended ? (proceeded(topic) ? "sent.proceeded" : "sent.next-round") : null,
  };
}

/// Whether the person proceeded with the result: it was sent without review requests, so the
/// brainstorm is over on this screen and no next round is coming; with review requests, the
/// LLM asks again. The server records which it was when the result was sent.
function proceeded(topic) {
  const round = topic.rounds[topic.rounds.length - 1];
  return !topic.ended && round !== undefined && round.submitted && round.sent_as === "proceeded";
}

/// How long the agent may be silent, neither waiting nor sending a command, before the
/// screen says it is not responding.
export const NOT_RESPONDING_MS = 10 * 60 * 1000;

/// What the header says the LLM is doing at `now`: waiting for the screen, working on what
/// it received, or not responding; nothing once the topic has ended. `agent` is the server's
/// report, received at `receivedAt`. Once the person proceeded with the result, the LLM writes
/// the specification without calling the screen, so its silence is not taken as not responding.
export function agentStatus(agent, receivedAt, topic, now) {
  if (topic.ended || !agent) return null;
  if (agent.waiting) return { state: "waiting", changesIn: null };
  if (proceeded(topic)) return { state: "working", changesIn: null };
  const quiet = agent.quiet_ms + (now - receivedAt);
  if (quiet >= NOT_RESPONDING_MS) return { state: "not-responding", changesIn: null };
  return { state: "working", changesIn: NOT_RESPONDING_MS - quiet };
}

/// 結果: what a round without questions shows — the finished picture and the records.
function resultData(view, locked) {
  const records = decisionsTabData(view);
  return {
    lang: view.lang,
    picture: view.topic.finished_picture,
    decisions: records.decisions.map((item) => ({
      ...item,
      review: reviewState(view, item.id, { resultWording: !locked }),
    })),
    not_building: records.not_building,
    rejected: records.rejected,
    undecided: records.undecided,
    delegated: records.delegated,
  };
}

/// What "Send all" is about to send, for the confirmation: each question's answer, stamp,
/// defer switch and note, and the review requests waiting to go with it.
export function confirmData(view) {
  const topic = view.topic;
  const round = topic.rounds[topic.rounds.length - 1];
  return {
    lang: view.lang,
    round: round.number,
    result: round.questions.length === 0,
    questions: round.questions.map((question) => ({
      id: question.id,
      text: question.text,
      answer: question.answer.deferred ? null : question.options[question.answer.selected]?.text ?? "",
      deferred: question.answer.deferred,
      note: question.answer.note,
      stamp: question.answer.stamp,
    })),
    reviews: topic.records.in_review
      .map((entry) => findDecision(topic, entry.decision))
      .filter((decision) => decision !== null)
      .map((decision) => ({ id: decision.id, name: decisionContent(decision).name })),
  };
}

export function decisionDetail(view, id) {
  const decision = findDecision(view.topic, id);
  if (!decision) return null;
  return decisionItemData(view, decision);
}

/// Whether 見直す can be pressed or stopped for this decision now. 見直す shows until the topic
/// ends, and cannot be pressed between sending a round and the next one, when the LLM would
/// not read it; stopping works until a round gives the review's conclusion. `resultWording`
/// says 見直す is pressed inside the current result, which is still to be sent.
export function reviewState(view, id, { resultWording = false } = {}) {
  const topic = view.topic;
  const round = topic.rounds[topic.rounds.length - 1];
  const inReview = topic.records.in_review.some((entry) => entry.decision === id);
  const offered = !inReview && !topic.ended;
  return {
    lang: view.lang,
    decision: id,
    inReview,
    showReview: offered,
    canReview: offered && !(round?.submitted ?? false),
    canStop: inReview && !topic.ended,
    resultWording,
  };
}

/// Whether a decision carried the 代決 mark as of `round`: it came from an answer sent with
/// the LLM's stamp and had not been revised by then. The server says it for today.
function preApprovedAsOf(view, decision, round) {
  if (round === Infinity) return view.pre_approved.includes(decision.id);
  const origin = decision.origin.question !== undefined ? findQuestion(view.topic, decision.origin.question) : null;
  const versions = decision.history.filter((version) => version.round <= round);
  return origin?.question.answer.stamp === "pre_approved" && versions.length === 1;
}

/// A decision for a list. `asDecided` shows the content it had when it was decided, marked
/// when it was revised later; `asOf` shows the content and the 代決 mark it had as of that
/// round; otherwise its current content.
export function decisionItemData(view, decision, { asDecided = false, asOf = Infinity } = {}) {
  const content = asDecided ? decision.history[0].content : decisionContent(decision, asOf);
  return {
    lang: view.lang,
    id: decision.id,
    name: content.name,
    text: content.text,
    source: decisionSource(view, decision),
    revised: asDecided && decision.history.length > 1,
    preApproved: preApprovedAsOf(view, decision, asOf),
    review: reviewState(view, decision.id),
    jump: decision.origin.question !== undefined ? questionTarget(view.topic, decision.origin.question) : null,
  };
}

function decidedBy(topic, questionId) {
  return topic.records.decisions.filter((decision) => decision.origin.question === questionId);
}

/// How many characters of a reply a follow-up quotes.
const QUOTE_CHARS = 40;

/// The opening of a reply, as a follow-up quotes it.
function excerpt(text) {
  const chars = [...text];
  return chars.length > QUOTE_CHARS ? `${chars.slice(0, QUOTE_CHARS).join("")}…` : text;
}

function askData(ask) {
  const replied = ask.state.status === "replied";
  return {
    id: ask.id,
    text: ask.text,
    follows: ask.follows,
    status: ask.state.status,
    reply: replied ? { text: ask.state.text, diagram: ask.state.diagram } : null,
    excerpt: replied ? excerpt(ask.state.text) : null,
  };
}

/// A question's exchanges in the order they were asked; a follow-up carries the quote of the
/// reply it continues.
function questionAsks(round, questionId) {
  const asks = round.asks.filter((ask) => ask.question === questionId).map(askData);
  const byId = new Map(asks.map((ask) => [ask.id, ask]));
  return asks.map((ask) => ({ ...ask, quote: byId.get(ask.follows)?.excerpt ?? null }));
}

/// Up to this many exchanges are all shown; beyond it the middle is folded away.
const ALL_EXCHANGES_UP_TO = 5;
const FIRST_SHOWN = 1;
const LAST_SHOWN = 2;

/// Which exchanges a thread shows: all of them, or the first and the latest with the number
/// folded away between them.
export function shownExchanges(asks, expanded) {
  if (expanded || asks.length <= ALL_EXCHANGES_UP_TO) return { head: asks, elided: 0, tail: [] };
  return {
    head: asks.slice(0, FIRST_SHOWN),
    elided: asks.length - FIRST_SHOWN - LAST_SHOWN,
    tail: asks.slice(asks.length - LAST_SHOWN),
  };
}

export function fixesData(view, round) {
  return round.fixes.map((fix, index) => ({
    index,
    text: fix.text,
    change: fix.change
      ? {
          before: fix.change.before,
          after: fix.change.after,
          review: reviewState(view, fix.change.decision),
        }
      : null,
  }));
}

/// The sent round the past rounds tab shows: the one chosen, or the latest.
export function shownPastRound(topic, selected) {
  const rounds = topic.rounds.filter((round) => round.submitted);
  return rounds.find((round) => round.number === selected) ?? rounds[rounds.length - 1] ?? null;
}

/// Every sent round, as it was answered: the same cards and provisional list as the current
/// round, read only, with what was sent and what was decided there.
export function pastRoundsData(view, selected, landed = null) {
  const topic = view.topic;
  const rounds = topic.rounds.filter((round) => round.submitted);
  const chosen = shownPastRound(topic, selected);
  const questions = chosen
    ? chosen.questions.map((question) => ({
        cls: question.class,
        data: {
          ...questionData(view, chosen, question, true, landed, chosen.number),
          past: {
            chosen: question.answer.deferred ? null : question.options[question.answer.selected].text,
            recommended: question.options.find((option) => option.recommended)?.text ?? "",
            decisions: decidedBy(topic, question.id).map((decision) =>
              decisionItemData(view, decision, { asDecided: true }),
            ),
          },
        },
      }))
    : [];
  return {
    lang: view.lang,
    choices: rounds.map((round) => ({ number: round.number, subject: round.subject })),
    selected: chosen?.number ?? null,
    round: chosen
      ? {
          number: chosen.number,
          subject: chosen.subject,
          result: chosen.questions.length === 0 ? pastResultData(view, chosen) : null,
          fixes: fixesData(view, chosen),
          human: questions.filter((q) => q.cls === "human").map((q) => q.data),
          provisional: questions.filter((q) => q.cls === "provisional").map((q) => q.data),
        }
      : null,
  };
}

/// A sent result round as it was sent: its own finished picture and records, the decisions
/// as they stood then, and how the person sent it.
function pastResultData(view, round) {
  const topic = view.topic;
  return {
    lang: view.lang,
    picture: round.finished_picture,
    decisions: topic.records.decisions
      .filter((decision) => decisionContent(decision, round.number) !== null)
      .map((decision) => decisionItemData(view, decision, { asOf: round.number })),
    not_building: round.records.not_building,
    rejected: rejectedData(topic, round.records.rejected),
    undecided: round.records.undecided,
    delegated: round.records.delegated,
    sentAs: round.sent_as,
  };
}

function rejectedData(topic, rejected) {
  return rejected.map((entry) => ({
    text: entry.text,
    reason: entry.reason,
    question: findQuestion(topic, entry.question)?.question.text ?? entry.question,
  }));
}

export function decisionsTabData(view) {
  const topic = view.topic;
  const records = topic.records;
  const name = (id) => {
    const decision = findDecision(topic, id);
    return decision ? decisionContent(decision).name : id;
  };
  return {
    lang: view.lang,
    decisions: records.decisions.map((decision) => decisionItemData(view, decision)),
    not_building: records.not_building,
    rejected: rejectedData(topic, records.rejected),
    undecided: records.undecided,
    delegated: records.delegated,
    revisions: records.revisions.map((revision) => ({
      name: name(revision.decision),
      round: revision.round,
      before: revision.before,
      after: revision.after,
    })),
    in_review: records.in_review
      .map((entry) => findDecision(topic, entry.decision))
      .filter((decision) => decision !== null)
      .map((decision) => decisionItemData(view, decision)),
  };
}

function answerText(view, question, round) {
  if (!round.submitted) return null;
  if (question.answer.deferred) return translator(view.lang)("map.asked-again");
  return question.options[question.answer.selected]?.text ?? null;
}

/// Where a jump to a question lands: its card in the current round, or the past round.
function questionTarget(topic, questionId) {
  const found = findQuestion(topic, questionId);
  if (!found) return null;
  const current = topic.rounds[topic.rounds.length - 1];
  if (found.round.number === current.number && !current.submitted) {
    return { tab: "current", round: null, question: questionId };
  }
  return { tab: "past", round: found.round.number, question: questionId };
}

function nodeDetail(view, node) {
  const topic = view.topic;
  const found = node.question_id ? findQuestion(topic, node.question_id) : null;
  switch (node.kind) {
    case "decision": {
      const decision = findDecision(topic, node.key.slice(2));
      return {
        text: decision ? decisionContent(decision).text : "",
        question: node.question,
        answer: found ? answerText(view, found.question, found.round) : null,
      };
    }
    case "rejected": {
      const rejected = topic.records.rejected[Number(node.key.slice(2))];
      const text = rejected ? translator(view.lang)("map.rejected-because", { reason: rejected.reason }) : "";
      return { text, question: node.question, answer: null };
    }
    default:
      return {
        text: null,
        question: node.label,
        answer: found ? found.question.options[found.question.answer.selected]?.text ?? null : null,
      };
  }
}

function nodeTarget(view, node) {
  const topic = view.topic;
  if (node.question_id) return questionTarget(topic, node.question_id);
  const decision = node.kind === "decision" ? findDecision(topic, node.key.slice(2)) : null;
  if (decision?.origin.fix_round !== undefined) {
    return { tab: "past", round: decision.origin.fix_round, question: null };
  }
  return null;
}

/// The map as drawn: the range, the point the 道筋 leads to (`root`), the selected point, and
/// only the points the range shows. Selecting a point never changes the 道筋; only its root does.
export function mapData(view, range, selected, root) {
  const map = view.map;
  const exists = (key) => map.nodes.some((node) => node.key === key);
  const rooted = range === "path" && exists(root) ? root : null;
  const shown = rooted ? new Set(view.paths[rooted]) : null;
  const visible = (key) => shown === null || shown.has(key);
  const chosen = exists(selected) && visible(selected) ? selected : null;
  const nodes = new Map(map.nodes.map((node) => [node.key, node]));
  return {
    lang: view.lang,
    range,
    root: rooted,
    selected: chosen,
    columns: map.columns.map((column) => ({
      round: column.round,
      subject: column.subject,
      nodes: column.nodes.filter(visible).map((key) => {
        const node = nodes.get(key);
        return {
          key,
          kind: node.kind,
          label: node.label,
          question: node.kind === "question" ? null : node.question,
          inReview: node.in_review,
          reviewMark: node.review_mark,
          selected: key === chosen,
          detail: nodeDetail(view, node),
          jump: nodeTarget(view, node),
          review: node.kind === "decision" ? reviewState(view, key.slice(2)) : null,
        };
      }),
    })),
    edges: map.edges.filter((edge) => visible(edge.from) && visible(edge.to)),
  };
}

/// The confirmation 見直す opens: which decision, and whether it is pressed in the current
/// result, where sending with "Send review requests" is what asks the LLM.
export function reviewConfirmData(view, decision, resultWording) {
  const found = findDecision(view.topic, decision);
  return {
    lang: view.lang,
    decision,
    name: found ? decisionContent(found).name : decision,
    resultWording,
  };
}

/// Whether a place "Back" would return to still shows what it was anchored on: the card, row
/// or result of the round it showed, or a decision. A place on the current tab is gone once
/// the next round has arrived, because its cards went with the round.
export function placeReachable(view, place) {
  const topic = view.topic;
  const anchor = place.anchor;
  if (!anchor) return true;
  if (anchor.kind === "decision") return findDecision(topic, anchor.key) !== null;
  const current = topic.rounds[topic.rounds.length - 1];
  const round =
    place.tab === "current"
      ? current?.number === place.currentRound
        ? current
        : null
      : shownPastRound(topic, place.pastRound);
  if (!round) return false;
  if (anchor.kind === "result") return round.questions.length === 0;
  return round.questions.some((question) => question.id === anchor.key);
}

export function arrivalsBetween(previous, next) {
  if (!previous) return [];
  const arrivals = [];
  const knownRounds = new Set(previous.topic.rounds.map(round => round.number));
  for (const round of next.topic.rounds) {
    if (knownRounds.has(round.number)) continue;
    arrivals.push({ id: `round:${round.number}`, kind: round.questions.length ? "round" : "result", round: round.number });
  }
  const replied = new Set(previous.topic.rounds.flatMap(round => round.asks.filter(ask => ask.state.status === "replied").map(ask => ask.id)));
  for (const round of next.topic.rounds) {
    for (const ask of round.asks) {
      if (ask.state.status !== "replied" || replied.has(ask.id)) continue;
      const question = round.questions.find(question => question.id === ask.question);
      arrivals.push({ id: `reply:${ask.id}`, kind: "reply", round: round.number, question: ask.question, ask: ask.id, text: question?.text ?? "" });
    }
  }
  return arrivals;
}

export function arrivalText(arrival, lang) {
  const characters = Array.from(arrival.text ?? "");
  return translator(lang)(`arrival.${arrival.kind}`, {
    n: arrival.round,
    question: characters.slice(0, 20).join("") + (characters.length > 20 ? "…" : ""),
  });
}
