// The finished picture in a view of its own: the whole picture fitted in at first, zoomed and
// moved the way the map is (docs/spec/screen.md, "完成図"). The zoom is this view's own state.
import { h } from "../dom.js";
import { drawDiagram } from "../diagram.js";
import { PanZoom, wheelHintKey } from "../pan-zoom.js";
import { translator } from "../strings.js";
import { zoomControls } from './diagram-controls.js';

const SVG = "http://www.w3.org/2000/svg";
/// "Fit" never draws the picture larger than its own size.
const FIT_MAX_ZOOM = 1;
/// The view is never taller than this share of the window.
const MAX_HEIGHT = "80vh";
const PREFERRED_MIN_HEIGHT = 260;

export function diagramView(text, lang, mode = "page") {
  const t = translator(lang);
  const { defs, content, size } = drawDiagram(text);
  const moved = document.createElementNS(SVG, "g");
  moved.append(content);
  const svg = document.createElementNS(SVG, "svg");
  for (const [name, value] of Object.entries({ class: "diagram", width: "100%", height: "100%", "data-diagram": "" })) {
    svg.setAttribute(name, value);
  }
  svg.append(defs, moved);
  const wrap = h("div", { class: `diagram-view ${mode}`, "data-diagram-view": true, style: `height: min(${MAX_HEIGHT}, ${Math.max(PREFERRED_MIN_HEIGHT, size.height)}px)` }, svg);
  // Until the person zooms or moves it, the picture keeps fitting the view as it is resized.
  let touched = false;
  let fitting = false;
  let controls;
  const zoom = new PanZoom(wrap, { x: 0, y: 0, k: 1 }, (view, commit) => {
    moved.setAttribute("transform", `translate(${view.x} ${view.y}) scale(${view.k})`);
    controls?.setScale(view.k);
    if (commit && !fitting) touched = true;
  }, { mode, hint: t(wheelHintKey()) });
  const fit = () => {
    fitting = true;
    zoom.fit(size, FIT_MAX_ZOOM);
    fitting = false;
  };
  new ResizeObserver(() => {
    if (!touched && wrap.clientWidth > 0) fit();
  }).observe(wrap);
  controls = zoomControls({ lang, scale: zoom.view.k, out: () => zoom.zoomBy(1 / 1.25), whole: fit, into: () => zoom.zoomBy(1.25), prefix: 'diagram' });
  controls.el.classList.add('diagram-zoom');
  return h(
    "div",
    { class: "diagram-viewer" },
    controls.el,
    wrap,
  );
}
