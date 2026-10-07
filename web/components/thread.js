// 聞き返し under a card: the three ways to ask, and the exchanges shown latest-only, all, or
// hidden (docs/spec/screen.md, "聞き返し").
import { h } from "../dom.js";
import { renderDiagram } from "../diagram.js";
import { translator } from "../strings.js";

const QUICK_ASKS = ["thread.quick.more", "thread.quick.diagram", "thread.quick.recommendation"];

const MODES = ["latest", "all", "hidden"];

/// The exchanges and the ask box. `local` is the card's own state: `mode` and `follows`
/// (the ask a follow-up continues). `onLocal` changes it; `emit` sends asks upwards.
export function thread(question, local, onLocal, emit) {
  const t = translator(question.lang);
  const asks = question.asks;
  return h(
    "div",
    { class: "thread" },
    asks.length > 0 ? modeSwitch(local, onLocal, t) : null,
    exchanges(asks, local, onLocal, t),
    question.locked ? null : askBox(question, local, onLocal, emit, t),
  );
}

function modeSwitch(local, onLocal, t) {
  return h(
    "div",
    { class: "thread-modes seg-group", role: "group", "aria-label": t("thread.modes") },
    MODES.map((mode) =>
      h(
        "button",
        {
          type: "button",
          class: `seg${mode === local.mode ? " on" : ""}`,
          "aria-pressed": String(mode === local.mode),
          "data-thread-mode": mode,
          onclick: () => onLocal({ mode }),
        },
        t(`thread.${mode}`),
      ),
    ),
  );
}

function exchanges(asks, local, onLocal, t) {
  if (local.mode === "hidden" || asks.length === 0) return null;
  if (local.mode === "latest") {
    return h("div", { class: "exchanges" }, exchange(asks[asks.length - 1], [], local, onLocal, t));
  }
  const ids = new Set(asks.map((ask) => ask.id));
  const children = (id) => asks.filter((ask) => ask.follows === id);
  const tree = (ask) => exchange(ask, children(ask.id).map(tree), local, onLocal, t);
  const roots = asks.filter((ask) => ask.follows === null || !ids.has(ask.follows));
  return h("div", { class: "exchanges" }, roots.map(tree));
}

function exchange(ask, followUps, local, onLocal, t) {
  return h(
    "div",
    { class: "exchange", "data-ask": ask.id },
    h("p", { class: "asked" }, h("span", { class: "speaker" }, t("thread.you")), ask.text),
    reply(ask, local, onLocal, t),
    followUps.length > 0 ? h("div", { class: "follow-ups" }, followUps) : null,
  );
}

function reply(ask, local, onLocal, t) {
  switch (ask.status) {
    case "waiting":
      return h("p", { class: "reply pending", "data-mark": "writing" }, t("thread.writing"));
    case "no_reply":
      return h("p", { class: "reply pending", "data-mark": "no-reply" }, t("thread.no-reply"));
    case "replied":
      return h(
        "div",
        { class: "reply" },
        h("span", { class: "speaker" }, t("thread.llm")),
        h("p", {}, ask.reply.text),
        ask.reply.diagram ? h("div", { class: "diagram-box" }, renderDiagram(ask.reply.diagram)) : null,
        local.canAsk
          ? h(
              "button",
              { type: "button", class: "link small", "data-action": "follow-up", onclick: () => onLocal({ follows: ask.id }) },
              t("thread.follow-up"),
            )
          : null,
      );
    default:
      return null;
  }
}

function askBox(question, local, onLocal, emit, t) {
  const send = (text) => {
    if (!text.trim()) return;
    emit({
      type: "op",
      op: { op: "ask", question: question.id, text, follows: local.follows ?? null },
    });
    onLocal({ follows: null });
  };
  // An emptied field holds nothing to lose, so the card can redraw at once.
  const sendTyped = () => {
    const text = input.value;
    if (!text.trim()) return;
    input.value = "";
    send(text);
  };
  const following = question.asks.find((ask) => ask.id === local.follows);
  // Enter sends and Shift+Enter starts a new line; Enter that confirms an input method's
  // conversion belongs to the input method.
  const input = h("textarea", {
    rows: 1,
    "data-field": "ask",
    "aria-label": t("thread.ask-placeholder"),
    placeholder: following ? t("thread.follow-up-placeholder") : t("thread.ask-placeholder"),
    "data-focus": `ask-${question.id}`,
    onkeydown: (event) => {
      if (event.key !== "Enter" || event.shiftKey || event.isComposing || event.keyCode === 229) return;
      event.preventDefault();
      sendTyped();
    },
  });
  return h(
    "div",
    { class: "ask-box" },
    following
      ? h(
          "p",
          { class: "following" },
          h("span", { class: "following-label" }, t("thread.following")),
          h("span", { class: "following-text" }, following.text),
          h(
            "button",
            { type: "button", class: "link small", "data-action": "cancel-follow-up", onclick: () => onLocal({ follows: null }) },
            t("thread.cancel"),
          ),
        )
      : null,
    h(
      "div",
      { class: "quick-asks" },
      QUICK_ASKS.map((key) =>
        h("button", { type: "button", class: "chip", "data-action": "quick-ask", onclick: () => send(t(key)) }, t(key)),
      ),
    ),
    h(
      "div",
      { class: "free-ask" },
      input,
      h("button", { type: "button", class: "btn small", "data-action": "ask", onclick: sendTyped }, t("thread.ask")),
    ),
  );
}
