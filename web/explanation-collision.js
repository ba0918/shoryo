const EPS = 0.5;

export function corners(box) {
  return [{ x: box.x, y: box.y }, { x: box.x + box.width, y: box.y }, { x: box.x + box.width, y: box.y + box.height }, { x: box.x, y: box.y + box.height }];
}

function polygon(shape) {
  const { x, y, width: w, height: h } = shape;
  if (shape.kind === "decision") return [{ x, y: y - h / 2 }, { x: x + w / 2, y }, { x, y: y + h / 2 }, { x: x - w / 2, y }];
  return corners({ x: x - w / 2, y: y - h / 2, width: w, height: h });
}

function pieces(shape) {
  if (shape.kind !== "start" && shape.kind !== "end") return [{ polygon: polygon(shape) }];
  const { x, y, width: w, height: h } = shape;
  const radius = Math.min(w, h) / 2;
  const horizontal = w >= h;
  const offset = Math.abs(w - h) / 2;
  const core = { x: x - (horizontal ? offset : radius), y: y - (horizontal ? radius : offset), width: horizontal ? offset * 2 : w, height: horizontal ? h : offset * 2 };
  if (offset === 0) return [{ circle: { x, y, radius } }];
  return [{ polygon: corners(core) }, ...[-1, 1].map(sign => ({ circle: { x: x + (horizontal ? sign * offset : 0), y: y + (horizontal ? 0 : sign * offset), radius } }))];
}

function distanceToSegment(p, a, b) {
  const dx = b.x - a.x, dy = b.y - a.y;
  const t = Math.max(0, Math.min(1, ((p.x - a.x) * dx + (p.y - a.y) * dy) / (dx * dx + dy * dy || 1)));
  return Math.hypot(p.x - a.x - t * dx, p.y - a.y - t * dy);
}

function insidePolygon(p, poly, margin = 0) {
  return poly.every((a, i) => {
    const b = poly[(i + 1) % poly.length];
    const cross = (b.x - a.x) * (p.y - a.y) - (b.y - a.y) * (p.x - a.x);
    return cross >= margin * Math.hypot(b.x - a.x, b.y - a.y);
  });
}

function polygonOverlap(a, b) {
  for (const poly of [a, b]) for (const [i, p] of poly.entries()) {
    const q = poly[(i + 1) % poly.length];
    const length = Math.hypot(q.x - p.x, q.y - p.y);
    if (!length) continue;
    const nx = -(q.y - p.y) / length, ny = (q.x - p.x) / length;
    const aa = a.map(v => v.x * nx + v.y * ny), bb = b.map(v => v.x * nx + v.y * ny);
    if (Math.min(Math.max(...aa), Math.max(...bb)) - Math.max(Math.min(...aa), Math.min(...bb)) <= EPS) return false;
  }
  return true;
}

function circlePolygon(circle, poly) {
  if (insidePolygon(circle, poly)) return true;
  return poly.some((a, i) => distanceToSegment(circle, a, poly[(i + 1) % poly.length]) < circle.radius - EPS);
}

function pieceOverlap(a, b) {
  if (a.polygon && b.polygon) return polygonOverlap(a.polygon, b.polygon);
  if (a.circle && b.circle) return Math.hypot(a.circle.x - b.circle.x, a.circle.y - b.circle.y) < a.circle.radius + b.circle.radius - EPS;
  return a.circle ? circlePolygon(a.circle, b.polygon) : circlePolygon(b.circle, a.polygon);
}

export function shapesOverlap(a, b) {
  return pieces(a).some(pa => pieces(b).some(pb => pieceOverlap(pa, pb)));
}

export function boxOverlapsShape(box, shape) {
  return pieces(shape).some(piece => pieceOverlap({ polygon: corners(box) }, piece));
}

export function containsBox(shape, box) {
  const { x, y, width: w, height: h } = shape;
  return corners(box).every(p => {
    if (shape.kind === "start" || shape.kind === "end") {
      const radius = Math.min(w, h) / 2;
      const dx = Math.max(0, Math.abs(p.x - x) - (w / 2 - radius));
      const dy = Math.max(0, Math.abs(p.y - y) - (h / 2 - radius));
      return Math.hypot(dx, dy) <= radius + EPS;
    }
    return insidePolygon(p, polygon(shape), -EPS);
  });
}

function segmentPolygon(a, b, poly) {
  let low = 0, high = 1;
  for (const [i, p] of poly.entries()) {
    const q = poly[(i + 1) % poly.length];
    const nx = -(q.y - p.y), ny = q.x - p.x;
    const threshold = EPS * Math.hypot(nx, ny);
    const base = nx * (a.x - p.x) + ny * (a.y - p.y);
    const delta = nx * (b.x - a.x) + ny * (b.y - a.y);
    if (Math.abs(delta) < 1e-9) { if (base <= threshold) return false; }
    else if (delta > 0) low = Math.max(low, (threshold - base) / delta);
    else high = Math.min(high, (threshold - base) / delta);
    if (low >= high) return false;
  }
  return high > 0 && low < 1 && low < high;
}

export function segmentEntersShape(a, b, shape) {
  return pieces(shape).some(piece => piece.circle ? distanceToSegment(piece.circle, a, b) < piece.circle.radius - EPS : segmentPolygon(a, b, piece.polygon));
}

export function boxesOverlap(a, b) {
  return Math.min(a.x + a.width, b.x + b.width) - Math.max(a.x, b.x) > EPS && Math.min(a.y + a.height, b.y + b.height) - Math.max(a.y, b.y) > EPS;
}
