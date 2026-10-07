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

export function questionData(view, question, locked) {
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
    locked,
  };
}

export function currentRoundData(view) {
  const topic = view.topic;
  const round = topic.rounds[topic.rounds.length - 1];
  if (!round) return null;
  const locked = round.submitted || topic.ended;
  const questions = round.questions.map((question) => ({
    cls: question.class,
    data: questionData(view, question, locked),
  }));
  return {
    human: questions.filter((q) => q.cls === "human").map((q) => q.data),
    provisional: questions.filter((q) => q.cls === "provisional").map((q) => q.data),
    send: { locked, unopened: locked ? 0 : view.unopened.length, sent: round.submitted },
  };
}

export function decisionDetail(view, id) {
  const decision = findDecision(view.topic, id);
  if (!decision) return null;
  const content = decisionContent(decision);
  return { id, name: content.name, text: content.text, source: decisionSource(view.topic, decision) };
}
