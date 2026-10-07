// 直したこと: what the LLM fixed from the records in a review round, for reading only
// (docs/spec/screen.md, "直したこと").
import { Component, h } from "../dom.js";
import { reviewControls } from "./decision-item.js";

export function fixList(fixes, emit) {
  if (fixes.length === 0) return null;
  return h(
    "section",
    { class: "fixes", "data-fixes": true },
    h("h2", {}, "Fixed by the LLM"),
    fixes.map((fix) =>
      h(
        "div",
        { class: "fix", "data-fix": fix.index },
        h("p", {}, fix.text),
        fix.change
          ? h(
              "div",
              { class: "fix-change" },
              h("p", {}, h("span", { class: "label" }, "Before: "), fix.change.before ? `${fix.change.before.name}: ${fix.change.before.text}` : "(new)"),
              h("p", {}, h("span", { class: "label" }, "After: "), `${fix.change.after.name}: ${fix.change.after.text}`),
              h("p", {}, reviewControls(fix.change.review, emit)),
            )
          : null,
      ),
    ),
  );
}

/// The fix list as a region of its own, hidden when there is nothing to list.
export class Fixes extends Component {
  draw(fixes) {
    return fixList(fixes, this.emit) ?? h("div", { hidden: true });
  }
}
