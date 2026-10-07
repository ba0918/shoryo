// Pieces shared by the question card and the provisional row: the small set of components
// every question is drawn from.
import { h } from "../dom.js";
import { translator } from "../strings.js";
import { decisionItem } from "./decision-item.js";

/// 前提の行: each direct prerequisite's chain of short names, side by side.
export function chainLine(question, emit) {
  const chains = question.chains;
  if (chains.length === 0) return null;
  const t = translator(question.lang);
  return h(
    "div",
    { class: "chain", "data-chain": true },
    h("span", { class: "chain-label" }, t("card.rests-on")),
    chains.map((chain, index) => [
      index > 0 ? h("span", { class: "chain-gap", "aria-hidden": "true" }, "/") : null,
      h(
        "span",
        { class: "chain-run" },
        chain.map((link, position) => [
          position > 0 ? h("span", { class: "chain-arrow", "aria-hidden": "true" }, "←") : null,
          h(
            "button",
            {
              type: "button",
              class: `chip chain-link${link.in_review ? " in-review" : ""}`,
              "data-decision": link.decision,
              onclick: () => emit({ type: "show-decision", decision: link.decision }),
            },
            link.name,
            link.in_review ? h("span", { class: "badge review", "data-mark": "in-review" }, t("card.in-review")) : null,
          ),
        ]),
      ),
    ]),
  );
}

/// The options as bordered rows: the chosen row tinted, the recommendation tagged, and each
/// option's description once the card is open.
export function optionList(question, emit, open) {
  const t = translator(question.lang);
  return h(
    "fieldset",
    { class: "options", disabled: question.locked },
    h("legend", { class: "visually-hidden" }, t("card.options")),
    question.options.map((option) => {
      const chosen = option.index === question.selected;
      return h(
        "label",
        { class: `option${chosen ? " chosen" : ""}` },
        h("input", {
          type: "radio",
          name: `option-${question.id}`,
          value: String(option.index),
          checked: chosen,
          "data-focus": `option-${question.id}-${option.index}`,
          onchange: () => emit({ type: "op", op: { op: "choose", question: question.id, option: option.index } }),
        }),
        h(
          "span",
          { class: "option-body" },
          h(
            "span",
            { class: "option-head" },
            h("span", { class: "option-text" }, option.text),
            option.recommended ? h("span", { class: "badge recommended", "data-mark": "recommended" }, t("card.recommended")) : null,
          ),
          open ? h("span", { class: "option-description" }, option.description) : null,
        ),
      );
    }),
  );
}

/// The options as one compact segmented choice, for a folded provisional row.
export function optionSegments(question, emit) {
  const t = translator(question.lang);
  return h(
    "fieldset",
    { class: "segments", disabled: question.locked },
    h("legend", { class: "visually-hidden" }, t("card.options")),
    question.options.map((option) =>
      h(
        "label",
        { class: `segment${option.index === question.selected ? " chosen" : ""}` },
        h("input", {
          type: "radio",
          name: `option-${question.id}`,
          value: String(option.index),
          checked: option.index === question.selected,
          "data-focus": `segment-${question.id}-${option.index}`,
          onchange: () => emit({ type: "op", op: { op: "choose", question: question.id, option: option.index } }),
        }),
        h("span", {}, option.text),
        option.recommended ? h("span", { class: "badge recommended", "data-mark": "recommended" }, t("card.recommended")) : null,
      ),
    ),
  );
}

/// この答えだと: the consequence of the option chosen now, as a tinted band.
export function consequence(question) {
  const chosen = question.options.find((option) => option.index === question.selected);
  const t = translator(question.lang);
  return h(
    "div",
    { class: "consequence" },
    h("span", { class: "consequence-head" }, t("card.consequence")),
    h("p", { "data-consequence": true }, chosen?.consequence ?? ""),
  );
}

/// The prerequisites' full text and the background, shown once the card is open.
export function details(question) {
  const t = translator(question.lang);
  const parts = [
    question.premises.length > 0
      ? h(
          "div",
          { class: "context-part", "data-premise-text": true },
          h("h4", {}, t("card.prerequisites")),
          h(
            "dl",
            {},
            question.premises.map((premise) => [h("dt", {}, premise.name), h("dd", {}, premise.text)]),
          ),
        )
      : null,
    question.background
      ? h("div", { class: "context-part" }, h("h4", {}, t("card.background")), h("p", {}, question.background))
      : null,
  ].filter(Boolean);
  return parts.length > 0 ? h("div", { class: "context" }, parts) : null;
}

/// The note and the defer switch: compact, under the answer. A past question shows only the
/// note it was sent with; its defer switch is one of its marks.
export function answerExtras(question, emit) {
  const save = noteSaver(question, emit);
  if (question.past && question.note === "") return null;
  const t = translator(question.lang);
  return h(
    "div",
    { class: "extras" },
    h(
      "label",
      { class: "note", "data-field": "note" },
      h("span", { class: "visually-hidden" }, t("card.note")),
      h("textarea", {
        rows: 1,
        value: question.note,
        placeholder: t("card.note-placeholder"),
        disabled: question.locked,
        "data-focus": `note-${question.id}`,
        oninput: (event) => save.later(event.target.value),
        onchange: (event) => save.now(event.target.value),
      }),
    ),
    question.past ? null : deferSwitch(question, emit, t),
  );
}

function deferSwitch(question, emit, t) {
  return h(
    "label",
    { class: "toggle defer", "data-field": "defer" },
    h("input", {
      type: "checkbox",
      role: "switch",
      checked: question.deferred,
      disabled: question.locked,
      "data-focus": `defer-${question.id}`,
      onchange: (event) => emit({ type: "op", op: { op: "defer", question: question.id, deferred: event.target.checked } }),
    }),
    h("span", { class: "toggle-track", "aria-hidden": "true" }),
    h("span", {}, t("card.defer")),
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

/// 判子: pressed by the person, or pre-approved by the LLM; pressing a pressed stamp lifts it.
export function stampButton(question, emit) {
  const stamped = question.stamp !== null;
  const t = translator(question.lang);
  return h(
    "button",
    {
      type: "button",
      class: `stamp ${question.stamp ?? "none"}`,
      "data-action": "stamp",
      "data-focus": `stamp-${question.id}`,
      "data-stamp": question.stamp ?? "none",
      "aria-pressed": String(stamped),
      disabled: question.locked,
      onclick: () => emit({ type: "op", op: { op: "stamp", question: question.id, stamped: !stamped } }),
    },
    h("span", { class: "stamp-face" }, t(`stamp.${question.stamp ?? "none"}`)),
  );
}

/// The marks a sent question carries in a past round.
export function pastMarks(question) {
  if (!question.past) return null;
  const t = translator(question.lang);
  return [
    question.past.preApproved ? h("span", { class: "badge pre-approved", "data-mark": "pre-approved" }, t("card.pre-approved")) : null,
    question.deferred ? h("span", { class: "badge deferred", "data-mark": "deferred" }, t("card.defer")) : null,
  ];
}

/// What a past round adds under a question: the answer sent, the recommendation, and the
/// decisions made from it, as they were then.
export function pastAnswer(question, emit) {
  const past = question.past;
  if (!past) return null;
  const t = translator(question.lang);
  return h(
    "div",
    { class: "past-answer" },
    h(
      "dl",
      { class: "answer-facts" },
      h("div", { "data-chosen": true }, h("dt", {}, t("card.answer")), h("dd", {}, past.chosen ?? t("card.deferred-answer"))),
      h("div", { "data-recommended": true }, h("dt", {}, t("card.recommended")), h("dd", {}, past.recommended)),
    ),
    past.decisions.length > 0
      ? h(
          "div",
          { class: "decided-here" },
          h("h4", {}, t("card.decided-here")),
          past.decisions.map((item) => decisionItem(item, emit)),
        )
      : null,
  );
}
