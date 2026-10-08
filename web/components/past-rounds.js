// 過去のラウンド: a sent round as it was answered, drawn with the same cards and provisional
// list as the current round, read only (docs/spec/screen.md, "過去のラウンド").
import { Component, KeyedList, h } from "../dom.js";
import { translator } from "../strings.js";
import { Card } from "./card.js";
import { Fixes } from "./fixes.js";
import { ProvisionalRow } from "./provisional.js";
import { ResultView } from "./result.js";

class RoundChoices extends Component {
  draw(data) {
    const t = translator(data.lang);
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
            h("span", { class: "round-number" }, t("round.number", { number: choice.number })),
            h("span", { class: "round-subject" }, choice.subject),
          ),
        ),
      ),
    );
  }
}

export class PastRounds {
  revealReply(question) {
    (this.cards.items.get(question) ?? this.rows.items.get(question))?.revealReply();
  }

  constructor(emit) {
    this.empty = h("p", { class: "empty" });
    this.choices = new RoundChoices(emit);
    this.title = h("h2", { class: "round-title" });
    this.fixes = new Fixes(emit);
    this.result = new ResultView(emit);
    this.cardBox = h("div", { class: "cards" });
    this.rowBox = h("div", { class: "provisional-rows" });
    this.list = h(
      "section",
      { class: "provisional-list", "data-provisional-list": true },
      h("header", { class: "list-head" }, (this.listTitle = h("h3"))),
      this.rowBox,
    );
    this.cards = new KeyedList(this.cardBox, () => new Card(emit, { past: true }));
    this.rows = new KeyedList(this.rowBox, () => new ProvisionalRow(emit, { past: true }));
    this.section = h("section", { class: "past-round" }, this.title, this.fixes.el, this.result.el, this.cardBox, this.list);
    this.el = h("div", { class: "past-rounds" }, this.empty, this.choices.el, this.section);
  }

  update(data) {
    const t = translator(data.lang);
    this.empty.textContent = t("past.none");
    this.listTitle.textContent = t("provisional.title");
    const round = data.round;
    this.empty.hidden = data.choices.length > 0;
    this.choices.update({ choices: data.choices, selected: data.selected, lang: data.lang });
    this.section.hidden = round === null;
    if (round) {
      this.section.dataset.pastRound = String(round.number);
      this.title.replaceChildren(
        h("span", { class: "round-number" }, t("round.number", { number: round.number })),
        h("span", {}, round.subject),
      );
    } else {
      delete this.section.dataset.pastRound;
    }
    this.list.hidden = round === null || round.provisional.length === 0;
    this.fixes.update({ fixes: round?.fixes ?? [], lang: data.lang });
    this.result.update(round?.result ?? null);
    this.cards.update((round?.human ?? []).map((q) => ({ key: q.id, data: q })));
    this.rows.update((round?.provisional ?? []).map((q) => ({ key: q.id, data: q })));
  }
}
