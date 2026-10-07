// The current round's tab: the cards, the provisional list and sending, in that order
// (docs/spec/screen.md, "並び").
import { Component, KeyedList, h } from "../dom.js";
import { Card } from "./card.js";
import { ProvisionalRow } from "./provisional.js";

class SendBar extends Component {
  draw(bar) {
    return h(
      "div",
      { class: "send-bar" },
      h(
        "button",
        {
          type: "button",
          class: "primary",
          "data-action": "send",
          disabled: bar.locked,
          onclick: () => this.emit({ type: "op", op: { op: "submit" } }),
        },
        "Send all",
      ),
      bar.unopened > 0
        ? h("span", { class: "unopened-count", "data-unopened-count": true }, `${bar.unopened} not opened`)
        : null,
      bar.sent ? h("p", { class: "sent-notice", "data-sent-notice": true }, "Sent. Waiting for the next round.") : null,
    );
  }
}

export class CurrentRound {
  constructor(emit) {
    this.empty = h("p", { class: "empty" }, "Waiting for the first round.");
    this.cardBox = h("div", { class: "cards" });
    this.rowBox = h("div", { class: "provisional-rows" });
    this.list = h(
      "section",
      { class: "provisional-list", "data-provisional-list": true },
      h("h2", {}, "Provisional answers"),
      this.rowBox,
    );
    this.send = new SendBar(emit);
    this.cards = new KeyedList(this.cardBox, () => new Card(emit));
    this.rows = new KeyedList(this.rowBox, () => new ProvisionalRow(emit));
    this.el = h("div", { class: "current-round" }, this.empty, this.cardBox, this.list, this.send.el);
  }

  /// `round` is null before the first round.
  update(round) {
    this.empty.hidden = round !== null;
    this.list.hidden = round === null || round.provisional.length === 0;
    this.cards.update((round?.human ?? []).map((q) => ({ key: q.id, data: q })));
    this.rows.update((round?.provisional ?? []).map((q) => ({ key: q.id, data: q })));
    this.send.update(round ? round.send : { locked: true, unopened: 0, sent: false });
    this.send.el.hidden = round === null;
  }
}
