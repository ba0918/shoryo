// 質問 under a card: the exchanges in the order they were asked, opened and closed under
// one heading, and the ask box (docs/spec/screen.md, "質問").
import { h } from "../dom.js";
import { explanation } from "./explanation.js";
import { translator } from "../strings.js";
import { shownExchanges } from "../view-data.js";

const SVG = "http://www.w3.org/2000/svg";

const QUICK_ASKS = ["thread.quick.more", "thread.quick.diagram", "thread.quick.recommendation"];

/// The heading, the exchanges and the ask box. `local` is the card's own state: `open` (null
/// until the person opens or closes the thread), `expanded` (the elided middle shown) and
/// `follows` (the ask a follow-up continues). `onLocal` changes it; `emit` sends asks upwards.
export function thread(question, local, onLocal, emit) {
  const t = translator(question.lang);
  const asks = question.asks;
  const open = local.open ?? asks.length > 0;
  return h(
    "div",
    { class: "thread" },
    asks.length > 0
      ? h(
          "button",
          {
            type: "button",
            class: "thread-toggle",
            "data-action": "thread-toggle",
            "data-focus": `thread-toggle-${question.id}`,
            "aria-expanded": String(open),
            onclick: () => onLocal({ open: !open }),
          },
          t("thread.heading", { count: asks.length }),
        )
      : null,
    open && asks.length > 0 ? exchanges(asks, local, onLocal, t, question) : null,
    question.locked ? null : askBox(question, local, onLocal, emit, t),
  );
}

function exchanges(asks, local, onLocal, t, question) {
  const { head, elided, tail } = shownExchanges(asks, local.expanded);
  return h(
    "div",
    { class: "exchanges", "data-exchanges": true },
    head.map((ask) => exchange(ask, local, onLocal, t, question)),
    elided > 0
      ? h(
          "button",
          { type: "button", class: "elided", "data-action": "show-all-exchanges", "data-focus": `exchanges-${asks[0].id}`, onclick: () => onLocal({ expanded: true }) },
          t("thread.elided", { count: elided }),
        )
      : null,
    tail.map((ask) => exchange(ask, local, onLocal, t, question)),
  );
}

function exchange(ask, local, onLocal, t, question) {
  return h(
    "div",
    { class: "exchange", "data-ask": ask.id, "data-focus-scope": `ask-${ask.id}` },
    ask.quote !== null
      ? h(
          "button",
          {
            type: "button",
            class: "quote",
            "data-quote": true,
            "data-focus": `quote-${ask.id}`,
            title: t("thread.go-to-quoted"),
            onclick: (event) => showReply(event.currentTarget, ask.follows, onLocal),
          },
          ask.quote,
        )
      : null,
    h("p", { class: "asked" }, h("span", { class: "speaker" }, t("thread.you")), h("span", { class: "asked-text" }, ask.text)),
    reply(ask, local, onLocal, t, question),
  );
}

/// Scrolls to the reply a follow-up continues, first showing the elided middle when it is there.
function showReply(from, id, onLocal) {
  const selector = `[data-ask="${CSS.escape(String(id))}"] [data-reply]`;
  const here = from.closest(".thread")?.querySelector(selector);
  if (here) return flash(here);
  onLocal({ expanded: true });
  const shown = [...document.querySelectorAll(selector)].find((element) => element.offsetParent !== null);
  if (shown) flash(shown);
}

function flash(element) {
  element.scrollIntoView({ block: "center" });
  element.classList.remove("flash");
  void element.offsetWidth;
  element.classList.add("flash");
}

function reply(ask, local, onLocal, t, question) {
  switch (ask.status) {
    case "waiting":
      return h(
        "p",
        { class: "reply pending typing", "data-mark": "writing" },
        h("span", { class: "typing-dots", "aria-hidden": "true" }, h("span", {}), h("span", {}), h("span", {})),
        h("span", { class: "visually-hidden" }, t("thread.writing")),
      );
    case "no_reply":
      return h("p", { class: "reply pending", "data-mark": "no-reply" }, t("thread.no-reply"));
    case "replied":
      return h(
        "div",
        { class: "reply", "data-reply": true },
        h("span", { class: "speaker" }, t("thread.llm")),
        explanation(ask.reply.parts, question.lang, `${question.round}-${question.id}-ask-${ask.id}`),
        local.canAsk
          ? h(
              "button",
              { type: "button", class: "link small", "data-action": "follow-up", "data-focus": `follow-up-${ask.id}`, onclick: () => onLocal({ follows: ask.id }) },
              t("thread.follow-up"),
            )
          : null,
      );
    default:
      return null;
  }
}

function sendIcon() {
  const svg = document.createElementNS(SVG, "svg");
  for (const [name, value] of Object.entries({ viewBox: "0 0 24 24", width: 18, height: 18, "aria-hidden": "true" })) {
    svg.setAttribute(name, String(value));
  }
  const path = document.createElementNS(SVG, "path");
  path.setAttribute("d", "M3.5 11.2 20.5 3.5l-7.7 17-2.4-6.9-6.9-2.4zM10.4 13.6l4.6-4.6");
  svg.append(path);
  return svg;
}

function askBox(question, local, onLocal, emit, t) {
  const following = question.asks.find((ask) => ask.id === local.follows);
  const send = (from) => {
    const input = from.closest(".ask-box").querySelector("textarea");
    const text = input.value;
    if (!text.trim()) return;
    input.value = "";
    emit({
      type: "op",
      op: { op: "ask", question: question.id, text, follows: local.follows ?? null },
    });
    onLocal({ follows: null, open: true });
  };
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
      send(event.currentTarget);
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
          h("span", { class: "following-text" }, following.excerpt ?? following.text),
          h(
            "button",
            { type: "button", class: "link small", "data-action": "cancel-follow-up", "data-focus": `cancel-follow-up-${question.id}`, onclick: () => onLocal({ follows: null }) },
            t("thread.cancel"),
          ),
        )
      : null,
    h(
      "div",
      { class: "quick-asks" },
      // A suggestion only fills the field: a mistaken press must not reach the LLM.
      QUICK_ASKS.map((key) =>
        h(
          "button",
          {
            type: "button",
            class: "chip",
            "data-action": "quick-ask",
            "data-focus": `quick-ask-${question.id}-${key}`,
            onclick: (event) => {
              const input = event.currentTarget.closest(".ask-box").querySelector("textarea");
              input.value = t(key);
              input.focus();
              input.setSelectionRange(input.value.length, input.value.length);
            },
          },
          t(key),
        ),
      ),
    ),
    h(
      "div",
      { class: "free-ask" },
      input,
      h(
        "button",
        {
          type: "button",
          class: "send-ask",
          "data-action": "ask",
          "data-focus": `send-ask-${question.id}`,
          "aria-label": t("thread.send"),
          title: t("thread.send"),
          // Keep the editor's selection intact when sending with the pointer.
          onmousedown: (event) => event.preventDefault(),
          onclick: (event) => send(event.currentTarget),
        },
        sendIcon(),
      ),
    ),
  );
}
