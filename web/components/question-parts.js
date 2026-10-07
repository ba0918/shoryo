// Pieces shared by the question card and the opened provisional row.
import { h } from "../dom.js";
import { decisionItem } from "./decision-item.js";

/// 前提の行: each direct prerequisite's chain of short names, side by side.
export function chainLine(chains, emit) {
  if (chains.length === 0) return null;
  return h(
    "div",
    { class: "chain", "data-chain": true },
    h("span", { class: "label" }, "Rests on: "),
    chains.map((chain, index) => [
      index > 0 ? h("span", { class: "chain-gap" }, " / ") : null,
      chain.map((link, position) => [
        position > 0 ? h("span", { class: "chain-arrow" }, " ← ") : null,
        h(
          "button",
          {
            type: "button",
            class: "chain-link",
            "data-decision": link.decision,
            onclick: () => emit({ type: "show-decision", decision: link.decision }),
          },
          link.name,
          link.in_review ? h("span", { class: "mark review", "data-mark": "in-review" }, "In review") : null,
        ),
      ]),
    ]),
  );
}

/// The options as radio buttons, with the recommendation marked.
export function optionList(question, emit, open) {
  return h(
    "fieldset",
    { class: "options", disabled: question.locked },
    question.options.map((option) =>
      h(
        "label",
        { class: "option" },
        h("input", {
          type: "radio",
          name: `option-${question.id}`,
          value: String(option.index),
          checked: option.index === question.selected,
          "data-focus": `option-${question.id}-${option.index}`,
          onchange: () =>
            emit({ type: "op", op: { op: "choose", question: question.id, option: option.index } }),
        }),
        h("span", { class: "option-text" }, option.text),
        option.recommended ? h("span", { class: "mark recommended", "data-mark": "recommended" }, "Recommended") : null,
        open ? h("p", { class: "option-description" }, option.description) : null,
      ),
    ),
  );
}

/// この答えだと: the consequence of the option chosen now.
export function consequence(question) {
  const chosen = question.options.find((option) => option.index === question.selected);
  return h("p", { class: "consequence", "data-consequence": true }, chosen?.consequence ?? "");
}

/// The prerequisites' full text and the background, shown once the card is open.
export function details(question) {
  return [
    question.premises.length > 0
      ? h(
          "div",
          { class: "premise-texts", "data-premise-text": true },
          h("h4", {}, "Prerequisites"),
          question.premises.map((premise) =>
            h("p", {}, h("strong", {}, premise.name), ": ", premise.text),
          ),
        )
      : null,
    question.background
      ? h("div", { class: "background" }, h("h4", {}, "Background"), h("p", {}, question.background))
      : null,
  ];
}

/// The defer switch and the note.
export function answerExtras(question, emit) {
  const save = noteSaver(question, emit);
  return h(
    "div",
    { class: "extras" },
    h(
      "label",
      { class: "defer", "data-field": "defer" },
      h("input", {
        type: "checkbox",
        checked: question.deferred,
        disabled: question.locked,
        "data-focus": `defer-${question.id}`,
        onchange: (event) =>
          emit({
            type: "op",
            op: { op: "defer", question: question.id, deferred: event.target.checked },
          }),
      }),
      " Ask me again next round",
    ),
    h(
      "label",
      { class: "note", "data-field": "note" },
      h("span", {}, "Note"),
      h("textarea", {
        rows: 2,
        value: question.note,
        disabled: question.locked,
        "data-focus": `note-${question.id}`,
        oninput: (event) => save.later(event.target.value),
        onchange: (event) => save.now(event.target.value),
      }),
    ),
  );
}

/// How long typing pauses before the note is saved, so a closed page keeps what was typed.
const NOTE_PAUSE_MS = 400;

function noteSaver(question, emit) {
  let timer = null;
  let saved = question.note;
  const now = (text) => {
    clearTimeout(timer);
    if (text === saved) return;
    saved = text;
    emit({ type: "op", op: { op: "note", question: question.id, text } });
  };
  const later = (text) => {
    clearTimeout(timer);
    timer = setTimeout(() => now(text), NOTE_PAUSE_MS);
  };
  return { now, later };
}

/// The marks a sent question carries in a past round.
export function pastMarks(question) {
  if (!question.past) return null;
  return [
    question.past.sentUnseen
      ? h("span", { class: "mark unopened", "data-mark": "sent-unseen" }, "Sent without opening")
      : null,
    question.deferred ? h("span", { class: "mark deferred", "data-mark": "deferred" }, "Ask me again next round") : null,
  ];
}

/// What a past round adds under a question: the answer sent, the recommendation, and the
/// decisions made from it, as they were then.
export function pastAnswer(question, emit) {
  const past = question.past;
  if (!past) return null;
  return h(
    "div",
    { class: "past-answer" },
    h("p", { "data-chosen": true }, h("span", { class: "label" }, "Answer: "), past.chosen ?? "(deferred)"),
    h("p", { "data-recommended": true }, h("span", { class: "label" }, "Recommended: "), past.recommended),
    past.decisions.length > 0
      ? h(
          "div",
          { class: "decided-here" },
          h("h4", {}, "Decided here"),
          past.decisions.map((item) => decisionItem(item, emit)),
        )
      : null,
  );
}
