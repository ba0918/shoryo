// The provisional list: one row per provisional question, changed in place with the same
// option choice the cards use, opened into a card (docs/spec/screen.md, "仮決めの一覧").
import { Component, h } from "../dom.js";
import { translator } from "../strings.js";
import {
  answerExtras,
  chainLine,
  consequence,
  details,
  optionList,
  optionSegments,
  pastAnswer,
  pastMarks,
  stampButton,
} from "./question-parts.js";
import { thread } from "./thread.js";

export class ProvisionalRow extends Component {
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

  draw(question) {
    const t = translator(question.lang);
    return h(
      "div",
      {
        class: `provisional-row${this.open ? " open" : ""}${question.landed ? " landed" : ""}${this.past ? " past" : ""}`,
        "data-provisional-row": question.id,
        "data-past-question": this.past ? question.id : undefined,
        "data-landed": question.landed,
      },
      h(
        "div",
        { class: "row-main" },
        h(
          "div",
          { class: "row-question" },
          h("span", { class: "question-text" }, question.text),
          this.past ? h("span", { class: "badges", "data-question-marks": true }, pastMarks(question)) : null,
        ),
        this.open ? null : optionSegments(question, this.emit),
        this.open ? null : consequence(question),
        this.past ? null : h("div", { class: "row-stamp" }, stampButton(question, this.emit)),
      ),
      pastAnswer(question, this.emit),
      this.open
        ? h(
            "div",
            { class: "row-card" },
            h("p", { class: "why-now" }, h("span", { class: "why-now-label" }, t("card.why-now")), question.why_now),
            chainLine(question, this.emit),
            details(question),
            optionList(question, this.emit, true),
            consequence(question),
            answerExtras(question, this.emit),
            this.threadFor(question),
          )
        : null,
      h(
        "div",
        { class: "row-links" },
        h(
          "button",
          {
            type: "button",
            class: "link",
            "data-action": "open",
            "aria-expanded": String(this.open),
            onclick: () => {
              this.open = !this.open;
              this.redraw();
            },
          },
          this.open ? t("provisional.fold") : t("provisional.open"),
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
              t("provisional.decide-myself"),
            ),
      ),
    );
  }
}
