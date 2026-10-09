import { h } from "../dom.js";
import { svgElement, textMetrics, drawing } from "../explanation-svg.js";
import { sequenceDrawing } from "../explanation-sequence.js";

export function explanationDiagram(part) {
  const host = h("div", { class: "explanation-diagram-region" });
  const svg = svgElement("svg", { class: "explanation-diagram", role: "img", "aria-label": part.title });
  async function render() {
    await document.fonts.ready;
    if (!host.isConnected) return;
    host.replaceChildren(svg);
    svg.replaceChildren();
    try {
      sequenceDrawing(part, drawing(svg, textMetrics(svg)));
    } catch (error) {
      host.replaceChildren(h("p", { role: "status" }, error.message), h("pre", { class: "explanation-failure" }, JSON.stringify(part, null, 2)));
    }
  }
  requestAnimationFrame(render);
  return host;
}
