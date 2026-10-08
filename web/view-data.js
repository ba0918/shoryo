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
    result: result ? resultData(view) : null,
    human: questions.filter((q) => q.cls === "human").map((q) => q.data),
    provisional: questions.filter((q) => q.cls === "provisional").map((q) => q.data),
    send: {
      round: round.number,
      locked,
      unstamped: locked ? 0 : view.unstamped.length,
      sent: round.submitted && !topic.ended,
      result,
      label: result ? (reviewing ? "send.review-requests" : "send.proceed") : "send.all",
      lang: view.lang,
    },
    fixes: round.review ? fixesData(view, round) : [],
  };
}

/// 結果: what a round without questions shows — the finished picture and the records.
function resultData(view) {
  const records = decisionsTabData(view);
  return {
    lang: view.lang,
    picture: view.topic.finished_picture,
    decisions: records.decisions,
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

function currentNumber(topic) {
  return topic.rounds.length > 0 ? topic.rounds[topic.rounds.length - 1].number : 0;
}

/// Whether 見直したい can be pressed or stopped for this decision now.
export function reviewState(view, id) {
  const topic = view.topic;
  const mark = topic.records.in_review.find((entry) => entry.decision === id);
  return {
    lang: view.lang,
    decision: id,
    inReview: mark !== undefined,
    canReview: mark === undefined && !topic.ended,
    canStop: mark !== undefined && !topic.ended && mark.since_round === currentNumber(topic),
  };
}

/// A decision for a list. `asDecided` shows the content it had when it was decided, marked
/// when it was revised later; otherwise its current content.
export function decisionItemData(view, decision, { asDecided = false } = {}) {
  const content = asDecided ? decision.history[0].content : decisionContent(decision);
  return {
    lang: view.lang,
    id: decision.id,
    name: content.name,
    text: content.text,
    source: decisionSource(view, decision),
    revised: asDecided && decision.history.length > 1,
    preApproved: view.pre_approved.includes(decision.id),
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

/// Every sent round, as it was answered: the same cards and provisional list as the current
/// round, read only, with what was sent and what was decided there.
export function pastRoundsData(view, selected, landed = null) {
  const topic = view.topic;
  const rounds = topic.rounds.filter((round) => round.submitted);
  const chosen = rounds.find((round) => round.number === selected) ?? rounds[rounds.length - 1];
  const questions = chosen
    ? chosen.questions.map((question) => ({
        cls: question.class,
        data: {
          ...questionData(view, chosen, question, true, landed, chosen.number),
          past: {
            chosen: question.answer.deferred ? null : question.options[question.answer.selected].text,
            recommended: question.options.find((option) => option.recommended)?.text ?? "",
            preApproved: question.answer.stamp === "pre_approved",
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
          fixes: fixesData(view, chosen),
          human: questions.filter((q) => q.cls === "human").map((q) => q.data),
          provisional: questions.filter((q) => q.cls === "provisional").map((q) => q.data),
        }
      : null,
  };
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
    rejected: records.rejected.map((rejected) => ({
      text: rejected.text,
      reason: rejected.reason,
      question: findQuestion(topic, rejected.question)?.question.text ?? rejected.question,
    })),
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
