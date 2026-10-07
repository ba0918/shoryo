// A human question's card: folded at first, opened to the prerequisites, background and
// option details (docs/spec/screen.md, "たたんだカード").
import { Component, h } from "../dom.js";
import { answerExtras, chainLine, consequence, details, optionList } from "./question-parts.js";

export class Card extends Component {
  constructor(emit) {
    super(emit);
    this.open = false;
  }

  toggle() {
    this.open = !this.open;
    // Opening a human card is what clears the unopened mark; folding it again is not news.
    if (this.open) this.emit({ type: "op", op: { op: "open", question: this.data.id } });
    this.redraw();
  }

  draw(question) {
    return h(
      "section",
      { class: `card${this.open ? " open" : ""}`, "data-card": question.id },
      h(
        "div",
        { class: "card-marks" },
        h("span", { class: "mark human", "data-mark": "human" }, "Human decides"),
        question.unopened ? h("span", { class: "mark unopened", "data-mark": "unopened" }, "Not opened yet") : null,
      ),
      h("h3", { class: "question-text" }, question.text),
      h("p", { class: "why-now" }, h("span", { class: "label" }, "Why now: "), question.why_now),
      chainLine(question.chains, this.emit),
      this.open ? details(question) : null,
      optionList(question, this.emit, this.open),
      consequence(question),
      answerExtras(question, this.emit),
      h(
        "div",
        { class: "card-actions" },
        h(
          "button",
          { type: "button", "data-action": "open", onclick: () => this.toggle() },
          this.open ? "Fold" : "Open",
        ),
        h(
          "button",
          {
            type: "button",
            "data-action": "swap",
            disabled: question.locked,
            onclick: () => this.emit({ type: "op", op: { op: "swap_class", question: question.id } }),
          },
          "Make provisional",
        ),
      ),
    );
  }
}
