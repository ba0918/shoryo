// The finished picture in a view of its own: the whole picture fitted in at first, zoomed and
// moved the way the map is (docs/spec/screen.md, "完成図"). The zoom is this view's own state.
import { h } from "../dom.js";
import { drawDiagram } from "../diagram.js";
import { PanZoom } from "../pan-zoom.js";
import { translator } from "../strings.js";

const SVG = "http://www.w3.org/2000/svg";
/// "Fit" never draws the picture larger than its own size.
const FIT_MAX_ZOOM = 1;
/// The view is never taller than this share of the window.
const MAX_HEIGHT = "80vh";

export function diagramView(text, lang) {
  const t = translator(lang);
  const { defs, content, size } = drawDiagram(text);
  const moved = document.createElementNS(SVG, "g");
  moved.append(content);
  const svg = document.createElementNS(SVG, "svg");
  for (const [name, value] of Object.entries({ class: "diagram", width: "100%", height: "100%", "data-diagram": "" })) {
    svg.setAttribute(name, value);
  }
  svg.append(defs, moved);
  const wrap = h("div", { class: "diagram-view", "data-diagram-view": true, style: `height: min(${MAX_HEIGHT}, ${size.height}px)` }, svg);
  // Until the person zooms or moves it, the picture keeps fitting the view as it is resized.
  let touched = false;
  let fitting = false;
  const zoom = new PanZoom(wrap, { x: 0, y: 0, k: 1 }, (view, commit) => {
    moved.setAttribute("transform", `translate(${view.x} ${view.y}) scale(${view.k})`);
    if (commit && !fitting) touched = true;
  });
  const fit = () => {
    fitting = true;
    zoom.fit(size, FIT_MAX_ZOOM);
    fitting = false;
  };
  new ResizeObserver(() => {
    if (!touched && wrap.clientWidth > 0) fit();
  }).observe(wrap);
  return h(
    "div",
    { class: "diagram-viewer" },
    h(
      "div",
      { class: "seg-group diagram-zoom", role: "group", "aria-label": t("map.zoom") },
      h("button", { type: "button", class: "seg", "data-action": "diagram-zoom-out", "aria-label": t("map.zoom-out"), onclick: () => zoom.zoomBy(1 / 1.25) }, "−"),
      h("button", { type: "button", class: "seg", "data-action": "diagram-fit", onclick: fit }, t("map.fit")),
      h("button", { type: "button", class: "seg", "data-action": "diagram-zoom-in", "aria-label": t("map.zoom-in"), onclick: () => zoom.zoomBy(1.25) }, "+"),
    ),
    wrap,
  );
}
