import { Component, h } from "../dom.js";
import { translator } from "../strings.js";

export class CodeParts {
  constructor() {
    this.items = new Map();
    this.shown = new Set();
  }

  begin() { this.shown.clear(); }

  render(part, lang, identity) {
    this.shown.add(identity);
    let component = this.items.get(identity);
    if (!component || JSON.stringify(component.data.part) !== JSON.stringify(part)) {
      component = new CodePart();
      this.items.set(identity, component);
    }
    return component.update({ part, lang, identity }).el;
  }

  end() {
    for (const identity of this.items.keys()) if (!this.shown.has(identity)) this.items.delete(identity);
  }
}

class CodePart extends Component {
  constructor() {
    super();
    this.feedback = null;
    this.manual = false;
  }

  async copy() {
    const body = this.data.part.body;
    this.feedback = null;
    this.redraw();
    try {
      if (!navigator.clipboard?.writeText) throw new Error("Clipboard unavailable");
      await navigator.clipboard.writeText(body);
      this.feedback = "part.copied";
    } catch {
      this.feedback = "part.copy-failed";
      this.manual = true;
    }
    this.redraw();
  }

  draw({ part, lang, identity }) {
    const t = translator(lang);
    const content = h("code", {}, part.body);
    const pre = h("pre", { class: "explanation-code", tabindex: "0", "data-focus": `code-${identity}` }, content);
    const select = h("button", {
      type: "button", hidden: !this.manual, "data-focus": `select-code-${identity}`,
      onclick: () => {
        const range = document.createRange();
        range.selectNodeContents(content);
        const selection = window.getSelection();
        selection.removeAllRanges();
        selection.addRange(range);
      },
    }, t("part.select-code"));
    return h("section", { "data-part": "code" },
      h("div", { class: "part-metadata" },
        part.title ? h("strong", {}, part.title) : null,
        h("span", {}, part.language === "pseudocode" ? t("part.pseudocode") : part.language),
        h("span", { class: "badge" }, t(`part.role.${part.role}`))),
      pre, h("div", { class: "code-controls" },
        h("button", { type: "button", "data-focus": `copy-code-${identity}`, onclick: () => this.copy() }, t("part.copy-code")),
        select, h("span", { role: "status", "aria-live": "polite" }, this.feedback ? t(this.feedback) : "")));
  }
}
