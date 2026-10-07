// The provisional list: one row per provisional question, changed in place, opened into a
// card (docs/spec/screen.md, "仮決めの一覧").
import { Component, h } from "../dom.js";
import { answerExtras, chainLine, consequence, details, optionList, pastAnswer, pastMarks, stampButton } from "./question-parts.js";
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
    const choose = (event) =>
      this.emit({
        type: "op",
        op: { op: "choose", question: question.id, option: Number(event.target.value) },
      });
    return h(
      "div",
      {
        class: `provisional-row${this.open ? " open" : ""}${question.landed ? " landed" : ""}`,
        "data-provisional-row": question.id,
        "data-past-question": this.past ? question.id : undefined,
        "data-landed": question.landed,
      },
      h(
        "div",
        { class: "row-line" },
        h("span", { class: "question-text" }, question.text),
        this.past ? h("span", { class: "card-marks", "data-question-marks": true }, pastMarks(question)) : null,
        this.open
          ? null
          : h(
              "select",
              { disabled: question.locked, "data-focus": `select-${question.id}`, onchange: choose },
              question.options.map((option) =>
                h(
                  "option",
                  { value: String(option.index), selected: option.index === question.selected },
                  option.recommended ? `${option.text} (recommended)` : option.text,
                ),
              ),
            ),
        this.open ? null : consequence(question),
        this.past ? null : stampButton(question, this.emit),
        h(
          "button",
          {
            type: "button",
            "data-action": "open",
            onclick: () => {
              this.open = !this.open;
              this.redraw();
            },
          },
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
              "Decide myself",
            ),
      ),
      pastAnswer(question, this.emit),
      this.open
        ? h(
            "div",
            { class: "row-card" },
            h("p", { class: "why-now" }, h("span", { class: "label" }, "Why now: "), question.why_now),
            chainLine(question.chains, this.emit),
            details(question),
            optionList(question, this.emit, true),
            consequence(question),
            answerExtras(question, this.emit),
            this.threadFor(question),
          )
        : null,
    );
  }
}
