// 地図: the decisions and the current questions by round, drawn from the computed map
// (docs/spec/screen.md, "地図"). The map zooms and moves inside its own area; details float
// in a layer of their own above the page, so they are never clipped and never move the map.
import { Component, h } from "../dom.js";
import { fitLines } from "../measure.js";
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
/// The least zoom the person can reach by hand; "fit" goes below it when the whole range
/// needs that, and zooming in or out from there never jumps back up to it.
const MIN_ZOOM = 0.2;
const MAX_ZOOM = 3;
/// A floor for "fit", so an area with no size never gives a zoom of zero.
const FIT_MIN_ZOOM = 0.01;
/// Below this zoom the edges' names are left out unless their point is hovered or selected.
const FAR_ZOOM = 0.65;
/// How far a pointer moves before a press becomes a drag rather than a click.
const DRAG_PX = 4;
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

const clamp = (value, low, high) => Math.min(high, Math.max(low, value));

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
    this.view = { ...(map.view ?? DEFAULT_VIEW) };
    this.hover.hidden = true;
    const { svg, size } = this.svg(map, t);
    this.size = size;
    const wrap = h("div", { class: "map-wrap", "data-map": true }, svg);
    this.wrap = wrap;
    this.listen(wrap);
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
          h("button", { type: "button", class: "seg", "data-action": "map-zoom-out", "data-focus": "map-zoom-out", "aria-label": t("map.zoom-out"), onclick: () => this.zoomBy(1 / 1.25) }, "−"),
          h("button", { type: "button", class: "seg", "data-action": "map-fit", "data-focus": "map-fit", onclick: () => this.fit() }, t("map.fit")),
          h("button", { type: "button", class: "seg", "data-action": "map-zoom-in", "data-focus": "map-zoom-in", "aria-label": t("map.zoom-in"), onclick: () => this.zoomBy(1.25) }, "+"),
        ),
        h("span", { class: "map-hint" }, map.range === "path" && !map.root ? t("map.pick-for-path") : t("map.how-to-move")),
      ),
      wrap,
    );
  }

  svg(map, t) {
    const height = TOP + Math.max(1, ...map.columns.map((c) => c.nodes.length)) * ROW_H + PAD;
    const width = PAD * 2 + Math.max(1, map.columns.length) * COLUMN_W;
    const k = this.view.k;
    const svg = el("svg", { class: `map${k < FAR_ZOOM ? " far" : ""}`, width: "100%", height: "100%" });
    const content = el("g", { class: "map-content", transform: this.transform() });
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
      if (this.dragged) return;
      this.emit({ type: "select-node", key: node.key });
    });
    group.addEventListener("keydown", (event) => {
      if (event.key === "Enter" || event.key === " ") {
        event.preventDefault();
        this.emit({ type: "select-node", key: node.key });
      }
    });
    group.addEventListener("pointerenter", (event) => {
      if (event.pointerType === "mouse" && !this.panning) this.showHover(node, group, t);
    });
    group.addEventListener("pointerleave", () => this.hideHover(node));
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

  /// The selected point's details, with going to its path, to its question, and 見直したい.
  fillDetails(map, t) {
    const node = map.columns.flatMap((column) => column.nodes).find((n) => n.key === map.selected);
    if (!node || !map.visible) {
      this.details.hidden = true;
      this.details.replaceChildren();
      return;
    }
    this.details.replaceChildren(
      h(
        "button",
        { type: "button", class: "btn quiet small details-close", "data-action": "close-details", onclick: () => this.emit({ type: "select-node", key: null }) },
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
              { type: "button", class: "btn small", "data-action": "show-path", onclick: () => this.emit({ type: "show-path", key: node.key }) },
              t("map.show-path"),
            ),
        node.jump
          ? h("button", { type: "button", class: "btn small", "data-action": "jump", onclick: () => this.emit({ type: "jump", target: node.jump }) }, t("map.go-to-question"))
          : null,
        reviewControls(node.review, this.emit),
      ),
    );
    this.details.hidden = false;
    this.placeDetails();
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

  transform(view = this.view) {
    return `translate(${view.x} ${view.y}) scale(${view.k})`;
  }

  /// Shows a view while a gesture goes on; `commit` hands it up once the gesture ends, which
  /// is what "Back" returns to.
  apply(view, commit) {
    this.view = view;
    this.content.setAttribute("transform", this.transform());
    this.content.ownerSVGElement.classList.toggle("far", view.k < FAR_ZOOM);
    this.hover.hidden = true;
    this.placeDetails();
    clearTimeout(this.idle);
    if (commit === "now") this.emit({ type: "map-view", view: { ...this.view } });
    if (commit === "idle") this.idle = setTimeout(() => this.emit({ type: "map-view", view: { ...this.view } }), WHEEL_IDLE_MS);
  }

  zoomAt(factor, cx, cy, commit) {
    const v = this.view;
    const k = clamp(v.k * factor, Math.min(MIN_ZOOM, v.k), MAX_ZOOM);
    const ratio = k / v.k;
    this.apply({ k, x: cx - (cx - v.x) * ratio, y: cy - (cy - v.y) * ratio }, commit);
  }

  zoomBy(factor) {
    const box = this.wrap.getBoundingClientRect();
    this.zoomAt(factor, box.width / 2, box.height / 2, "now");
  }

  /// Fits every point of the range into the area.
  fit() {
    const box = this.wrap.getBoundingClientRect();
    const k = clamp(Math.min(box.width / this.size.width, box.height / this.size.height), FIT_MIN_ZOOM, 1.5);
    this.apply({ k, x: (box.width - this.size.width * k) / 2, y: (box.height - this.size.height * k) / 2 }, "now");
  }

  /// Wheel and pinch zoom, and mouse or touch drag to move, all inside the map's area.
  listen(wrap) {
    const pointers = new Map();
    let gesture = null;
    const local = (event) => {
      const box = wrap.getBoundingClientRect();
      return { x: event.clientX - box.left, y: event.clientY - box.top };
    };
    wrap.addEventListener(
      "wheel",
      (event) => {
        event.preventDefault();
        const p = local(event);
        this.zoomAt(Math.exp(-event.deltaY * 0.0015), p.x, p.y, "idle");
      },
      { passive: false },
    );
    wrap.addEventListener("pointerdown", (event) => {
      if (event.button !== 0) return;
      pointers.set(event.pointerId, local(event));
      this.dragged = false;
      gesture = { view: { ...this.view }, start: [...pointers.values()].map((p) => ({ ...p })) };
    });
    wrap.addEventListener("pointermove", (event) => {
      if (!pointers.has(event.pointerId) || !gesture) return;
      pointers.set(event.pointerId, local(event));
      const now = [...pointers.values()];
      if (now.length === 1 && gesture.start.length === 1) {
        const dx = now[0].x - gesture.start[0].x;
        const dy = now[0].y - gesture.start[0].y;
        if (!this.panning && Math.hypot(dx, dy) < DRAG_PX) return;
        if (!this.panning) wrap.setPointerCapture(event.pointerId);
        this.panning = true;
        this.dragged = true;
        this.apply({ ...gesture.view, x: gesture.view.x + dx, y: gesture.view.y + dy });
      } else if (now.length === 2) {
        if (gesture.start.length !== 2) gesture = { view: { ...this.view }, start: now.map((p) => ({ ...p })) };
        const [a, b] = gesture.start;
        const [c, d] = now;
        const before = Math.hypot(a.x - b.x, a.y - b.y) || 1;
        const factor = Math.hypot(c.x - d.x, c.y - d.y) / before;
        const k = clamp(gesture.view.k * factor, Math.min(MIN_ZOOM, gesture.view.k), MAX_ZOOM);
        const mid = { x: (a.x + b.x) / 2, y: (a.y + b.y) / 2 };
        const to = { x: (c.x + d.x) / 2, y: (c.y + d.y) / 2 };
        const ratio = k / gesture.view.k;
        this.panning = true;
        this.dragged = true;
        this.apply({ k, x: to.x - (mid.x - gesture.view.x) * ratio, y: to.y - (mid.y - gesture.view.y) * ratio });
      }
    });
    const end = (event) => {
      if (!pointers.delete(event.pointerId)) return;
      if (pointers.size > 0) {
        gesture = { view: { ...this.view }, start: [...pointers.values()].map((p) => ({ ...p })) };
        return;
      }
      gesture = null;
      if (this.panning) {
        this.panning = false;
        this.apply(this.view, "now");
        // The click that ends a drag is not a selection.
        setTimeout(() => (this.dragged = false), 0);
      }
    };
    wrap.addEventListener("pointerup", end);
    wrap.addEventListener("pointercancel", end);
  }
}

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
