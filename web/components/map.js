// 地図: the decisions and the current questions by round, drawn from the computed map
// (docs/spec/screen.md, "地図"). The map zooms and moves inside its own area; details float
// in a layer of their own above the page, so they are never clipped and never move the map.
import { Component, h } from "../dom.js";
import { fitLines } from "../measure.js";
import { PanZoom, clamp, wheelHintKey } from "../pan-zoom.js";
import { translator } from "../strings.js";
import { reviewControls } from "./decision-item.js";

const SVG = "http://www.w3.org/2000/svg";
const COLUMN_W = 360;
const NODE_W = 200;
const NODE_H = 76;
const ROW_H = 100;
const TOP = 48;
const PAD = 24;
const TEXT_X = 8;
const TEXT_W = NODE_W - 2 * TEXT_X;
/// An edge's name sits in the gap left of the point it leads to, inside that point's row.
const LABEL_GAP = 8;
const LABEL_W = COLUMN_W - NODE_W - 2 * LABEL_GAP;
/// The most "fit" zooms in on a small range.
const FIT_MAX_ZOOM = 1.5;
/// Below this zoom the edges' names are left out unless their point is hovered or selected.
const FAR_ZOOM = 0.65;
const WHEEL_IDLE_MS = 250;

export const DEFAULT_VIEW = { x: 0, y: 0, k: 1 };

function el(name, attrs = {}, ...children) {
  const element = document.createElementNS(SVG, name);
  for (const [key, value] of Object.entries(attrs)) {
    if (value === undefined || value === null || value === false) continue;
    element.setAttribute(key, value === true ? "" : String(value));
  }
  for (const child of children) if (child) element.append(child);
  return element;
}

function lines(values, attrs, x, y, step) {
  return values.map((value, index) => {
    const element = el("text", { ...attrs, x, y: y + index * step });
    element.textContent = value;
    return element;
  });
}

export class MapTab extends Component {
  constructor(emit) {
    super(emit);
    this.hover = h("div", { class: "map-hover", "data-map-hover": true, role: "tooltip", hidden: true });
    this.details = h("div", { class: "map-details", "data-map-selection": true, role: "dialog", hidden: true });
    document.body.append(this.hover, this.details);
    const follow = () => {
      this.hover.hidden = true;
      this.placeDetails();
    };
    window.addEventListener("scroll", follow, { passive: true });
    window.addEventListener("resize", follow);
  }

  draw(map) {
    const t = translator(map.lang);
    const view = { ...(map.view ?? DEFAULT_VIEW) };
    this.hover.hidden = true;
    const { svg, size } = this.svg(map, view, t);
    this.size = size;
    const wrap = h("div", { class: "map-wrap", "data-map": true }, svg);
    this.wrap = wrap;
    this.zoom = new PanZoom(wrap, view, (next, commit) => this.apply(next, commit), { hint: t(wheelHintKey()) });
    requestAnimationFrame(() => this.fillDetails(map, t));
    return h(
      "div",
      { class: "map-tab" },
      h(
        "div",
        { class: "map-toolbar" },
        h(
          "div",
          { class: "map-ranges seg-group", role: "group", "aria-label": t("map.range") },
          ["path", "all"].map((range) =>
            h(
              "button",
              {
                type: "button",
                class: `seg${range === map.range ? " on" : ""}`,
                "data-map-range": range,
                "data-focus": `map-range-${range}`,
                "aria-pressed": String(range === map.range),
                onclick: () => this.emit({ type: "map-range", range }),
              },
              t(`map.${range}`),
            ),
          ),
        ),
        h(
          "div",
          { class: "map-zoom seg-group", role: "group", "aria-label": t("map.zoom") },
          h("button", { type: "button", class: "seg", "data-action": "map-zoom-out", "data-focus": "map-zoom-out", "aria-label": t("map.zoom-out"), onclick: () => this.zoom.zoomBy(1 / 1.25) }, "−"),
          h("button", { type: "button", class: "seg", "data-action": "map-fit", "data-focus": "map-fit", onclick: () => this.zoom.fit(this.size, FIT_MAX_ZOOM) }, t("map.fit")),
          h("button", { type: "button", class: "seg", "data-action": "map-zoom-in", "data-focus": "map-zoom-in", "aria-label": t("map.zoom-in"), onclick: () => this.zoom.zoomBy(1.25) }, "+"),
        ),
        h("span", { class: "map-hint" }, map.range === "path" && !map.root ? t("map.pick-for-path") : t("map.how-to-move")),
      ),
      wrap,
    );
  }

  svg(map, view, t) {
    const height = TOP + Math.max(1, ...map.columns.map((c) => c.nodes.length)) * ROW_H + PAD;
    const width = PAD * 2 + Math.max(1, map.columns.length) * COLUMN_W;
    const svg = el("svg", { class: `map${view.k < FAR_ZOOM ? " far" : ""}`, width: "100%", height: "100%" });
    const content = el("g", { class: "map-content", transform: transform(view) });
    this.content = content;
    const at = new Map();
    map.columns.forEach((column, x) => {
      const left = PAD + x * COLUMN_W;
      const [head] = fitLines(t("map.column", { round: column.round, subject: column.subject }), "column-head", COLUMN_W - 16, 1);
      content.append(...lines(head ? [head] : [], { class: "column-head", "text-anchor": "middle" }, left + COLUMN_W / 2, 24, 0));
      column.nodes.forEach((node, y) => at.set(node.key, { x: left + (COLUMN_W - NODE_W) / 2, y: TOP + y * ROW_H, node }));
    });
    const lit = (edge) => map.selected !== null && (edge.from === map.selected || edge.to === map.selected);
    const edges = el("g", { class: "edges" });
    const labels = el("g", { class: "edge-labels" });
    const named = new Set();
    for (const edge of map.edges) {
      const from = at.get(edge.from);
      const to = at.get(edge.to);
      if (!from || !to) continue;
      const start = { x: from.x + NODE_W, y: from.y + NODE_H / 2 };
      const end = { x: to.x, y: to.y + NODE_H / 2 };
      const bend = Math.max(40, Math.abs(end.x - start.x) / 2);
      const d =
        end.x > start.x
          ? `M ${start.x} ${start.y} C ${start.x + bend} ${start.y}, ${end.x - bend} ${end.y}, ${end.x} ${end.y}`
          : `M ${start.x} ${start.y} C ${start.x + 60} ${start.y}, ${end.x - 60} ${end.y}, ${end.x} ${end.y}`;
      const classes = ["map-edge", edge.dashed ? "dashed" : "", lit(edge) ? "lit" : ""].filter(Boolean).join(" ");
      edges.append(
        el(
          "g",
          { class: classes, "data-map-edge": `${edge.from}->${edge.to}`, "data-from": edge.from, "data-to": edge.to },
          el("path", { d }),
          el("title", {}, edge.label),
        ),
      );
      // Every edge into one point carries the same name, so it is written once, in the gap
      // left of that point and within its row, where no other point or name can be.
      if (edge.label && !named.has(edge.to)) {
        named.add(edge.to);
        const group = el("g", { class: `edge-label-group${lit(edge) ? " lit" : ""}`, "data-to": edge.to });
        group.append(...lines(fitLines(edge.label, "edge-label", LABEL_W, 2), { class: "edge-label", "text-anchor": "end" }, to.x - LABEL_GAP, to.y + 14, 13));
        labels.append(group);
      }
    }
    content.append(edges, labels);
    for (const { x, y, node } of at.values()) content.append(this.node(node, x, y, t));
    svg.append(content);
    return { svg, size: { width, height } };
  }

  node(node, x, y, t) {
    const label = fitLines(node.label, "node-label", TEXT_W, 2);
    const question = node.question ? fitLines(node.question, "node-question", TEXT_W, 1) : [];
    const group = el(
      "g",
      {
        class: `map-node ${node.kind}${node.selected ? " selected" : ""}`,
        "data-map-node": node.key,
        "data-focus": `map-node-${node.key}`,
        "data-selected": node.selected,
        tabindex: 0,
        role: "button",
        "aria-label": node.label,
      },
      el("rect", { x, y, width: NODE_W, height: NODE_H, rx: 6 }),
      ...lines(label, { class: "node-label" }, x + TEXT_X, y + 18, 15),
      ...lines(question, { class: "node-question" }, x + TEXT_X, y + 50, 0),
      node.inReview
        ? el("text", { x: x + TEXT_X, y: y + 67, class: "node-mark review", "data-mark": "in-review" }, t("map.in-review"))
        : null,
      node.reviewMark
        ? el(
            "text",
            { x: x + NODE_W - TEXT_X, y: y + 67, class: "node-mark recheck", "text-anchor": "end", "data-mark": "review" },
            t("map.premise-changed"),
          )
        : null,
    );
    group.addEventListener("click", () => {
      if (this.zoom.dragged) return;
      this.emit({ type: "select-node", key: node.key });
    });
    group.addEventListener("keydown", (event) => {
      if (event.key === "Enter" || event.key === " ") {
        event.preventDefault();
        this.emit({ type: "select-node", key: node.key });
      }
    });
    group.addEventListener("pointerenter", (event) => {
      if (event.pointerType === "mouse" && !this.zoom.panning) this.showHover(node, group, t);
    });
    group.addEventListener("pointerleave", () => this.hideHover(node));
    group.addEventListener("focus", () => {
      this.zoom.reveal(group);
      this.showHover(node, group, t);
    });
    group.addEventListener("blur", () => this.hideHover(node));
    return group;
  }

  /// The lines a point's details show; an empty line is left out.
  detailLines(node, t) {
    const detail = node.detail;
    return [
      h("p", { class: "details-title" }, node.label),
      detail.text ? h("p", {}, detail.text) : null,
      detail.question && detail.question !== node.label
        ? h("p", {}, h("span", { class: "meta-label" }, t("map.question")), " ", detail.question)
        : null,
      detail.answer ? h("p", {}, h("span", { class: "meta-label" }, t("map.answer-then")), " ", detail.answer) : null,
    ];
  }

  showHover(node, group, t) {
    if (node.selected) return;
    this.hover.replaceChildren(...this.detailLines(node, t).filter(Boolean));
    this.hover.hidden = false;
    this.lightEdges(node.key, true);
    place(this.hover, group.getBoundingClientRect());
  }

  hideHover(node) {
    this.hover.hidden = true;
    this.lightEdges(node.key, false);
  }

  lightEdges(key, on) {
    for (const part of this.content?.querySelectorAll(`[data-from="${CSS.escape(key)}"], [data-to="${CSS.escape(key)}"]`) ?? []) {
      part.classList.toggle("hovered", on);
    }
  }

  /// The selected point's details, with going to its path, to its question, and 見直す.
  fillDetails(map, t) {
    const active = document.activeElement;
    const focused = this.details.contains(active) ? active.dataset.focus : undefined;
    const node = map.columns.flatMap((column) => column.nodes).find((n) => n.key === map.selected);
    if (!node || !map.visible) {
      this.details.hidden = true;
      this.details.replaceChildren();
      return;
    }
    this.details.replaceChildren(
      h(
        "button",
        { type: "button", class: "btn quiet small details-close", "data-action": "close-details", "data-focus": `close-details-${node.key}`, onclick: () => this.emit({ type: "select-node", key: null }) },
        t("dialog.close"),
      ),
      ...this.detailLines(node, t).filter(Boolean),
      h(
        "div",
        { class: "details-actions" },
        map.range === "path" && node.key === map.root
          ? null
          : h(
              "button",
              { type: "button", class: "btn small", "data-action": "show-path", "data-focus": `show-path-${node.key}`, onclick: () => this.emit({ type: "show-path", key: node.key }) },
              t("map.show-path"),
            ),
        node.jump
          ? h("button", { type: "button", class: "btn small", "data-action": "jump", "data-focus": `jump-${node.key}`, onclick: () => this.emit({ type: "jump", target: node.jump }) }, t("map.go-to-question"))
          : null,
        reviewControls(node.review, this.emit),
      ),
    );
    this.details.hidden = false;
    this.placeDetails();
    if (focused) this.details.querySelector(`[data-focus="${CSS.escape(focused)}"]`)?.focus({ preventScroll: true });
  }

  placeDetails() {
    if (this.details.hidden || !this.data) return;
    const group = this.wrap?.querySelector(`[data-map-node="${CSS.escape(this.data.selected ?? "")}"]`);
    if (!group || !this.wrap.isConnected) {
      this.details.hidden = true;
      return;
    }
    place(this.details, group.getBoundingClientRect());
  }

  /// Shows a view while a gesture goes on; `commit` hands it up once the gesture ends, which
  /// is what "Back" returns to.
  apply(view, commit) {
    this.content.setAttribute("transform", transform(view));
    this.content.ownerSVGElement.classList.toggle("far", view.k < FAR_ZOOM);
    this.hover.hidden = true;
    this.placeDetails();
    clearTimeout(this.idle);
    if (commit === "now") this.emit({ type: "map-view", view: { ...view } });
    if (commit === "idle") this.idle = setTimeout(() => this.emit({ type: "map-view", view: { ...view } }), WHEEL_IDLE_MS);
  }
}

const transform = (view) => `translate(${view.x} ${view.y}) scale(${view.k})`;

/// Puts a floating box beside `anchor` (a viewport rectangle): right of it when there is
/// room, otherwise left, and always inside the viewport below the fixed header. A box taller
/// than that room is cut to it and scrolls inside, so all of its text and buttons can be reached.
function place(box, anchor) {
  const margin = 8;
  const top = (document.querySelector(".topbar")?.getBoundingClientRect().bottom ?? 0) + margin;
  box.style.left = "0px";
  box.style.top = "0px";
  box.style.maxHeight = `${Math.max(0, window.innerHeight - margin - top)}px`;
  const width = box.offsetWidth;
  const height = box.offsetHeight;
  let left = anchor.right + margin;
  if (left + width > window.innerWidth - margin) left = anchor.left - margin - width;
  left = clamp(left, margin, Math.max(margin, window.innerWidth - margin - width));
  const y = clamp(anchor.top, top, Math.max(top, window.innerHeight - margin - height));
  box.style.left = `${left}px`;
  box.style.top = `${y}px`;
}
