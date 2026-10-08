// A human question's card: folded at first, opened to the prerequisites, background and
// option details (docs/spec/screen.md, "たたんだカード"). The question, its options with the
// consequence, the note sent with them and the stamp are the body; the defer switch sits by
// the question's mark, and asks to the LLM below.
import { Component, h } from "../dom.js";
import { translator } from "../strings.js";
import {
  chainLine,
  consequence,
  deferSwitch,
  details,
  noteField,
  optionList,
  pastAnswer,
  pastMarks,
  stampButton,
} from "./question-parts.js";
import { thread } from "./thread.js";

export class Card extends Component {
  /// `past` draws a question of a past round: read only.
  constructor(emit, { past = false } = {}) {
    super(emit);
    this.past = past;
    this.open = false;
    this.thread = { open: null, expanded: false, follows: null };
    this.timeShown = false;
  }

  /// Whether a stamp that can no longer be pressed shows its time, and the tap that toggles it.
  stampTime() {
    return {
      timeShown: this.timeShown,
      toggleTime: () => {
        this.timeShown = !this.timeShown;
        this.redraw();
      },
    };
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

  revealReply() {
    this.open = true;
    this.thread = { ...this.thread, open: true, expanded: true };
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
          deferSwitch(question, this.emit),
          pastMarks(question),
        ),
      ),
      h("p", { class: "why-now" }, h("span", { class: "why-now-label" }, t("card.why-now")), question.why_now),
      chainLine(question, this.emit),
      h(
        "button",
        { type: "button", class: "link details-toggle", "data-action": "open", "data-focus": `details-${question.id}`, "aria-expanded": String(this.open), onclick: () => this.toggle() },
        this.open ? t("card.hide-details") : t("card.show-details"),
      ),
      this.open ? details(question) : null,
      h(
        "div",
        { class: "card-body" },
        h(
          "div",
          { class: "card-answer" },
          optionList(question, this.emit, this.open),
          consequence(question),
          noteField(question, this.emit),
        ),
        h("div", { class: "card-stamp" }, stampButton(question, this.emit, this.stampTime())),
      ),
      pastAnswer(question, this.emit),
      h(
        "footer",
        { class: "card-foot" },
        this.threadFor(question),
        h(
          "div",
          { class: "card-links" },
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
