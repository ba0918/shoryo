// 決まったこと tab: the six records and the decisions in review
// (docs/spec/screen.md, "決まったこと").
import { Component, h } from "../dom.js";
import { decisionItem, reviewControls } from "./decision-item.js";

function section(kind, title, items, draw) {
  return h(
    "section",
    { class: "record", "data-record": kind },
    h("h2", {}, title),
    items.length > 0 ? items.map(draw) : h("p", { class: "empty" }, "None"),
  );
}

export class DecisionsTab extends Component {
  draw(data) {
    const plain = (text) => h("p", {}, text);
    return h(
      "div",
      { class: "decisions-tab" },
      section("decisions", "Decisions", data.decisions, (item) => decisionItem(item, this.emit)),
      section("not-building", "Not building", data.not_building, plain),
      section("rejected", "Rejected options", data.rejected, (item) =>
        h("p", {}, h("strong", {}, item.text), " — ", item.reason, " ", h("span", { class: "label" }, `(in: ${item.question})`)),
      ),
      section("undecided", "Undecided", data.undecided, (item) =>
        h("p", {}, item.text, " ", h("span", { class: "label" }, "Decided by: "), item.decider),
      ),
      section("delegated", "Delegated", data.delegated, (item) =>
        h("p", {}, h("strong", {}, item.text), " — ", item.reason),
      ),
      section("revisions", "Revised", data.revisions, (item) =>
        h(
          "p",
          {},
          h("strong", {}, item.name),
          ` (round ${item.round}): `,
          `${item.before.name}: ${item.before.text}`,
          " → ",
          `${item.after.name}: ${item.after.text}`,
        ),
      ),
      section("in-review", "In review", data.in_review, (item) =>
        h("p", { "data-in-review-item": item.id }, h("strong", {}, item.name), ": ", item.text, " ", reviewControls(item.review, this.emit)),
      ),
    );
  }
}
