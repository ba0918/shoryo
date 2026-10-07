// 過去のラウンド: a sent round as it was answered, read only
// (docs/spec/screen.md, "過去のラウンド").
import { Component, h } from "../dom.js";
import { decisionItem } from "./decision-item.js";
import { fixList } from "./fixes.js";
import { allExchanges } from "./thread.js";

export class PastRounds extends Component {
  draw(data) {
    if (data.choices.length === 0) {
      return h("div", { class: "past-rounds" }, h("p", { class: "empty" }, "No round has been sent yet."));
    }
    return h(
      "div",
      { class: "past-rounds" },
      h(
        "ul",
        { class: "round-choices" },
        data.choices.map((choice) =>
          h(
            "li",
            { "data-round-choice": choice.number },
            h(
              "button",
              {
                type: "button",
                "data-action": "choose-round",
                "aria-pressed": String(choice.number === data.selected),
                onclick: () => this.emit({ type: "choose-round", round: choice.number }),
              },
              `Round ${choice.number}: `,
              h("span", {}, choice.subject),
            ),
          ),
        ),
      ),
      data.round ? this.round(data.round, data.highlight) : null,
    );
  }

  round(round, highlight) {
    return h(
      "section",
      { class: "past-round", "data-past-round": round.number },
      h("h2", {}, `Round ${round.number}: `, h("span", {}, round.subject)),
      fixList(round.fixes, this.emit),
      round.questions.map((question) => this.question(question, highlight)),
    );
  }

  question(question, highlight) {
    return h(
      "article",
      {
        class: `card past${question.id === highlight ? " landed" : ""}`,
        "data-past-question": question.id,
      },
      h(
        "div",
        { class: "card-marks", "data-question-marks": true },
        h("span", { class: "mark human" }, question.human ? "Human decides" : "Provisional"),
        question.sentUnseen ? h("span", { class: "mark unopened", "data-mark": "sent-unseen" }, "Sent without opening") : null,
        question.deferred ? h("span", { class: "mark deferred", "data-mark": "deferred" }, "Ask me again next round") : null,
      ),
      h("h3", { class: "question-text" }, question.text),
      h("p", { "data-chosen": true }, h("span", { class: "label" }, "Answer: "), question.chosen ?? "(deferred)"),
      h("p", { "data-recommended": true }, h("span", { class: "label" }, "Recommended: "), question.recommended),
      question.note ? h("p", {}, h("span", { class: "label" }, "Note: "), question.note) : null,
      question.asks.length > 0 ? h("div", { class: "thread" }, allExchanges(question.asks)) : null,
      question.decisions.length > 0
        ? h(
            "div",
            { class: "decided-here" },
            h("h4", {}, "Decided here"),
            question.decisions.map((item) => decisionItem(item, this.emit)),
          )
        : null,
    );
  }
}
