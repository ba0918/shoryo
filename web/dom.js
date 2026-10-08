// The two pieces every component is built from: `h`, which makes elements and only ever
// puts strings in as text (so text the LLM wrote can never become markup), and
// `Component`, which redraws itself only when its view data changes.

const listeners = new WeakMap();

export function h(tag, attrs = {}, ...children) {
  const element = document.createElement(tag);
  const events = [];
  for (const [name, value] of Object.entries(attrs)) {
    if (value === undefined || value === null || value === false) continue;
    if (name.startsWith("on") && typeof value === "function") {
      element.addEventListener(name.slice(2), value);
      events.push([name.slice(2), value]);
    } else if (name === "class") {
      element.className = value;
    } else if (["checked", "disabled", "selected", "value", "open", "hidden"].includes(name)) {
      element[name] = value;
    } else {
      element.setAttribute(name, value === true ? "" : String(value));
    }
  }
  append(element, children);
  listeners.set(element, events);
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

const textField = (element) =>
  (element instanceof HTMLTextAreaElement ||
    (element instanceof HTMLInputElement && element.type === "text"));

export function focusSelector(element, root = document) {
  if (!root.contains?.(element)) return null;
  const attribute = element.hasAttribute("data-focus") ? "data-focus" : element.hasAttribute("data-action") ? "data-action" : null;
  if (!attribute) return null;
  // An item/action can appear in several regions; an unqualified key would focus another copy.
  const scopes = [];
  for (let parent = element.parentElement; parent && parent !== root; parent = parent.parentElement) {
    if (parent.hasAttribute("data-focus-scope")) scopes.unshift(`[data-focus-scope="${CSS.escape(parent.dataset.focusScope)}"]`);
  }
  return [...scopes, `[${attribute}="${CSS.escape(element.getAttribute(attribute))}"]`].join(" ");
}

function retainEditor(current, next, editor, replacement) {
  for (const attr of [...current.attributes]) if (!next.hasAttribute(attr.name)) current.removeAttribute(attr.name);
  for (const attr of next.attributes) current.setAttribute(attr.name, attr.value);
  for (const [name, handler] of listeners.get(current) ?? []) current.removeEventListener(name, handler);
  const events = listeners.get(next) ?? [];
  for (const [name, handler] of events) current.addEventListener(name, handler);
  listeners.set(current, events);
  if (current === editor) return;

  const branch = [...current.childNodes].find(child => child === editor || child.contains?.(editor));
  let cursor = current.firstChild;
  for (const child of [...next.childNodes]) {
    if (child === replacement || child.contains?.(replacement)) {
      while (cursor !== branch) {
        const removed = cursor;
        cursor = cursor.nextSibling;
        removed.remove();
      }
      retainEditor(branch, child, editor, replacement);
      cursor = branch.nextSibling;
    } else {
      current.insertBefore(child, cursor);
    }
  }
  while (cursor) {
    const removed = cursor;
    cursor = cursor.nextSibling;
    removed.remove();
  }
}

/// A region of the screen. Subclasses implement `draw(data)`, returning one element, and may
/// keep their own local state in fields. `emit` reports the person's actions upwards.
export class Component {
  constructor(emit) {
    this.emit = emit;
    this.el = document.createComment("component");
    this.key = undefined;
    this.data = undefined;
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
    const focused = focusSelector(active, this.el);
    const drafts = [...this.el.querySelectorAll?.('textarea[data-field="ask"]') ?? []];
    const next = this.draw(this.data);
    for (const field of drafts) {
      const replacement = next.querySelector(focusSelector(field, this.el));
      if (replacement) replacement.value = field.value;
    }
    const replacement = focused ? next.querySelector(focused) : null;
    if (replacement && textField(active)) {
      // Replacing and refocusing the editor interrupts the browser's input-method conversion.
      retainEditor(this.el, next, active, replacement);
      return;
    }
    this.el.replaceWith(next);
    this.el = next;
    if (focused) next.querySelector(focused)?.focus({ preventScroll: true });
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
