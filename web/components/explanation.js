import { h } from "../dom.js";
import { renderDiagram } from "../diagram.js";
import { translator } from "../strings.js";
import { explanationDiagram } from "./explanation-diagram.js";

export function explanation(parts, lang, key, emit) {
  const t = translator(lang);
  return h("div", { class: "explanation", "data-explanation": key }, parts.map((part, index) => {
    const identity = `${key}-${index}`;
    switch (part.type) {
      case "text":
        return h("p", { "data-part": "text", class: "explanation-text" }, part.body);
      case "code":
        return codePart(part, t, identity);
      case "diagram":
        return h("section", { "data-part": "diagram" }, metadata(part, t), h("div", { class: "diagram-box" }, renderDiagram(part.source)));
      case "sequence":
      case "flow":
        return h("section", { "data-part": part.type }, metadata(part, t), explanationDiagram(part, lang, { identity, emit }));
      default:
        return null;
    }
  }));
}

function metadata(part, t) {
  return h("div", { class: "part-metadata" },
    part.title ? h("strong", {}, part.title) : null,
    part.language ? h("span", {}, part.language === "pseudocode" ? t("part.pseudocode") : part.language) : null,
    h("span", { class: "badge" }, t(`part.role.${part.role}`)),
  );
}

function codePart(part, t, identity) {
  const content = h("code", {}, part.body);
  const pre = h("pre", { class: "explanation-code", tabindex: "0", "data-focus": `code-${identity}` }, content);
  const status = h("span", { role: "status", "aria-live": "polite" });
  const select = h("button", {
    type: "button", hidden: true, "data-focus": `select-code-${identity}`,
    onclick: () => {
      const range = document.createRange();
      range.selectNodeContents(content);
      const selection = window.getSelection();
      selection.removeAllRanges();
      selection.addRange(range);
    },
  }, t("part.select-code"));
  const copy = h("button", {
    type: "button", "data-focus": `copy-code-${identity}`,
    onclick: async () => {
      status.textContent = "";
      try {
        if (!navigator.clipboard?.writeText) throw new Error("Clipboard unavailable");
        await navigator.clipboard.writeText(part.body);
        status.textContent = t("part.copied");
      } catch {
        status.textContent = t("part.copy-failed");
        select.hidden = false;
      }
    },
  }, t("part.copy-code"));
  return h("section", { "data-part": "code" }, metadata(part, t), pre,
    h("div", { class: "code-controls" }, copy, select, status));
}
