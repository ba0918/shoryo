// The two pieces every component is built from: `h`, which makes elements and only ever
// puts strings in as text (so text the LLM wrote can never become markup), and
// `Component`, which redraws itself only when its view data changes.

export function h(tag, attrs = {}, ...children) {
  const element = document.createElement(tag);
  for (const [name, value] of Object.entries(attrs)) {
    if (value === undefined || value === null || value === false) continue;
    if (name.startsWith("on") && typeof value === "function") {
      element.addEventListener(name.slice(2), value);
    } else if (name === "class") {
      element.className = value;
    } else if (["checked", "disabled", "selected", "value", "open", "hidden"].includes(name)) {
      element[name] = value;
    } else {
      element.setAttribute(name, value === true ? "" : String(value));
    }
  }
  append(element, children);
  return element;
}

function append(element, children) {
  for (const child of children) {
    if (child === undefined || child === null || child === false) continue;
    if (Array.isArray(child)) append(element, child);
    else if (child instanceof Node) element.appendChild(child);
    else element.appendChild(document.createTextNode(String(child)));
  }
}

const typing = (element) =>
  element instanceof HTMLTextAreaElement ||
  (element instanceof HTMLInputElement && element.type === "text");

/// A region of the screen. Subclasses implement `draw(data)`, returning one element, and may
/// keep their own local state in fields. `emit` reports the person's actions upwards.
export class Component {
  constructor(emit) {
    this.emit = emit;
    this.el = document.createComment("component");
    this.key = undefined;
    this.data = undefined;
    this.pending = false;
  }

  update(data) {
    const key = JSON.stringify(data);
    if (key === this.key) return this;
    this.key = key;
    this.data = data;
    this.redraw();
    return this;
  }

  redraw() {
    const active = document.activeElement;
    if (this.el.contains?.(active) && typing(active)) {
      // Redrawing would take the text field away while the person types; wait for blur.
      if (!this.pending) {
        this.pending = true;
        active.addEventListener("blur", () => {
          this.pending = false;
          this.redraw();
        }, { once: true });
      }
      return;
    }
    const focused = this.el.contains?.(active) ? active.dataset.focus : undefined;
    const next = this.draw(this.data);
    this.el.replaceWith(next);
    this.el = next;
    if (focused) next.querySelector(`[data-focus="${CSS.escape(focused)}"]`)?.focus();
  }
}

/// Keeps one component per key inside `container`, in the order given.
export class KeyedList {
  constructor(container, make) {
    this.container = container;
    this.make = make;
    this.items = new Map();
  }

  update(entries) {
    const keys = new Set(entries.map((entry) => entry.key));
    for (const [key, component] of this.items) {
      if (!keys.has(key)) {
        component.el.remove();
        this.items.delete(key);
      }
    }
    entries.forEach(({ key, data }, index) => {
      let component = this.items.get(key);
      if (!component) {
        component = this.make(key);
        this.items.set(key, component);
        this.container.appendChild(component.el);
      }
      component.update(data);
      // Moving a node that is already in place would take the focus away from it.
      const here = this.container.childNodes[index];
      if (here !== component.el) this.container.insertBefore(component.el, here ?? null);
    });
  }
}
