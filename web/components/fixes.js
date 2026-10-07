// 直したこと: what the LLM fixed from the records in a review round, for reading only
// (docs/spec/screen.md, "直したこと").
import { Component, h } from "../dom.js";
import { translator } from "../strings.js";
import { reviewControls } from "./decision-item.js";

export function fixList(fixes, lang, emit) {
  if (fixes.length === 0) return null;
  const t = translator(lang);
  return h(
    "section",
    { class: "fixes", "data-fixes": true },
    h("h2", {}, t("fixes.title")),
    fixes.map((fix) =>
      h(
        "div",
        { class: "fix", "data-fix": fix.index },
        h("p", { class: "fix-text" }, fix.text),
        fix.change
          ? h(
              "div",
              { class: "fix-change" },
              h(
                "div",
                { class: "before-after" },
                h("div", { class: "before" }, h("span", { class: "meta-label" }, t("records.before")), h("p", {}, fix.change.before ? `${fix.change.before.name}: ${fix.change.before.text}` : t("fixes.new"))),
                h("div", { class: "after" }, h("span", { class: "meta-label" }, t("records.after")), h("p", {}, `${fix.change.after.name}: ${fix.change.after.text}`)),
              ),
              h("div", { class: "decision-meta" }, reviewControls(fix.change.review, emit)),
            )
          : null,
      ),
    ),
  );
}

/// The fix list as a region of its own, hidden when there is nothing to list.
export class Fixes extends Component {
  draw({ fixes, lang }) {
    return fixList(fixes, lang, this.emit) ?? h("div", { hidden: true });
  }
}
