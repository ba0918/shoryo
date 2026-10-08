// 結果: what the current round shows when it has no questions — the topic has converged
// (docs/spec/screen.md, "結果"). The finished picture and the records, with 見直したい beside
// each decision; sending happens from the send bar.
import { Component, h } from "../dom.js";
import { diagramView } from "./diagram-view.js";
import { translator } from "../strings.js";
import { recordSections } from "./decisions.js";

const RESULT_RECORDS = ["decisions", "not-building", "rejected", "undecided", "delegated"];

export class ResultView extends Component {
  draw(result) {
    if (!result) return h("div", { hidden: true });
    const t = translator(result.lang);
    return h(
      "section",
      { class: "result", "data-result": true },
      h("header", { class: "result-head" }, h("h2", {}, t("result.title")), h("p", { class: "list-hint" }, t("result.hint"))),
      h(
        "div",
        { class: "result-picture" },
        h("h3", {}, t("header.finished-picture")),
        result.picture
          ? diagramView(result.picture, result.lang)
          : h("p", { class: "empty" }, t("result.no-picture")),
      ),
      h("div", { class: "result-records" }, recordSections(result, RESULT_RECORDS, this.emit)),
    );
  }
}
