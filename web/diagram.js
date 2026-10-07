// Draws the diagrams the LLM writes (in replies and the finished picture) at the grid
// positions it gave, never re-arranged (docs/spec/screen.md, "図").
//
// The text, one item per line:
//   id = label        a node
//   id = ? label      an empty slot, drawn as a dashed frame
//   | a | b | . |     one grid row; "." is an empty cell
//   a -> b : label    an edge (the label is optional)
// Blank lines are ignored.

const SVG = "http://www.w3.org/2000/svg";
const CELL_W = 180;
const CELL_H = 100;
const BOX_W = 150;
const BOX_H = 64;
const PAD = 24;
const LINE_CHARS = 18;
const MAX_LINES = 3;
/// How far apart two edges between the same two nodes in opposite directions are drawn.
const REVERSE_OFFSET = 8;
const LABEL_GAP = 6;

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

function wrap(label) {
  const chars = [...label];
  const lines = [];
  for (let i = 0; i < chars.length && lines.length < MAX_LINES; i += LINE_CHARS) {
    lines.push(chars.slice(i, i + LINE_CHARS).join(""));
  }
  if (chars.length > LINE_CHARS * MAX_LINES) lines[MAX_LINES - 1] += "…";
  return lines;
}

const centre = ({ x, y }) => ({ x: PAD + x * CELL_W + CELL_W / 2, y: PAD + y * CELL_H + CELL_H / 2 });

/// The point where a ray from the box centre in direction (dx, dy) leaves the box.
function border(point, dx, dy) {
  const hw = BOX_W / 2 + 3;
  const hh = BOX_H / 2 + 3;
  const t = Math.min(dx === 0 ? Infinity : hw / Math.abs(dx), dy === 0 ? Infinity : hh / Math.abs(dy));
  return { x: point.x + dx * t, y: point.y + dy * t };
}

export function renderDiagram(text) {
  const diagram = parseDiagram(text);
  const at = positions(diagram);
  const columns = Math.max(1, ...[...at.values()].map((p) => p.x + 1));
  const rowCount = Math.max(1, ...[...at.values()].map((p) => p.y + 1));
  const width = PAD * 2 + columns * CELL_W;
  const height = PAD * 2 + rowCount * CELL_H;
  const svg = el("svg", {
    class: "diagram",
    viewBox: `0 0 ${width} ${height}`,
    width,
    height,
    "data-diagram": "",
  });
  svg.append(
    el(
      "defs",
      {},
      el(
        "marker",
        { id: "arrow", viewBox: "0 0 10 10", refX: 9, refY: 5, markerWidth: 7, markerHeight: 7, orient: "auto-start-reverse" },
        el("path", { d: "M 0 0 L 10 5 L 0 10 z", class: "arrow-head" }),
      ),
    ),
  );

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
    const start = border(a, dx, dy);
    const end = border(b, -dx, -dy);
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
      const label = el("text", { x: mid.x, y: mid.y, "text-anchor": anchor, "dominant-baseline": baseline });
      label.textContent = edge.label;
      group.append(label);
    }
    svg.append(group);
  }

  for (const [id, point] of at) {
    const node = diagram.nodes.get(id) ?? { label: id, slot: false };
    const c = centre(point);
    const lines = wrap(node.label);
    const text = el("text", { x: c.x, y: c.y - ((lines.length - 1) * 15) / 2, "text-anchor": "middle", "dominant-baseline": "middle" });
    lines.forEach((line, index) => {
      const span = el("tspan", { x: c.x, dy: index === 0 ? 0 : 15 });
      span.textContent = line;
      text.append(span);
    });
    svg.append(
      el(
        "g",
        { class: node.slot ? "node slot" : "node", "data-node": id },
        el("rect", { x: c.x - BOX_W / 2, y: c.y - BOX_H / 2, width: BOX_W, height: BOX_H, rx: 6 }),
        text,
      ),
    );
  }
  return svg;
}
