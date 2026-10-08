// The provisional list: one row per provisional question, changed in place with the same
// option choice the cards use, opened into a card (docs/spec/screen.md, "仮決めの一覧").
import { Component, h } from "../dom.js";
import { translator } from "../strings.js";
import {
  chainLine,
  consequence,
  deferSwitch,
  details,
  noteField,
  optionList,
  optionSegments,
  pastAnswer,
  pastMarks,
  stampButton,
} from "./question-parts.js";
import { thread } from "./thread.js";

export class ProvisionalRow extends Component {
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

  revealReply() {
    this.open = true;
    this.thread = { ...this.thread, open: true, expanded: true };
    this.redraw();
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
        h("div", { class: "row-stamp" }, stampButton(question, this.emit, this.stampTime())),
      ),
      pastAnswer(question, this.emit),
      this.open
        ? h(
            "div",
            { class: "row-card" },
            h(
              "div",
              { class: "row-card-head" },
              h("p", { class: "why-now" }, h("span", { class: "why-now-label" }, t("card.why-now")), question.why_now),
              deferSwitch(question, this.emit),
            ),
            chainLine(question, this.emit),
            details(question),
            optionList(question, this.emit, true),
            consequence(question),
            noteField(question, this.emit),
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
            "data-focus": `provisional-toggle-${question.id}`,
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
