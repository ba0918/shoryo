// 聞き返し under a card: the three ways to ask, and the exchanges shown latest-only, all, or
// hidden (docs/spec/screen.md, "聞き返し").
import { h } from "../dom.js";
import { renderDiagram } from "../diagram.js";

export const QUICK_ASKS = ["Explain more", "Simplify with a diagram", "What does the recommendation mean?"];

const MODES = [
  { id: "latest", name: "Latest only" },
  { id: "all", name: "All" },
  { id: "hidden", name: "Hide" },
];

/// The exchanges and the ask box. `local` is the card's own state: `mode` and `follows`
/// (the ask a follow-up continues). `onLocal` changes it; `emit` sends asks upwards.
export function thread(question, local, onLocal, emit) {
  const asks = question.asks;
  return h(
    "div",
    { class: "thread" },
    asks.length > 0 ? modeSwitch(local, onLocal) : null,
    exchanges(asks, local, onLocal),
    question.locked ? null : askBox(question, local, onLocal, emit),
  );
}

function modeSwitch(local, onLocal) {
  return h(
    "div",
    { class: "thread-modes" },
    MODES.map((mode) =>
      h(
        "button",
        {
          type: "button",
          class: mode.id === local.mode ? "selected" : "",
          "aria-pressed": String(mode.id === local.mode),
          "data-thread-mode": mode.id,
          onclick: () => onLocal({ mode: mode.id }),
        },
        mode.name,
      ),
    ),
  );
}

function exchanges(asks, local, onLocal) {
  if (local.mode === "hidden" || asks.length === 0) return null;
  if (local.mode === "latest") {
    return h("div", { class: "exchanges" }, exchange(asks[asks.length - 1], [], local, onLocal));
  }
  const ids = new Set(asks.map((ask) => ask.id));
  const children = (id) => asks.filter((ask) => ask.follows === id);
  const tree = (ask) => exchange(ask, children(ask.id).map(tree), local, onLocal);
  const roots = asks.filter((ask) => ask.follows === null || !ids.has(ask.follows));
  return h("div", { class: "exchanges" }, roots.map(tree));
}

function exchange(ask, followUps, local, onLocal) {
  return h(
    "div",
    { class: "exchange", "data-ask": ask.id },
    h("p", { class: "asked" }, h("span", { class: "label" }, "You asked: "), ask.text),
    reply(ask, local, onLocal),
    followUps.length > 0 ? h("div", { class: "follow-ups" }, followUps) : null,
  );
}

function reply(ask, local, onLocal) {
  switch (ask.status) {
    case "waiting":
      return h("p", { class: "mark writing", "data-mark": "writing" }, "Writing a reply...");
    case "no_reply":
      return h("p", { class: "mark no-reply", "data-mark": "no-reply" }, "No reply");
    case "replied":
      return h(
        "div",
        { class: "reply" },
        h("p", {}, ask.reply.text),
        ask.reply.diagram ? h("div", { class: "diagram-box" }, renderDiagram(ask.reply.diagram)) : null,
        local.canAsk
          ? h(
              "button",
              { type: "button", "data-action": "follow-up", onclick: () => onLocal({ follows: ask.id }) },
              "Follow up",
            )
          : null,
      );
    default:
      return null;
  }
}

function askBox(question, local, onLocal, emit) {
  const send = (text) => {
    if (!text.trim()) return;
    emit({
      type: "op",
      op: { op: "ask", question: question.id, text, follows: local.follows ?? null },
    });
    onLocal({ follows: null });
  };
  const following = question.asks.find((ask) => ask.id === local.follows);
  const input = h("input", {
    type: "text",
    "data-field": "ask",
    "aria-label": "Ask about this question",
    placeholder: following ? "Your follow-up" : "Ask about this question",
    "data-focus": `ask-${question.id}`,
    onkeydown: (event) => {
      if (event.key === "Enter") send(event.target.value);
    },
  });
  return h(
    "div",
    { class: "ask-box" },
    following
      ? h(
          "p",
          { class: "following" },
          h("span", { class: "label" }, "Following up on: "),
          following.text,
          " ",
          h("button", { type: "button", "data-action": "cancel-follow-up", onclick: () => onLocal({ follows: null }) }, "Cancel"),
        )
      : null,
    h(
      "div",
      { class: "quick-asks" },
      QUICK_ASKS.map((text) =>
        h("button", { type: "button", "data-action": "quick-ask", onclick: () => send(text) }, text),
      ),
    ),
    h(
      "div",
      { class: "free-ask" },
      input,
      h("button", { type: "button", "data-action": "ask", onclick: () => send(input.value) }, "Ask"),
    ),
  );
}
