import { Component, KeyedList, h } from "../dom.js";
import { translator } from "../strings.js";

function bell() {
  const ns = "http://www.w3.org/2000/svg";
  const svg = document.createElementNS(ns, "svg");
  for (const [name, value] of Object.entries({ viewBox: "0 0 24 24", width: 18, height: 18, "aria-hidden": "true", fill: "none", stroke: "currentColor", "stroke-width": 1.5 })) svg.setAttribute(name, value);
  const path = document.createElementNS(ns, "path");
  path.setAttribute("d", "M5 17h14l-2-3V9a5 5 0 0 0-10 0v5zM10 20h4");
  svg.append(path);
  return svg;
}

class Toast extends Component {
  draw(data) {
    const hold = (kind, held) => this.emit({ type: "hold-toast", id: data.id, kind, held });
    return h("div", {
      class: "toast", "data-toast": data.id,
      onpointerenter: () => hold("pointer", true),
      onpointerleave: () => hold("pointer", false),
      onfocusin: () => hold("focus", true),
      onfocusout: event => { if (!event.currentTarget.contains(event.relatedTarget)) hold("focus", false); },
    },
    h("p", {}, data.text),
    h("button", {
      type: "button", class: "btn small", "data-action": "view-arrival", "data-focus": `arrival-${data.id}`,
      onclick: () => this.emit({ type: "view-arrival", id: data.id }),
    }, translator(data.lang)("arrival.view")));
  }
}

export class Toasts {
  constructor(emit) {
    this.el = h("div", { class: "toasts", "data-toasts": true, "aria-live": "polite" });
    this.list = new KeyedList(this.el, () => new Toast(emit));
  }

  update(entries) {
    this.list.update(entries.map(data => ({ key: data.id, data })));
  }
}

export class Arrivals extends Component {
  draw(data) {
    const t = translator(data.lang);
    return h("div", { class: "arrivals" },
      h("button", {
        type: "button", class: "icon-button", "data-action": "arrivals", "data-focus": "arrivals",
        "aria-label": t("arrival.label"), "aria-expanded": String(data.open),
        onclick: () => this.emit({ type: "toggle-arrivals" }),
      }, bell(),
      data.count ? h("span", { class: "arrival-count", "data-arrival-count": true }, data.count) : null),
      data.open ? h("div", { class: "arrival-list", "data-arrival-list": true },
        data.entries.length ? data.entries.map(entry => h("button", {
          type: "button", "data-arrival-entry": entry.id, "data-focus": `entry-${entry.id}`,
          onclick: () => this.emit({ type: "view-arrival", id: entry.id }),
        }, entry.text)) : h("p", {}, t("arrival.empty"))) : null);
  }
}
