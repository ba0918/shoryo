// The one dialog layer: at most one dialog is open, named by the top-level component.
import { Component, h } from "../dom.js";
import { translator } from "../strings.js";
import { decisionItem } from "./decision-item.js";
import { ExplanationViewer } from './explanation-viewer.js';
import { operationButton } from './diagram-controls.js';

const bodies = {
  /// The full text of a decision and where it was decided.
  decision: (detail, emit) => ({
    label: translator(detail.lang)("dialog.decision"),
    attrs: { "data-decision-detail": detail.id, "data-focus-scope": `decision-dialog-${detail.id}` },
    body: decisionItem(detail, emit),
  }),
  /// 見直す asks first: a mistaken press must not reach the LLM.
  "confirm-review": (data, emit) => {
    const t = translator(data.lang);
    return {
      label: t("review.title"),
      attrs: { "data-review-confirmation": data.decision, "data-focus-scope": `review-dialog-${data.decision}` },
      body: [
        h("h2", { class: "dialog-title" }, t("review.title")),
        h("p", { class: "review-target" }, data.name),
        h("p", {}, t(data.resultWording ? "review.confirm-result" : "review.confirm")),
        h(
          "div",
          { class: "dialog-actions" },
          h(
            "button",
            { type: "button", class: "btn", "data-action": "cancel-review", onclick: () => emit({ type: "close-dialog" }) },
            t("review.no"),
          ),
          h(
            "button",
            {
              type: "button",
              class: "btn primary",
              "data-action": "confirm-review",
              onclick: () => emit({ type: "review", decision: data.decision }),
            },
            t("review.yes"),
          ),
        ),
      ],
    };
  },
};

export class DialogLayer extends Component {
  /// `extra` adds dialog kinds: { kind: (data, emit) => { label, attrs, body } }.
  constructor(emit, extra = {}) {
    super(emit);
    this.bodies = { ...bodies, ...extra };
  }

  draw(dialog) {
    if (dialog?.kind !== 'explanation') { this.viewer?.dispose(); this.viewer = null; this.viewerIdentity = null; }
    if (!dialog) return h("div", { class: "dialog-layer", hidden: true });
    const { label, attrs, body } = this.bodies[dialog.kind](dialog.data, this.emit);
    let content = body;
    if (dialog.kind === 'explanation') {
      const identity = JSON.stringify([dialog.data.identity, dialog.data.part]);
      if (identity !== this.viewerIdentity) { this.viewer?.dispose(); this.viewer = new ExplanationViewer(); this.viewerIdentity = identity; }
      content = this.viewer.update(dialog.data).el;
    }
    const t = translator(dialog.lang);
    const close = () => this.emit({ type: "close-dialog" });
    return h(
      "div",
      { class: "dialog-layer", onclick: (event) => event.target === event.currentTarget && close() },
      h(
        "div",
        { class: "dialog", role: "dialog", "aria-modal": "true", "aria-label": label, "data-focus-scope": `dialog-${dialog.kind}`, ...attrs,
          onkeydown: event => {
            if (dialog.kind !== "explanation" || event.key !== "Tab") return;
            const controls = [...event.currentTarget.querySelectorAll('button:not(:disabled), [href], input:not(:disabled), select:not(:disabled), textarea:not(:disabled), [tabindex="0"]')].filter(element => !element.hidden);
            const first = controls[0], last = controls.at(-1);
            if (event.shiftKey && document.activeElement === first) { event.preventDefault(); last?.focus(); }
            else if (!event.shiftKey && document.activeElement === last) { event.preventDefault(); first?.focus(); }
          },
        },
        dialog.kind === 'explanation' ? operationButton('close', t('dialog.close'), { class: 'operation-button close', 'data-action': 'close', onclick: close }) : h("button", { type: "button", class: "btn quiet small close", "data-action": "close", onclick: close }, t("dialog.close")),
        h("div", { class: "dialog-body" }, content),
      ),
    );
  }
}
