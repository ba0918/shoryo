// 地図: the decisions and the current questions by round, drawn from the computed map
// (docs/spec/screen.md, "地図"). Hover details float over the map and never move it.
import { Component, h } from "../dom.js";

const SVG = "http://www.w3.org/2000/svg";
const COLUMN_W = 320;
const NODE_W = 190;
const NODE_H = 62;
const ROW_H = 86;
const TOP = 44;
const PAD = 16;
const LINE_CHARS = 24;

function el(name, attrs = {}, ...children) {
  const element = document.createElementNS(SVG, name);
  for (const [key, value] of Object.entries(attrs)) {
    if (value === undefined || value === null || value === false) continue;
    element.setAttribute(key, value === true ? "" : String(value));
  }
  for (const child of children) if (child) element.append(child);
  return element;
}

function text(attrs, content) {
  const element = el("text", attrs);
  element.textContent = content;
  return element;
}

const clip = (value, length) => ([...value].length > length ? `${[...value].slice(0, length - 1).join("")}…` : value);

export class MapTab extends Component {
  draw(map) {
    this.hover = h("div", { class: "map-hover", "data-map-hover": true, hidden: true });
    return h(
      "div",
      { class: "map-tab" },
      h(
        "div",
        { class: "map-ranges" },
        [
          { id: "path", name: "Path" },
          { id: "all", name: "All" },
        ].map((range) =>
          h(
            "button",
            {
              type: "button",
              "data-map-range": range.id,
              "aria-pressed": String(range.id === map.range),
              onclick: () => this.emit({ type: "map-range", range: range.id }),
            },
            range.name,
          ),
        ),
        map.range === "path" && !map.selected
          ? h("span", { class: "label" }, " Select a point to see what it rests on.")
          : null,
      ),
      h("div", { class: "map-wrap", "data-map": true }, this.svg(map), this.hover),
      this.selection(map),
    );
  }

  selection(map) {
    const node = map.columns.flatMap((column) => column.nodes).find((n) => n.key === map.selected);
    return h(
      "div",
      { class: "map-selection" },
      node
        ? [
            h("span", {}, h("span", { class: "label" }, "Selected: "), node.label, " "),
            node.jump
              ? h(
                  "button",
                  { type: "button", "data-action": "jump", onclick: () => this.emit({ type: "jump", target: node.jump }) },
                  "Go to its question",
                )
              : null,
          ]
        : h("span", { class: "label" }, "Click a point to select it."),
    );
  }

  svg(map) {
    const height = TOP + Math.max(1, ...map.columns.map((c) => c.nodes.length)) * ROW_H + PAD;
    const width = PAD * 2 + Math.max(1, map.columns.length) * COLUMN_W;
    const svg = el("svg", { class: "map", width, height, viewBox: `0 0 ${width} ${height}` });
    const at = new Map();
    map.columns.forEach((column, x) => {
      const left = PAD + x * COLUMN_W;
      svg.append(text({ x: left + COLUMN_W / 2, y: 22, class: "column-head", "text-anchor": "middle" }, clip(`Round ${column.round}: ${column.subject}`, 30)));
      column.nodes.forEach((node, y) => at.set(node.key, { x: left + (COLUMN_W - NODE_W) / 2, y: TOP + y * ROW_H, node }));
    });
    const edges = el("g", { class: "edges" });
    const labelled = new Set();
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
          : `M ${start.x} ${start.y} C ${start.x + 60} ${start.y}, ${to.x + NODE_W + 60} ${end.y}, ${to.x + NODE_W} ${end.y}`;
      const group = el(
        "g",
        { class: edge.dashed ? "map-edge dashed" : "map-edge", "data-map-edge": `${edge.from}->${edge.to}` },
        el("path", { d }),
        el("title", {}, edge.label),
      );
      // Every edge into one point carries the same name, so it is written once, beside
      // the point, rather than once per line.
      if (edge.label && !labelled.has(edge.to)) {
        labelled.add(edge.to);
        group.append(text({ x: end.x - 6, y: end.y - 8, class: "edge-label", "text-anchor": "end" }, clip(edge.label, 22)));
      }
      edges.append(group);
    }
    svg.append(edges);
    for (const { x, y, node } of at.values()) svg.append(this.node(node, x, y));
    return svg;
  }

  node(node, x, y) {
    // A question point has no second line, so its own text may take two.
    const chars = [...node.label];
    const lines = node.question
      ? [clip(node.label, LINE_CHARS), clip(node.question, LINE_CHARS + 6)]
      : [chars.slice(0, LINE_CHARS).join(""), clip(chars.slice(LINE_CHARS).join(""), LINE_CHARS)].filter(Boolean);
    const group = el(
      "g",
      {
        class: `map-node ${node.kind}${node.selected ? " selected" : ""}`,
        "data-map-node": node.key,
        "data-selected": node.selected,
        tabindex: 0,
      },
      el("rect", { x, y, width: NODE_W, height: NODE_H, rx: 6 }),
      text({ x: x + 8, y: y + 20, class: "node-label" }, lines[0]),
      lines[1] ? text({ x: x + 8, y: y + 38, class: "node-question" }, lines[1]) : null,
      node.inReview ? text({ x: x + 8, y: y + 55, class: "node-mark review", "data-mark": "in-review" }, "In review") : null,
      node.reviewMark
        ? text({ x: x + NODE_W - 8, y: y + 55, class: "node-mark recheck", "text-anchor": "end", "data-mark": "review" }, "Premise changed")
        : null,
    );
    group.addEventListener("click", () => this.emit({ type: "select-node", key: node.key }));
    group.addEventListener("mouseenter", () => this.showHover(node, x, y));
    group.addEventListener("mouseleave", () => (this.hover.hidden = true));
    return group;
  }

  /// Fills the floating detail box beside the node; nothing else on the screen moves.
  showHover(node, x, y) {
    this.hover.replaceChildren(
      h("p", {}, h("strong", {}, node.label)),
      node.detail.text ? h("p", {}, node.detail.text) : null,
      node.detail.question ? h("p", {}, h("span", { class: "label" }, "Question: "), node.detail.question) : null,
      node.detail.answer ? h("p", {}, h("span", { class: "label" }, "Answer then: "), node.detail.answer) : null,
    );
    this.hover.style.left = `${x + NODE_W + 8}px`;
    this.hover.style.top = `${y}px`;
    this.hover.hidden = false;
  }
}
