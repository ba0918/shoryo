// The one dialog layer: at most one dialog is open, named by the top-level component.
import { Component, h } from "../dom.js";

const bodies = {
  /// The full text of a decision and where it was decided.
  decision: (detail) => ({
    label: "Decision",
    attrs: { "data-decision-detail": detail.id },
    body: [
      h("h3", {}, detail.name),
      h("p", {}, detail.text),
      h("p", { class: "source" }, h("span", { class: "label" }, "Decided in: "), detail.source),
    ],
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
    const close = () => this.emit({ type: "close-dialog" });
    return h(
      "div",
      { class: "dialog-layer", onclick: (event) => event.target === event.currentTarget && close() },
      h(
        "div",
        { class: "dialog", role: "dialog", "aria-label": label, ...attrs },
        h("button", { type: "button", class: "close", "data-action": "close", onclick: close }, "Close"),
        body,
      ),
    );
  }
}
