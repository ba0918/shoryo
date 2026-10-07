// 過去のラウンド: a sent round as it was answered, drawn with the same cards and provisional
// list as the current round, read only (docs/spec/screen.md, "過去のラウンド").
import { Component, KeyedList, h } from "../dom.js";
import { Card } from "./card.js";
import { Fixes } from "./fixes.js";
import { ProvisionalRow } from "./provisional.js";

class RoundChoices extends Component {
  draw(data) {
    return h(
      "ul",
      { class: "round-choices seg-group" },
      data.choices.map((choice) =>
        h(
          "li",
          { "data-round-choice": choice.number },
          h(
            "button",
            {
              type: "button",
              class: `seg${choice.number === data.selected ? " on" : ""}`,
              "data-action": "choose-round",
              "aria-pressed": String(choice.number === data.selected),
              onclick: () => this.emit({ type: "choose-round", round: choice.number }),
            },
            h("span", { class: "round-number" }, `Round ${choice.number}`),
            h("span", { class: "round-subject" }, choice.subject),
          ),
        ),
      ),
    );
  }
}

export class PastRounds {
  constructor(emit) {
    this.empty = h("p", { class: "empty" }, "No round has been sent yet.");
    this.choices = new RoundChoices(emit);
    this.title = h("h2", { class: "round-title" });
    this.fixes = new Fixes(emit);
    this.cardBox = h("div", { class: "cards" });
    this.rowBox = h("div", { class: "provisional-rows" });
    this.list = h(
      "section",
      { class: "provisional-list", "data-provisional-list": true },
      h("header", { class: "list-head" }, h("h3", {}, "Provisional answers")),
      this.rowBox,
    );
    this.cards = new KeyedList(this.cardBox, () => new Card(emit, { past: true }));
    this.rows = new KeyedList(this.rowBox, () => new ProvisionalRow(emit, { past: true }));
    this.section = h("section", { class: "past-round" }, this.title, this.fixes.el, this.cardBox, this.list);
    this.el = h("div", { class: "past-rounds" }, this.empty, this.choices.el, this.section);
  }

  update(data) {
    const round = data.round;
    this.empty.hidden = data.choices.length > 0;
    this.choices.update({ choices: data.choices, selected: data.selected });
    this.section.hidden = round === null;
    if (round) {
      this.section.dataset.pastRound = String(round.number);
      this.title.replaceChildren(h("span", { class: "round-number" }, `Round ${round.number}`), h("span", {}, round.subject));
    } else {
      delete this.section.dataset.pastRound;
    }
    this.list.hidden = round === null || round.provisional.length === 0;
    this.fixes.update(round?.fixes ?? []);
    this.cards.update((round?.human ?? []).map((q) => ({ key: q.id, data: q })));
    this.rows.update((round?.provisional ?? []).map((q) => ({ key: q.id, data: q })));
  }
}
