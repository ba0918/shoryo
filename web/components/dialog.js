// The one dialog layer: at most one dialog is open, named by the top-level component.
import { Component, h } from "../dom.js";
import { translator } from "../strings.js";
import { decisionItem } from "./decision-item.js";

const bodies = {
  /// The full text of a decision and where it was decided.
  decision: (detail, emit) => ({
    label: translator(detail.lang)("dialog.decision"),
    attrs: { "data-decision-detail": detail.id },
    body: decisionItem(detail, emit),
  }),
};

export class DialogLayer extends Component {
  /// `extra` adds dialog kinds: { kind: (data, emit) => { label, attrs, body } }.
  constructor(emit, extra = {}) {
    super(emit);
    this.bodies = { ...bodies, ...extra };
  }

  draw(dialog) {
    if (!dialog) return h("div", { class: "dialog-layer", hidden: true });
    const { label, attrs, body } = this.bodies[dialog.kind](dialog.data, this.emit);
    const t = translator(dialog.lang);
    const close = () => this.emit({ type: "close-dialog" });
    return h(
      "div",
      { class: "dialog-layer", onclick: (event) => event.target === event.currentTarget && close() },
      h(
        "div",
        { class: "dialog", role: "dialog", "aria-label": label, ...attrs },
        h("button", { type: "button", class: "btn quiet small close", "data-action": "close", onclick: close }, t("dialog.close")),
        h("div", { class: "dialog-body" }, body),
      ),
    );
  }
}
