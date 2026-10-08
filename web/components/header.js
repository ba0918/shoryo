// The thin header fixed at the top: "Back" while there is somewhere to go back to, the
// topic's title with the original request on demand, and on the right, in this order, what the
// LLM is doing, the notice that the settings file is unreadable, the theme icon and the
// language switch (docs/spec/screen.md, "ヘッダーと切替", "移動と現在地"). The tabs and the
// finished picture sit in the row below.
import { Component, h } from "../dom.js";
import { translator } from "../strings.js";
import { Arrivals } from "./toasts.js";

const SVG = "http://www.w3.org/2000/svg";

/// The order the theme icon cycles through.
export const THEMES = ["light", "dark", "system"];

const ICONS = {
  light: [
    ["circle", { cx: 12, cy: 12, r: 4 }],
    ["path", { d: "M12 2v2M12 20v2M2 12h2M20 12h2M4.9 4.9l1.4 1.4M17.7 17.7l1.4 1.4M4.9 19.1l1.4-1.4M17.7 6.3l1.4-1.4" }],
  ],
  dark: [["path", { d: "M20 14.5A8 8 0 0 1 9.5 4a8 8 0 1 0 10.5 10.5z" }]],
  system: [
    ["rect", { x: 3, y: 4, width: 18, height: 12, rx: 2 }],
    ["path", { d: "M8 20h8M12 16v4" }],
  ],
};

function icon(theme) {
  const svg = document.createElementNS(SVG, "svg");
  for (const [name, value] of Object.entries({ viewBox: "0 0 24 24", width: 18, height: 18, "aria-hidden": "true" })) {
    svg.setAttribute(name, String(value));
  }
  for (const [tag, attrs] of ICONS[theme]) {
    const part = document.createElementNS(SVG, tag);
    for (const [name, value] of Object.entries(attrs)) part.setAttribute(name, String(value));
    svg.append(part);
  }
  return svg;
}

/// What the LLM is doing. It changes on its own, with every `wait`, so it redraws apart from
/// the rest of the header: an opened original request stays open and the focus stays put.
/// The live region is one element that stays put while only its content changes, because a
/// screen reader may miss text that arrives together with a new live region. With no status
/// it is left empty and takes no room.
class AgentStatus extends Component {
  constructor(emit) {
    super(emit);
    this.el = h("p", { role: "status" });
  }

  redraw() {
    const { agent, lang } = this.data;
    const t = translator(lang);
    this.el.className = agent ? `agent-status ${agent}` : "visually-hidden";
    this.el.hidden = !agent;
    this.el.textContent = agent ? t(`agent.${agent}`) : "";
    for (const [name, value] of [["data-agent-status", agent], ["title", agent && t(`agent.${agent}-title`)]]) {
      if (value) this.el.setAttribute(name, value);
      else this.el.removeAttribute(name);
    }
  }
}

export class Header extends Component {
  constructor(emit) {
    super(emit);
    this.status = new AgentStatus(emit);
    this.arrivals = new Arrivals(emit);
  }

  update({ agent, arrivals, ...rest }) {
    this.status.update({ agent, lang: rest.lang });
    this.arrivals.update({ ...arrivals, lang: rest.lang });
    return super.update(rest);
  }

  draw(data) {
    const t = translator(data.lang);
    const next = THEMES[(THEMES.indexOf(data.theme) + 1) % THEMES.length];
    return h(
      "header",
      { class: "topbar" },
      h(
        "div",
        { class: "topbar-inner" },
        data.canGoBack
          ? h(
              "button",
              { type: "button", class: "btn quiet small back", "data-action": "back", onclick: () => this.emit({ type: "back" }) },
              h("span", { "aria-hidden": "true" }, "←"),
              t("screen.back"),
            )
          : null,
        h(
          "div",
          { class: "topbar-title" },
          h("h1", {}, data.title || t("header.untitled")),
          data.original_request
            ? h(
                "details",
                { class: "original-request" },
                h("summary", {}, t("header.original-request")),
                h("p", {}, data.original_request),
              )
            : null,
        ),
        h(
          "div",
          { class: "topbar-tools" },
          this.arrivals.el,
          this.status.el,
          data.unreadable
            ? h("p", { class: "config-notice", role: "status", "data-config-unreadable": true }, t("header.config-unreadable"))
            : null,
          h(
            "button",
            {
              type: "button",
              class: "icon-button",
              "data-action": "theme",
              "data-focus": "theme",
              "data-theme-value": data.theme,
              "aria-label": t(`header.theme.${data.theme}`),
              title: t(`header.theme.${data.theme}`),
              onclick: () => this.emit({ type: "config", change: { theme: next } }),
            },
            icon(data.theme),
          ),
          h(
            "div",
            { class: "seg-group language", role: "group", "aria-label": t("header.language") },
            ["en", "ja"].map((lang) =>
              h(
                "button",
                {
                  type: "button",
                  class: `seg${lang === data.lang ? " on" : ""}`,
                  lang,
                  "data-language": lang,
                  "data-focus": `language-${lang}`,
                  "aria-pressed": String(lang === data.lang),
                  onclick: () => this.emit({ type: "config", change: { language: lang } }),
                },
                lang,
              ),
            ),
          ),
        ),
      ),
    );
  }
}

/// The tab row: the four tabs, and at its right end the finished picture, which every tab
/// can open.
export class Tabs extends Component {
  draw(state) {
    const t = translator(state.lang);
    return h(
      "div",
      { class: "tab-row", "data-tab-row": true },
      h(
        "nav",
        { class: "tabs", role: "tablist" },
        state.tabs.map((tab) =>
          h(
            "button",
            {
              type: "button",
              role: "tab",
              "aria-selected": String(tab.id === state.current),
              "data-tab": tab.id,
              "data-focus": `tab-${tab.id}`,
              onclick: () => this.emit({ type: "tab", tab: tab.id }),
            },
            t(`tab.${tab.id}`),
          ),
        ),
      ),
      h(
        "button",
        {
          type: "button",
          class: "btn small finished-picture-button",
          "data-action": "finished-picture",
          "data-focus": "finished-picture",
          onclick: () => this.emit({ type: "show-finished-picture" }),
        },
        t("header.finished-picture"),
      ),
    );
  }
}
