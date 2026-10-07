// A human question's card: folded at first, opened to the prerequisites, background and
// option details (docs/spec/screen.md, "たたんだカード").
import { Component, h } from "../dom.js";
import { answerExtras, chainLine, consequence, details, optionList, pastAnswer, pastMarks } from "./question-parts.js";
import { thread } from "./thread.js";

export class Card extends Component {
  /// `past` draws a question of a past round: read only, with every exchange shown.
  constructor(emit, { past = false } = {}) {
    super(emit);
    this.past = past;
    this.open = false;
    this.thread = { mode: past ? "all" : "latest", follows: null };
  }

  threadFor(question) {
    const onLocal = (change) => {
      this.thread = { ...this.thread, ...change };
      this.redraw();
    };
    return thread(question, { ...this.thread, canAsk: !question.locked }, onLocal, this.emit);
  }

  toggle() {
    this.open = !this.open;
    // Opening a human card is what clears the unopened mark; folding it again is not news.
    if (this.open && !this.past) this.emit({ type: "op", op: { op: "open", question: this.data.id } });
    this.redraw();
  }

  draw(question) {
    return h(
      "section",
      {
        class: `card${this.open ? " open" : ""}${question.landed ? " landed" : ""}`,
        "data-card": question.id,
        "data-past-question": this.past ? question.id : undefined,
        "data-landed": question.landed,
      },
      h(
        "div",
        { class: "card-marks", "data-question-marks": true },
        h("span", { class: "mark human", "data-mark": "human" }, "Human decides"),
        question.unopened ? h("span", { class: "mark unopened", "data-mark": "unopened" }, "Not opened yet") : null,
        pastMarks(question),
      ),
      h("h3", { class: "question-text" }, question.text),
      h("p", { class: "why-now" }, h("span", { class: "label" }, "Why now: "), question.why_now),
      chainLine(question.chains, this.emit),
      this.open ? details(question) : null,
      optionList(question, this.emit, this.open),
      consequence(question),
      answerExtras(question, this.emit),
      pastAnswer(question, this.emit),
      this.threadFor(question),
      h(
        "div",
        { class: "card-actions" },
        h(
          "button",
          { type: "button", "data-action": "open", onclick: () => this.toggle() },
          this.open ? "Fold" : "Open",
        ),
        this.past
          ? null
          : h(
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
