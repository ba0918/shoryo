// The thin header fixed at the top: the topic's title with the original request on demand,
// and on the right the finished picture, the theme icon and the language switch
// (docs/spec/screen.md, "ヘッダーと切替").
import { Component, h } from "../dom.js";
import { translator } from "../strings.js";

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

export class Header extends Component {
  draw(data) {
    const t = translator(data.lang);
    const next = THEMES[(THEMES.indexOf(data.theme) + 1) % THEMES.length];
    return h(
      "header",
      { class: "topbar" },
      h(
        "div",
        { class: "topbar-inner" },
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
          data.unreadable
            ? h("p", { class: "config-notice", role: "status", "data-config-unreadable": true }, t("header.config-unreadable"))
            : null,
          h(
            "button",
            {
              type: "button",
              class: "btn small",
              "data-action": "finished-picture",
              onclick: () => this.emit({ type: "show-finished-picture" }),
            },
            t("header.finished-picture"),
          ),
          h(
            "button",
            {
              type: "button",
              class: "icon-button",
              "data-action": "theme",
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

export class Tabs extends Component {
  draw(state) {
    const t = translator(state.lang);
    return h(
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
            onclick: () => this.emit({ type: "tab", tab: tab.id }),
          },
          t(`tab.${tab.id}`),
        ),
      ),
    );
  }
}
