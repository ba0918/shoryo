// Pieces shared by the question card and the provisional row: the small set of components
// every question is drawn from.
import { h } from "../dom.js";
import { translator } from "../strings.js";
import { decisionItem } from "./decision-item.js";
import { explanation } from "./explanation.js";

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
export function optionList(question, emit, open, codeParts) {
  const t = translator(question.lang);
  return h(
    "fieldset",
    { class: "options" },
    h("legend", { class: "visually-hidden" }, t("card.options")),
    question.options.map((option) => {
      const chosen = option.index === question.selected;
      return h(
        "div",
        { class: `option${chosen ? " chosen" : ""}` },
        h("label", { class: "option-choice" }, h("input", {
          type: "radio",
          name: `option-${question.id}`,
          value: String(option.index),
          checked: chosen,
          disabled: question.locked,
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
        ),
        ),
        open ? h("div", { class: "option-description" }, explanation(option.description, question.lang, `${question.round}-${question.id}-option-${option.index}`, emit, codeParts)) : null,
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
export function details(question, emit, codeParts) {
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
    question.background.length > 0
      ? h("div", { class: "context-part" }, h("h4", {}, t("card.background")), explanation(question.background, question.lang, `${question.round}-${question.id}-background`, emit, codeParts))
      : null,
  ].filter(Boolean);
  return parts.length > 0 ? h("div", { class: "context" }, parts) : null;
}

/// The note sent with the answer, labelled as such, under the answer. A past question shows
/// it only when it was sent with one.
export function noteField(question, emit) {
  if (question.past && question.note === "") return null;
  const t = translator(question.lang);
  return h(
    "label",
    { class: "note", "data-field": "note" },
    h("span", { class: "note-label" }, t("card.note")),
    h("textarea", {
      rows: 1,
      value: question.note,
      disabled: question.locked,
      "data-focus": `note-${question.id}`,
      oninput: (event) => noteSaver(event.target, question, emit).later(event.target.value),
      onchange: (event) => noteSaver(event.target, question, emit).now(event.target.value),
    }),
  );
}

/// 先送りのラベル. A past question has none; its deferral is one of its marks.
export function deferSwitch(question, emit) {
  if (question.past) return null;
  const t = translator(question.lang);
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
// A redraw must not give a retained editor a second saver whose timer cannot cancel the first.
const noteSavers = new WeakMap();

function noteSaver(field, question, emit) {
  const existing = noteSavers.get(field);
  if (existing) return existing;
  let timer = null;
  const now = (text) => {
    clearTimeout(timer);
    // Last-sent text is not authoritative: another tab may have stored a different note.
    emit({ type: "op", op: { op: "note", question: question.id, text } });
  };
  const later = (text) => {
    clearTimeout(timer);
    timer = setTimeout(() => now(text), NOTE_PAUSE_MS);
  };
  const saver = { now, later };
  noteSavers.set(field, saver);
  return saver;
}

/// 判子: pressed by the person, or pre-approved by the LLM; pressing a pressed stamp lifts it.
/// A dated stamp shows its month and day, and the year and time on hover and focus. A stamp
/// that can no longer be pressed shows them on a tap instead: `timeShown` and `toggleTime`
/// belong to the card or row that holds it.
export function stampButton(question, emit, { timeShown = false, toggleTime = () => {} } = {}) {
  const stamped = question.stamp !== null;
  const t = translator(question.lang);
  const at = question.stampedAt ? new Date(question.stampedAt) : null;
  const where = question.past ? "past-" : "";
  const tip = `${where}stamp-time-${question.id}`;
  return h(
    "span",
    { class: `stamp-holder${timeShown && at ? " time-shown" : ""}` },
    h(
      "button",
      {
        type: "button",
        class: `stamp ${question.stamp ?? "none"}${question.locked ? " fixed" : ""}`,
        "data-action": "stamp",
        "data-focus": `${where}stamp-${question.id}`,
        "data-stamp": question.stamp ?? "none",
        "aria-pressed": String(stamped),
        // Neither `disabled` nor `aria-disabled`: a stamp that can no longer be pressed still
        // takes focus and taps, which show when it was pressed.
        "aria-describedby": at ? tip : undefined,
        onclick: () =>
          question.locked
            ? toggleTime()
            : emit({ type: "op", op: { op: "stamp", question: question.id, stamped: !stamped } }),
      },
      h(
        "span",
        { class: "stamp-face" },
        t(`stamp.${question.stamp ?? "none"}`),
        at ? h("span", { class: "stamp-date", "data-stamp-date": true }, `${at.getMonth() + 1}/${at.getDate()}`) : null,
      ),
    ),
    at ? h("span", { class: "stamp-time", role: "tooltip", id: tip, "data-stamp-time": true }, stampTime(at, question.lang)) : null,
  );
}

/// The year, date and time of a stamp, in the browser's time zone.
function stampTime(at, lang) {
  return at.toLocaleString(lang === "ja" ? "ja-JP" : "en-US", {
    year: "numeric",
    month: "numeric",
    day: "numeric",
    hour: "numeric",
    minute: "2-digit",
    hourCycle: "h23",
  });
}

/// The marks a sent question carries in a past round.
export function pastMarks(question) {
  if (!question.past) return null;
  const t = translator(question.lang);
  return [
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
