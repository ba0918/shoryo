// A human question's card: folded at first, opened to the prerequisites, background and
// option details (docs/spec/screen.md, "たたんだカード"). The question, its options with the
// consequence and the stamp are the body; the note, the defer switch and asking back sit
// below as secondary parts.
import { Component, h } from "../dom.js";
import { translator } from "../strings.js";
import {
  answerExtras,
  chainLine,
  consequence,
  details,
  optionList,
  pastAnswer,
  pastMarks,
  stampButton,
} from "./question-parts.js";
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
    this.redraw();
  }

  draw(question) {
    const t = translator(question.lang);
    return h(
      "article",
      {
        class: `card${this.open ? " open" : ""}${question.landed ? " landed" : ""}${this.past ? " past" : ""}`,
        "data-card": question.id,
        "data-past-question": this.past ? question.id : undefined,
        "data-landed": question.landed,
      },
      h(
        "header",
        { class: "card-head" },
        h("h3", { class: "question-text" }, question.text),
        h(
          "div",
          { class: "badges", "data-question-marks": true },
          h("span", { class: "badge human", "data-mark": "human" }, t("card.human")),
          pastMarks(question),
        ),
      ),
      h("p", { class: "why-now" }, h("span", { class: "why-now-label" }, t("card.why-now")), question.why_now),
      chainLine(question, this.emit),
      this.open ? details(question) : null,
      h(
        "div",
        { class: "card-body" },
        h("div", { class: "card-answer" }, optionList(question, this.emit, this.open), consequence(question)),
        this.past ? null : h("div", { class: "card-stamp" }, stampButton(question, this.emit)),
      ),
      pastAnswer(question, this.emit),
      h(
        "footer",
        { class: "card-foot" },
        answerExtras(question, this.emit),
        this.threadFor(question),
        h(
          "div",
          { class: "card-links" },
          h(
            "button",
            { type: "button", class: "link", "data-action": "open", "aria-expanded": String(this.open), onclick: () => this.toggle() },
            this.open ? t("card.hide-details") : t("card.show-details"),
          ),
          this.past
            ? null
            : h(
                "button",
                {
                  type: "button",
                  class: "btn quiet small",
                  "data-action": "swap",
                  disabled: question.locked,
                  onclick: () => this.emit({ type: "op", op: { op: "swap_class", question: question.id } }),
                },
                t("card.make-provisional"),
              ),
        ),
      ),
    );
  }
}
