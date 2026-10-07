// 決まったこと tab: the six records and the decisions in review
// (docs/spec/screen.md, "決まったこと").
import { Component, h } from "../dom.js";
import { decisionItem, reviewControls } from "./decision-item.js";

function section(kind, title, items, draw) {
  return h(
    "section",
    { class: "record", "data-record": kind },
    h("h2", {}, title, h("span", { class: "count" }, String(items.length))),
    items.length > 0 ? h("div", { class: "record-items" }, items.map(draw)) : h("p", { class: "empty" }, "None yet"),
  );
}

export class DecisionsTab extends Component {
  draw(data) {
    const plain = (text) => h("p", { class: "record-item" }, text);
    return h(
      "div",
      { class: "decisions-tab" },
      section("decisions", "Decisions", data.decisions, (item) => decisionItem(item, this.emit)),
      section("not-building", "Not building", data.not_building, plain),
      section("rejected", "Rejected options", data.rejected, (item) =>
        h(
          "div",
          { class: "record-item" },
          h("p", { class: "record-title" }, item.text),
          h("p", {}, item.reason),
          h("p", { class: "record-meta" }, h("span", { class: "meta-label" }, "Rejected in"), item.question),
        ),
      ),
      section("undecided", "Undecided", data.undecided, (item) =>
        h(
          "div",
          { class: "record-item" },
          h("p", { class: "record-title" }, item.text),
          h("p", { class: "record-meta" }, h("span", { class: "meta-label" }, "Decided by"), item.decider),
        ),
      ),
      section("delegated", "Delegated", data.delegated, (item) =>
        h("div", { class: "record-item" }, h("p", { class: "record-title" }, item.text), h("p", {}, item.reason)),
      ),
      section("revisions", "Revised", data.revisions, (item) =>
        h(
          "div",
          { class: "record-item" },
          h("p", { class: "record-title" }, item.name, h("span", { class: "record-meta" }, ` Round ${item.round}`)),
          h(
            "div",
            { class: "before-after" },
            h("div", { class: "before" }, h("span", { class: "meta-label" }, "Before"), h("p", {}, `${item.before.name}: ${item.before.text}`)),
            h("div", { class: "after" }, h("span", { class: "meta-label" }, "After"), h("p", {}, `${item.after.name}: ${item.after.text}`)),
          ),
        ),
      ),
      section("in-review", "In review", data.in_review, (item) =>
        h(
          "div",
          { class: "record-item", "data-in-review-item": item.id },
          h("p", { class: "record-title" }, item.name),
          h("p", {}, item.text),
          h("div", { class: "decision-meta" }, reviewControls(item.review, this.emit)),
        ),
      ),
    );
  }
}
