// The topic's header: its title, the original request on demand, and the finished picture.
import { Component, h } from "../dom.js";

export class Header extends Component {
  draw(topic) {
    return h(
      "header",
      { class: "topic-header" },
      h("h1", {}, topic.title || "Untitled topic"),
      topic.original_request
        ? h(
            "details",
            { class: "original-request" },
            h("summary", {}, "The original request"),
            h("p", {}, topic.original_request),
          )
        : null,
      h(
        "button",
        {
          type: "button",
          "data-action": "finished-picture",
          onclick: () => this.emit({ type: "show-finished-picture" }),
        },
        "Finished picture",
      ),
    );
  }
}

export class Tabs extends Component {
  draw(state) {
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
          tab.name,
        ),
      ),
    );
  }
}
