// 決まったこと tab: the six records and the decisions in review
// (docs/spec/screen.md, "決まったこと"). The record lists are shared with the result.
import { Component, h } from "../dom.js";
import { translator } from "../strings.js";
import { decisionItem, reviewControls } from "./decision-item.js";

function recordSection(kind, title, items, draw, t) {
  return h(
    "section",
    { class: "record", "data-record": kind },
    h("h2", {}, title, h("span", { class: "count" }, String(items.length))),
    items.length > 0 ? h("div", { class: "record-items" }, items.map(draw)) : h("p", { class: "empty" }, t("records.none")),
  );
}

/// The record lists `kinds` names, in that order, from data shaped like the decisions tab's.
export function recordSections(data, kinds, emit) {
  const t = translator(data.lang);
  const section = (kind, title, items, draw) => recordSection(kind, title, items, draw, t);
  const plain = (text) => h("p", { class: "record-item" }, text);
  const all = {
    decisions: () =>
      section("decisions", t("records.decisions"), data.decisions, (item) => decisionItem(item, emit)),
    "not-building": () =>
      section("not-building", t("records.not-building"), data.not_building, plain),
    rejected: () =>
      section("rejected", t("records.rejected"), data.rejected, (item) =>
        h(
          "div",
          { class: "record-item" },
          h("p", { class: "record-title" }, item.text),
          h("p", {}, item.reason),
          h("p", { class: "record-meta" }, h("span", { class: "meta-label" }, t("records.rejected-in")), item.question),
        ),
      ),
    undecided: () =>
      section("undecided", t("records.undecided"), data.undecided, (item) =>
        h(
          "div",
          { class: "record-item" },
          h("p", { class: "record-title" }, item.text),
          h("p", { class: "record-meta" }, h("span", { class: "meta-label" }, t("records.decider")), item.decider),
        ),
      ),
    delegated: () =>
      section("delegated", t("records.delegated"), data.delegated, (item) =>
        h("div", { class: "record-item" }, h("p", { class: "record-title" }, item.text), h("p", {}, item.reason)),
      ),
    revisions: () =>
      section("revisions", t("records.revisions"), data.revisions, (item) =>
        h(
          "div",
          { class: "record-item" },
          h("p", { class: "record-title" }, item.name, h("span", { class: "record-meta" }, ` ${t("round.number", { number: item.round })}`)),
          h(
            "div",
            { class: "before-after" },
            h("div", { class: "before" }, h("span", { class: "meta-label" }, t("records.before")), h("p", {}, `${item.before.name}: ${item.before.text}`)),
            h("div", { class: "after" }, h("span", { class: "meta-label" }, t("records.after")), h("p", {}, `${item.after.name}: ${item.after.text}`)),
          ),
        ),
      ),
    "in-review": () =>
      section("in-review", t("records.in-review"), data.in_review, (item) =>
        h(
          "div",
          { class: "record-item", "data-in-review-item": item.id },
          h("p", { class: "record-title" }, item.name),
          h("p", {}, item.text),
          h("div", { class: "decision-meta" }, reviewControls(item.review, emit)),
        ),
      ),
  };
  return kinds.map((kind) => all[kind]());
}

export const ALL_RECORDS = ["decisions", "not-building", "rejected", "undecided", "delegated", "revisions", "in-review"];

export class DecisionsTab extends Component {
  draw(data) {
    return h("div", { class: "decisions-tab" }, recordSections(data, ALL_RECORDS, this.emit));
  }
}
