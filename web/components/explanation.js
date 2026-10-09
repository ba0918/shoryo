import { h } from "../dom.js";
import { renderDiagram } from "../diagram.js";
import { translator } from "../strings.js";

export function explanation(parts, lang, key, emit, codeParts, diagramParts) {
  const t = translator(lang);
  return h("div", { class: "explanation", "data-explanation": key }, parts.map((part, index) => {
    const identity = `${key}-${index}`;
    switch (part.type) {
      case "text":
        return h("p", { "data-part": "text", class: "explanation-text" }, part.body);
      case "code":
        return codeParts.render(part, lang, identity);
      case "diagram":
        return h("section", { "data-part": "diagram" }, metadata(part, t), h("div", { class: "diagram-box" }, renderDiagram(part.source)));
      case "sequence":
      case "flow":
        return diagramParts.render(part, lang, identity);
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
