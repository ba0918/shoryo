// Turns the server's view into the read-only data each component draws. Decisions about
// what is shown are made here, once, not inside the components.

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
export function decisionSource(topic, decision) {
  if (decision.origin.question !== undefined) {
    return findQuestion(topic, decision.origin.question)?.question.text ?? decision.origin.question;
  }
  return `Fixed in round ${decision.origin.fix_round}`;
}

export function questionData(view, round, question, locked, landed = null) {
  const topic = view.topic;
  return {
    id: question.id,
    text: question.text,
    why_now: question.why_now,
    background: question.background,
    chains: view.chains[question.id] ?? [],
    premises: question.premises.map((id) => {
      const decision = findDecision(topic, id);
      const content = decision ? decisionContent(decision) : null;
      return { id, name: content?.name ?? id, text: content?.text ?? "" };
    }),
    options: question.options.map((option, index) => ({ index, ...option })),
    selected: question.answer.selected,
    note: question.answer.note,
    deferred: question.answer.deferred,
    unopened: view.unopened.includes(question.id),
    asks: round.asks.filter((ask) => ask.question === question.id).map(askData),
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
  return {
    human: questions.filter((q) => q.cls === "human").map((q) => q.data),
    provisional: questions.filter((q) => q.cls === "provisional").map((q) => q.data),
    send: {
      round: round.number,
      locked,
      unopened: locked ? 0 : view.unopened.length,
      sent: round.submitted && !topic.ended,
    },
    fixes: round.review ? fixesData(view, round) : [],
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
    decision: id,
    inReview: mark !== undefined,
    canReview: mark === undefined && !topic.ended,
    canStop: mark !== undefined && !topic.ended && mark.since_round === currentNumber(topic),
  };
}

/// 見ずに送った: the answer the decision came from was sent without being opened.
function fromUnseenAnswer(topic, decision) {
  if (decision.origin.question === undefined) return false;
  return findQuestion(topic, decision.origin.question)?.question.answer.sent_unseen ?? false;
}

/// A decision for a list. `asDecided` shows the content it had when it was decided, marked
/// when it was revised later; otherwise its current content.
export function decisionItemData(view, decision, { asDecided = false } = {}) {
  const content = asDecided ? decision.history[0].content : decisionContent(decision);
  return {
    id: decision.id,
    name: content.name,
    text: content.text,
    source: decisionSource(view.topic, decision),
    revised: asDecided && decision.history.length > 1,
    sentUnseen: fromUnseenAnswer(view.topic, decision),
    review: reviewState(view, decision.id),
    jump: decision.origin.question !== undefined ? questionTarget(view.topic, decision.origin.question) : null,
  };
}

function decidedBy(topic, questionId) {
  return topic.records.decisions.filter((decision) => decision.origin.question === questionId);
}

function askData(ask) {
  return {
    id: ask.id,
    text: ask.text,
    follows: ask.follows,
    status: ask.state.status,
    reply: ask.state.status === "replied" ? { text: ask.state.text, diagram: ask.state.diagram } : null,
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

/// Every sent round, as it was answered.
export function pastRoundsData(view, selected, landed = null) {
  const topic = view.topic;
  const rounds = topic.rounds.filter((round) => round.submitted);
  const chosen = rounds.find((round) => round.number === selected) ?? rounds[rounds.length - 1];
  return {
    choices: rounds.map((round) => ({ number: round.number, subject: round.subject })),
    selected: chosen?.number ?? null,
    highlight: landed,
    round: chosen
      ? {
          number: chosen.number,
          subject: chosen.subject,
          fixes: fixesData(view, chosen),
          questions: chosen.questions.map((question) => ({
            id: question.id,
            text: question.text,
            human: question.class === "human",
            chosen: question.answer.deferred ? null : question.options[question.answer.selected].text,
            recommended: question.options.find((option) => option.recommended)?.text ?? "",
            note: question.answer.note,
            deferred: question.answer.deferred,
            sentUnseen: question.answer.sent_unseen,
            asks: chosen.asks.filter((ask) => ask.question === question.id).map(askData),
            decisions: decidedBy(topic, question.id).map((decision) =>
              decisionItemData(view, decision, { asDecided: true }),
            ),
          })),
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

function answerText(question, round) {
  if (!round.submitted) return null;
  if (question.answer.deferred) return "(asked again next round)";
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
        answer: found ? answerText(found.question, found.round) : null,
      };
    }
    case "rejected": {
      const rejected = topic.records.rejected[Number(node.key.slice(2))];
      return { text: rejected ? `Rejected: ${rejected.reason}` : "", question: node.question, answer: null };
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

/// The map as drawn: the range, the selected point, and only the points the range shows.
export function mapData(view, range, selected) {
  const map = view.map;
  const exists = map.nodes.some((node) => node.key === selected);
  const chosen = exists ? selected : null;
  const shown = range === "path" && chosen ? new Set(view.paths[chosen]) : null;
  const visible = (key) => shown === null || shown.has(key);
  const nodes = new Map(map.nodes.map((node) => [node.key, node]));
  return {
    range,
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
        };
      }),
    })),
    edges: map.edges.filter((edge) => visible(edge.from) && visible(edge.to)),
  };
}
