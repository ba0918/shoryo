// Draws the diagrams the LLM writes (in replies and the finished picture) at the grid
// positions it gave, never re-arranged (docs/spec/screen.md, "図").
//
// The text, one item per line:
//   id = label        a node
//   id = ? label      an empty slot, drawn as a dashed frame
//   | a | b | . |     one grid row; "." is an empty cell
//   a -> b : label    an edge (the label is optional)
// Blank lines are ignored.

import { fitLines, textWidth } from "./measure.js";

const SVG = "http://www.w3.org/2000/svg";
const CELL_W = 250;
const CELL_H = 130;
const BOX_W = 150;
const TEXT_PAD_X = 8;
const TEXT_PAD_Y = 10;
const TEXT_W = BOX_W - 2 * TEXT_PAD_X;
const LINE_H = 15;
const MIN_BOX_H = 44;
/// A box grows to this many lines; a longer label is cut with "…" and shown whole on hover
/// or tap.
const MAX_LINES = 3;
/// The margin around what is drawn.
const PAD = 16;
/// How far apart two edges between the same two nodes in opposite directions are drawn.
const REVERSE_OFFSET = 8;
const LABEL_GAP = 6;
const LABEL_H = 14;

export function parseDiagram(text) {
  const nodes = new Map();
  const rows = [];
  const edges = [];
  for (const raw of text.split("\n")) {
    const line = raw.trim();
    if (line === "") continue;
    if (line.startsWith("|")) {
      rows.push(
        line
          .replace(/^\|/, "")
          .replace(/\|$/, "")
          .split("|")
          .map((cell) => cell.trim()),
      );
      continue;
    }
    const edge = line.match(/^(\S+)\s*->\s*(\S+?)\s*(?::\s*(.*))?$/);
    if (edge) {
      edges.push({ from: edge[1], to: edge[2], label: edge[3] ?? "" });
      continue;
    }
    const node = line.match(/^(\S+)\s*=\s*(\?)?\s*(.*)$/);
    if (node) nodes.set(node[1], { label: node[3], slot: node[2] === "?" });
  }
  return { nodes, rows, edges };
}

/// Where each node sits: its first cell in the grid; nodes the grid leaves out go in one
/// row below it, in the order they were declared.
function positions({ nodes, rows }) {
  const at = new Map();
  rows.forEach((row, y) =>
    row.forEach((id, x) => {
      if (id !== "." && id !== "" && !at.has(id)) at.set(id, { x, y });
    }),
  );
  let extra = 0;
  for (const id of nodes.keys()) {
    if (!at.has(id)) at.set(id, { x: extra++, y: rows.length });
  }
  return at;
}

function el(name, attrs = {}, ...children) {
  const element = document.createElementNS(SVG, name);
  for (const [key, value] of Object.entries(attrs)) element.setAttribute(key, String(value));
  for (const child of children) element.append(child);
  return element;
}

const boxHeight = (lineCount) => Math.max(MIN_BOX_H, lineCount * LINE_H + 2 * TEXT_PAD_Y);

const centre = ({ x, y }) => ({ x: x * CELL_W + CELL_W / 2, y: y * CELL_H + CELL_H / 2 });

/// The point where a ray from the centre of a box `height` tall in direction (dx, dy) leaves it.
function border(point, height, dx, dy) {
  const hw = BOX_W / 2 + 3;
  const hh = height / 2 + 3;
  const t = Math.min(dx === 0 ? Infinity : hw / Math.abs(dx), dy === 0 ? Infinity : hh / Math.abs(dy));
  return { x: point.x + dx * t, y: point.y + dy * t };
}

/// A label's lines centred on (x, y).
function labelText(lines, x, y, cls) {
  const text = el("text", { class: cls, x, y: y - ((lines.length - 1) * LINE_H) / 2, "text-anchor": "middle", "dominant-baseline": "middle" });
  lines.forEach((line, index) => {
    const span = el("tspan", { x, dy: index === 0 ? 0 : LINE_H });
    span.textContent = line;
    text.append(span);
  });
  return text;
}

/// Grows `bounds` to take in a rectangle.
function take(bounds, x, y, width, height) {
  bounds.left = Math.min(bounds.left, x);
  bounds.top = Math.min(bounds.top, y);
  bounds.right = Math.max(bounds.right, x + width);
  bounds.bottom = Math.max(bounds.bottom, y + height);
}

/// Draws a diagram's text. Returns the arrowhead's definition, the drawing moved so that what
/// is drawn starts at the margin, and the size with the margin around it. While a tapped
/// label shows whole, what is drawn reaches further; `onExtent` then receives the area, in
/// the drawing's coordinates, that takes it in with the margin around it.
export function drawDiagram(text, onExtent = () => {}) {
  const diagram = parseDiagram(text);
  const at = positions(diagram);
  const nodes = [...at].map(([id, point]) => {
    const node = diagram.nodes.get(id) ?? { label: id, slot: false };
    const cut = fitLines(node.label, "node-label", TEXT_W, MAX_LINES, "diagram");
    const whole = fitLines(node.label, "node-label", TEXT_W, Infinity, "diagram");
    return { id, point, node, cut, whole };
  });
  // Every box in a grid row is as tall as the row's tallest, so the row stays one line.
  const rowHeights = new Map();
  for (const { point, cut } of nodes) {
    rowHeights.set(point.y, Math.max(rowHeights.get(point.y) ?? 0, boxHeight(cut.length)));
  }
  const heightAt = (point) => rowHeights.get(point.y);
  const bounds = { left: Infinity, top: Infinity, right: -Infinity, bottom: -Infinity };

  const edges = el("g", { class: "edges" });
  const pairs = new Set(diagram.edges.map((edge) => `${edge.from}->${edge.to}`));
  for (const edge of diagram.edges) {
    const from = at.get(edge.from);
    const to = at.get(edge.to);
    if (!from || !to || edge.from === edge.to) continue;
    const a = centre(from);
    const b = centre(to);
    const length = Math.hypot(b.x - a.x, b.y - a.y);
    const dx = (b.x - a.x) / length;
    const dy = (b.y - a.y) / length;
    // The normal to the left of the direction of travel; an edge and its reverse therefore
    // shift to opposite sides.
    const nx = -dy;
    const ny = dx;
    const shift = pairs.has(`${edge.to}->${edge.from}`) ? REVERSE_OFFSET : 0;
    const start = border(a, heightAt(from), dx, dy);
    const end = border(b, heightAt(to), -dx, -dy);
    const line = el("line", {
      x1: start.x + nx * shift,
      y1: start.y + ny * shift,
      x2: end.x + nx * shift,
      y2: end.y + ny * shift,
      "marker-end": "url(#arrow)",
    });
    const group = el("g", { class: "edge", "data-edge": `${edge.from}->${edge.to}` }, line);
    if (edge.label) {
      const mid = { x: (start.x + end.x) / 2 + nx * (shift + LABEL_GAP), y: (start.y + end.y) / 2 + ny * (shift + LABEL_GAP) };
      // Put the text wholly on the normal's side of the line, so it never sits on it.
      const anchor = Math.abs(nx) > 0.5 ? (nx > 0 ? "start" : "end") : "middle";
      const baseline = Math.abs(ny) >= 0.5 ? (ny > 0 ? "hanging" : "alphabetic") : "middle";
      const label = el("text", { class: "edge-label", x: mid.x, y: mid.y, "text-anchor": anchor, "dominant-baseline": baseline });
      label.textContent = edge.label;
      group.append(label);
      const width = textWidth(edge.label, "edge-label", "diagram");
      const left = anchor === "start" ? mid.x : anchor === "end" ? mid.x - width : mid.x - width / 2;
      const top = baseline === "hanging" ? mid.y : baseline === "alphabetic" ? mid.y - LABEL_H : mid.y - LABEL_H / 2;
      take(bounds, left, top, width, LABEL_H);
    }
    edges.append(group);
  }

  const boxes = el("g", { class: "nodes" });
  /// The grown box of each label showing whole.
  const grownBoxes = new Map();
  const extent = () => {
    const reach = { ...bounds };
    for (const box of grownBoxes.values()) take(reach, box.x, box.y, box.width, box.height);
    return {
      x: reach.left - bounds.left,
      y: reach.top - bounds.top,
      width: reach.right - reach.left + 2 * PAD,
      height: reach.bottom - reach.top + 2 * PAD,
    };
  };
  for (const { id, point, node, cut, whole } of nodes) {
    const c = centre(point);
    const height = heightAt(point);
    const rect = el("rect", { x: c.x - BOX_W / 2, y: c.y - height / 2, width: BOX_W, height, rx: 6 });
    const group = el("g", { class: node.slot ? "node slot" : "node", "data-node": id }, rect, labelText(cut, c.x, c.y, "node-label"));
    take(bounds, c.x - BOX_W / 2, c.y - height / 2, BOX_W, height);
    if (whole.length > cut.length) {
      group.prepend(el("title", {}, node.label));
      group.classList.add("cut");
      group.setAttribute("tabindex", "0");
      group.setAttribute("role", "button");
      group.setAttribute("aria-label", node.label);
      group.setAttribute("data-focus", `diagram-node-${id}`);
      group.setAttribute("aria-expanded", "false");
      const full = labelText(whole, c.x, c.y, "node-label whole");
      group.append(full);
      const toggle = () => {
        const focused = document.activeElement === group;
        const showing = group.classList.toggle("showing-whole");
        group.setAttribute("aria-expanded", String(showing));
        const grown = showing ? boxHeight(whole.length) : height;
        rect.setAttribute("y", String(c.y - grown / 2));
        rect.setAttribute("height", String(grown));
        if (showing) {
          group.parentNode?.append(group);
          grownBoxes.set(id, { x: c.x - BOX_W / 2, y: c.y - grown / 2, width: BOX_W, height: grown });
        } else {
          grownBoxes.delete(id);
        }
        onExtent(extent());
        if (focused) group.focus({ preventScroll: true });
      };
      group.addEventListener("click", toggle);
      group.addEventListener("keydown", (event) => {
        if (event.key !== "Enter" && event.key !== " ") return;
        event.preventDefault();
        toggle();
      });
    }
    boxes.append(group);
  }

  if (bounds.left === Infinity) take(bounds, 0, 0, 0, 0);
  const content = el("g", { transform: `translate(${PAD - bounds.left} ${PAD - bounds.top})` }, edges, boxes);
  const defs = el(
    "defs",
    {},
    el(
      "marker",
      { id: "arrow", viewBox: "0 0 10 10", refX: 9, refY: 5, markerWidth: 7, markerHeight: 7, orient: "auto-start-reverse" },
      el("path", { d: "M 0 0 L 10 5 L 0 10 z", class: "arrow-head" }),
    ),
  );
  const size = { width: bounds.right - bounds.left + 2 * PAD, height: bounds.bottom - bounds.top + 2 * PAD };
  return { defs, content, size };
}

/// A diagram at its own size, which the page may shrink to fit (a reply's diagram).
/// It grows to take in a label shown whole.
export function renderDiagram(text) {
  const svg = el("svg", { class: "diagram", "data-diagram": "" });
  const frame = ({ x, y, width, height }) => {
    svg.setAttribute("viewBox", `${x} ${y} ${width} ${height}`);
    svg.setAttribute("width", String(width));
    svg.setAttribute("height", String(height));
  };
  const { defs, content, size } = drawDiagram(text, frame);
  frame({ x: 0, y: 0, ...size });
  svg.append(defs, content);
  return svg;
}
