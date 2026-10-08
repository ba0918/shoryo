// The current round's tab: the cards, the provisional list and sending, in that order
// (docs/spec/screen.md, "並び").
import { Component, KeyedList, h } from "../dom.js";
import { translator } from "../strings.js";
import { Card } from "./card.js";
import { ProvisionalRow } from "./provisional.js";
import { Fixes } from "./fixes.js";
import { ResultView } from "./result.js";

class SendBar extends Component {
  draw(bar) {
    const t = translator(bar.lang);
    const status = bar.unstamped > 0
        ? h(
            "button",
            {
              type: "button",
              class: "send-status warn link-like",
              "data-unstamped-count": true,
              title: t("send.go-unstamped"),
              onclick: () => this.emit({ type: "go-unstamped" }),
            },
            t("send.unstamped", { count: bar.unstamped }),
          )
        : h("p", { class: "send-status" }, bar.locked || bar.result ? "" : t("send.ready"));
    return h(
      "div",
      { class: "send-bar" },
      status,
      h(
        "button",
        {
          type: "button",
          class: "btn primary",
          "data-action": "send",
          disabled: bar.locked || bar.unstamped > 0,
          onclick: () => this.emit({ type: "confirm-send" }),
        },
        t(bar.label),
      ),
    );
  }
}

export class CurrentRound {
  revealReply(question) {
    (this.cards.items.get(question) ?? this.rows.items.get(question))?.revealReply();
  }

  constructor(emit) {
    this.empty = h("p", { class: "empty" });
    this.cardBox = h("div", { class: "cards" });
    this.rowBox = h("div", { class: "provisional-rows" });
    this.list = h(
      "section",
      { class: "provisional-list", "data-provisional-list": true },
      h(
        "header",
        { class: "list-head" },
        (this.listTitle = h("h2")),
        (this.listHint = h("p", { class: "list-hint" })),
      ),
      this.rowBox,
    );
    this.send = new SendBar(emit);
    this.result = new ResultView(emit);
    this.fixes = new Fixes(emit);
    this.cards = new KeyedList(this.cardBox, () => new Card(emit));
    this.rows = new KeyedList(this.rowBox, () => new ProvisionalRow(emit));
    this.el = h("div", { class: "current-round" }, this.empty, this.fixes.el, this.result.el, this.cardBox, this.list, this.send.el);
  }

  /// `round` is null before the first round; `lang` is the screen's language.
  update(round, lang) {
    const t = translator(lang);
    this.empty.textContent = t("current.waiting");
    this.listTitle.textContent = t("provisional.title");
    this.listHint.textContent = t("provisional.hint");
    this.empty.hidden = round !== null;
    this.list.hidden = round === null || round.provisional.length === 0;
    this.fixes.update({ fixes: round?.fixes ?? [], lang });
    this.result.update(round?.result ?? null);
    this.cards.update((round?.human ?? []).map((q) => ({ key: q.id, data: q })));
    this.rows.update((round?.provisional ?? []).map((q) => ({ key: q.id, data: q })));
    this.send.update(round ? round.send : { round: 0, locked: true, unstamped: 0, label: "send.all", lang });
    this.send.el.hidden = !round?.showSend;
  }
}
