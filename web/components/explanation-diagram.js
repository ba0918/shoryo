import { h } from "../dom.js";
import { svgElement, textMetrics, drawing } from "../explanation-svg.js";
import { sequenceDrawing } from "../explanation-sequence.js";
import { flowDrawing } from "../explanation-flow.js";
import { translator } from "../strings.js";
import { validateDrawing } from "../explanation-layout-validation.js";
import { guardDiagram } from "../explanation-bounds.js";

export function explanationDiagram(part, lang) {
  const host = h("div", { class: "explanation-diagram-region" });
  const svg = svgElement("svg", { class: "explanation-diagram", role: "img", "aria-label": part.title });
  let queued = false;
  let signature = "";
  function conditions() {
    const style = getComputedStyle(host);
    const root = getComputedStyle(document.documentElement);
    return [root.fontSize, style.fontFamily, style.fontSize, style.fontWeight, style.fontStyle, style.letterSpacing, style.lineHeight, window.devicePixelRatio, document.documentElement.dataset.theme].join("|");
  }
  async function render() {
    await document.fonts.ready;
    if (!host.isConnected) return;
    signature = conditions();
    host.replaceChildren(svg);
    svg.replaceChildren();
    try {
      guardDiagram(part);
      const draw = drawing(svg, textMetrics(svg));
      const result = part.type === "flow" ? flowDrawing(part, draw, translator(lang)) : sequenceDrawing(part, draw);
      validateDrawing(result);
    } catch (error) {
      host.replaceChildren(h("p", { role: "status", "data-layout-failure": true }, error.message), h("pre", { class: "explanation-failure" }, JSON.stringify(part, null, 2)));
    }
  }
  function check() {
    if (!host.isConnected) { observer.disconnect(); resize.disconnect(); document.fonts.removeEventListener("loadingdone", schedule); return; }
    if (signature !== conditions()) schedule();
  }
  function schedule() {
    if (queued) return;
    queued = true;
    requestAnimationFrame(() => { queued = false; render(); });
  }
  const observer = new MutationObserver(check);
  const resize = new ResizeObserver(check);
  requestAnimationFrame(() => {
    if (!host.isConnected) return;
    observer.observe(document.documentElement, { attributes: true });
    observer.observe(document.head, { childList: true, subtree: true, characterData: true, attributes: true });
    resize.observe(host);
    document.fonts.addEventListener("loadingdone", schedule);
    render();
  });
  return host;
}
