import { h } from "../dom.js";
import { PanZoom, wheelHintKey } from "../pan-zoom.js";
import { translator } from "../strings.js";
import { explanationDiagram } from "./explanation-diagram.js";

export function explanationViewer({ part, identity, lang }) {
  const t = translator(lang);
  const initial = { x: 24, y: 24, k: 1 };
  let current = { ...initial };
  const transform = svg => { svg.style.transform = `translate(${current.x}px, ${current.y}px) scale(${current.k})`; };
  const surface = explanationDiagram(part, lang, { onSvg: transform });
  surface.className = "explanation-viewer-surface";
  surface.setAttribute("tabindex", "0");
  surface.setAttribute("data-focus", `viewer-${identity}`);
  surface.setAttribute("data-explanation-viewer", identity);
  surface.setAttribute("role", "group");
  surface.setAttribute("aria-label", t("part.viewer"));
  const zoom = new PanZoom(surface, initial, value => {
    current = value;
    const svg = surface.querySelector("svg");
    if (svg) transform(svg);
  }, { mode: "canvas", hint: t(wheelHintKey()) });
  const move = (dx, dy) => zoom.set({ ...zoom.view, x: zoom.view.x + dx, y: zoom.view.y + dy }, "now");
  const reset = () => zoom.set({ ...initial }, "now");
  const actions = {
    ArrowLeft: () => move(-40, 0), ArrowRight: () => move(40, 0),
    ArrowUp: () => move(0, -40), ArrowDown: () => move(0, 40),
    "+": () => zoom.zoomBy(1.2), "=": () => zoom.zoomBy(1.2),
    "-": () => zoom.zoomBy(1 / 1.2), "0": reset,
  };
  surface.addEventListener("keydown", event => {
    if (event.target !== surface || event.ctrlKey || event.metaKey || event.altKey) return;
    const action = actions[event.key];
    if (action) { event.preventDefault(); action(); }
  });
  const button = (key, action) => h("button", { type: "button", "data-focus": `${identity}-${key}`, onclick: action }, t(key));
  return h("div", { class: "explanation-viewer" },
    h("h2", { class: "dialog-title" }, part.title),
    h("div", { class: "explanation-viewer-controls", role: "group", "aria-label": t("part.viewer") },
      button("part.left", actions.ArrowLeft), button("part.right", actions.ArrowRight),
      button("part.up", actions.ArrowUp), button("part.down", actions.ArrowDown),
      button("map.zoom-in", actions["+"]), button("map.zoom-out", actions["-"]), button("part.reset", reset)),
    surface);
}
