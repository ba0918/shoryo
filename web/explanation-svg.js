const NS = "http://www.w3.org/2000/svg";

export function svgElement(tag, attrs = {}, ...children) {
  const node = document.createElementNS(NS, tag);
  for (const [key, value] of Object.entries(attrs)) node.setAttribute(key, String(value));
  for (const child of children.flat()) node.append(typeof child === "string" ? document.createTextNode(child) : child);
  return node;
}

export function textMetrics(svg) {
  const probe = svgElement("text", { "xml:space": "preserve", "dominant-baseline": "text-before-edge" });
  svg.append(probe);
  const size = Number.parseFloat(getComputedStyle(probe).fontSize);
  if (!Number.isFinite(size) || size <= 0) throw new Error("Text measurement unavailable");
  const scale = size / 14;
  const cache = new Map();
  function width(value) {
    if (!cache.has(value)) {
      probe.textContent = value;
      const measured = probe.getComputedTextLength();
      if (!Number.isFinite(measured) || (value.trim() && measured <= 0)) throw new Error("Text measurement unavailable");
      cache.set(value, measured);
    }
    return cache.get(value);
  }
  function wrap(value, limit) {
    const lines = [];
    for (const run of value.split("\n")) {
      let rest = [...run];
      if (!rest.length) lines.push("");
      while (rest.length) {
        let lo = 0, hi = rest.length;
        while (lo < hi) {
          const mid = Math.ceil((lo + hi) / 2);
          if (width(rest.slice(0, mid).join("")) <= limit) lo = mid;
          else hi = mid - 1;
        }
        if (lo === 0) throw new Error("A character cannot fit its text region");
        let count = lo;
        if (lo < rest.length) {
          for (let i = lo - 1; i >= 0; i--) if (/\s/u.test(rest[i])) { count = i + 1; break; }
        }
        lines.push(rest.slice(0, count).join(""));
        rest = rest.slice(count);
      }
    }
    return { lines, width: Math.max(0, ...lines.map(width)), height: lines.length * 18 * scale, original: value };
  }
  return { scale, width, wrap, dispose: () => probe.remove() };
}

export function drawing(svg, metrics) {
  const shapes = [], labels = [], arrows = [], frames = [];
  function text(block, x, y, attrs = {}, owner = null) {
    const element = svgElement("text", { x, y, "dominant-baseline": "text-before-edge", "xml:space": "preserve", "data-original": block.original, ...attrs }, block.lines.map((line, index) => svgElement("tspan", { x, y: y + index * 18 * metrics.scale }, line)));
    svg.append(element);
    labels.push({ element, block, x, y, owner });
    return element;
  }
  function arrow(points, attrs = {}, endpoints = []) {
    const path = svgElement("path", { d: points.map((p, i) => `${i ? "L" : "M"} ${p.x} ${p.y}`).join(" "), fill: "none", "stroke-width": 1.5, ...attrs });
    svg.append(path);
    const a = points.at(-2), b = points.at(-1);
    const length = Math.hypot(b.x - a.x, b.y - a.y);
    if (!length) throw new Error("Arrow has no visible final segment");
    const ux = (b.x - a.x) / length, uy = (b.y - a.y) / length;
    const head = [{ x: b.x, y: b.y }, { x: b.x - 8 * ux + 3 * uy, y: b.y - 8 * uy - 3 * ux }, { x: b.x - 8 * ux - 3 * uy, y: b.y - 8 * uy + 3 * ux }];
    svg.append(svgElement("polygon", { points: head.map(p => `${p.x},${p.y}`).join(" "), class: "explanation-arrowhead" }));
    arrows.push({ points, head, endpoints });
  }
  function shape(value, element) { shapes.push(value); svg.append(element); }
  function finish(canvas) {
    metrics.dispose();
    const bounds = svg.getBBox();
    const width = canvas?.width ?? Math.max(64, Math.ceil(bounds.x + bounds.width + 24));
    const height = canvas?.height ?? Math.max(64, Math.ceil(bounds.y + bounds.height + 24));
    svg.setAttribute("width", width);
    svg.setAttribute("height", height);
    svg.setAttribute("viewBox", `0 0 ${width} ${height}`);
    return { svg, shapes, labels, arrows, frames, width, height };
  }
  return { svg, metrics, shapes, labels, arrows, frames, text, arrow, shape, finish };
}
