// Zooming and moving a drawing inside its own area: the wheel or a pinch zooms, a mouse or
// touch drag moves, and "fit" shows the whole drawing. The map and the diagram viewer share it.

/// The least zoom the person can reach by hand; "fit" goes below it when the whole drawing
/// needs that, and zooming in or out from there never jumps back up to it.
const MIN_ZOOM = 0.2;
const MAX_ZOOM = 3;
/// A floor for "fit", so an area with no size never gives a zoom of zero.
const FIT_MIN_ZOOM = 0.01;
/// How far a pointer moves before a press becomes a drag rather than a click.
const DRAG_PX = 4;

export const clamp = (value, low, high) => Math.min(high, Math.max(low, value));

export const wheelHintKey = () => /Mac/.test(navigator.userAgentData?.platform ?? navigator.platform) ? "map.wheel-mac" : "map.wheel-ctrl";

/// The view shown in `wrap`: `{ x, y, k }`, a move and a zoom. `onView(view, commit)` draws a
/// view; `commit` is "now" when a gesture or a button has ended on it, "idle" while the wheel
/// turns, and undefined during a drag or a pinch.
export class PanZoom {
  constructor(wrap, view, onView, { mode = "canvas", hint = "" } = {}) {
    this.wrap = wrap;
    this.view = { ...view };
    this.onView = onView;
    this.mode = mode;
    this.hint = hint;
    /// A drag or pinch is going on.
    this.panning = false;
    /// The press that is ending moved the drawing, so its click is not a selection.
    this.dragged = false;
    this.listen();
  }

  set(view, commit) {
    this.view = view;
    this.onView(view, commit);
  }

  zoomAt(factor, cx, cy, commit) {
    const v = this.view;
    const k = clamp(v.k * factor, Math.min(MIN_ZOOM, v.k), MAX_ZOOM);
    const ratio = k / v.k;
    this.set({ k, x: cx - (cx - v.x) * ratio, y: cy - (cy - v.y) * ratio }, commit);
  }

  zoomBy(factor) {
    const box = this.wrap.getBoundingClientRect();
    this.zoomAt(factor, box.width / 2, box.height / 2, "now");
  }

  reveal(element) {
    const area = this.wrap.getBoundingClientRect();
    const box = element.getBoundingClientRect();
    const shift = (start, end, low, high) => start < low ? low - start : end > high ? high - end : 0;
    const x = shift(box.left, box.right, area.left + 8, area.right - 8);
    const y = shift(box.top, box.bottom, area.top + 8, area.bottom - 8);
    if (x || y) this.set({ ...this.view, x: this.view.x + x, y: this.view.y + y }, "idle");
  }

  /// Fits a drawing of `size` into the area, centred, zooming in no further than `maxZoom`.
  fit(size, maxZoom) {
    this.set(fitView(this.wrap.getBoundingClientRect(), size, maxZoom), "now");
  }

  listen() {
    const wrap = this.wrap;
    const pointers = new Map();
    let gesture = null;
    const local = (event) => {
      const box = wrap.getBoundingClientRect();
      return { x: event.clientX - box.left, y: event.clientY - box.top };
    };
    wrap.addEventListener(
      "wheel",
      (event) => {
        if (!event.ctrlKey && !event.metaKey) {
          let hint = wrap.querySelector("[data-wheel-hint]");
          if (!hint) {
            hint = document.createElement("span");
            hint.className = "wheel-hint";
            hint.dataset.wheelHint = "";
            hint.textContent = this.hint;
            wrap.append(hint);
          }
          hint.hidden = false;
          clearTimeout(this.hintTimer);
          this.hintTimer = setTimeout(() => { hint.hidden = true; }, 1500);
          return;
        }
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
        if (this.mode === "page" && event.pointerType === "touch") return;
        const dx = now[0].x - gesture.start[0].x;
        const dy = now[0].y - gesture.start[0].y;
        if (!this.panning && Math.hypot(dx, dy) < DRAG_PX) return;
        if (!this.panning) wrap.setPointerCapture(event.pointerId);
        this.panning = true;
        this.dragged = true;
        this.set({ ...gesture.view, x: gesture.view.x + dx, y: gesture.view.y + dy });
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
        this.set({ k, x: to.x - (mid.x - gesture.view.x) * ratio, y: to.y - (mid.y - gesture.view.y) * ratio });
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
        this.set(this.view, "now");
        // The click that ends a drag is not a selection.
        setTimeout(() => (this.dragged = false), 0);
      }
    };
    wrap.addEventListener("pointerup", end);
    wrap.addEventListener("pointercancel", end);
  }
}

/// The view that shows a drawing of `size` whole and centred in an area of `box`'s size.
export function fitView(box, size, maxZoom) {
  const k = clamp(Math.min(box.width / size.width, box.height / size.height), FIT_MIN_ZOOM, maxZoom);
  return { k, x: (box.width - size.width * k) / 2, y: (box.height - size.height * k) / 2 };
}
