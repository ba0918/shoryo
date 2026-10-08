import { Component, h } from "../dom.js";
import { translator } from "../strings.js";

export class WaitFooter extends Component {
  draw(data) {
    if (!data) return h("footer", { hidden: true });
    const t = translator(data.lang);
    return h("footer", {
      class: "wait-footer",
      role: "status",
      "data-wait-footer": true,
      "data-sent-kind": data.kind,
      "data-wait-target": data.target,
    }, h("div", { class: "wait-footer-inner" },
      h("div", { "data-wait-text": true },
        h("p", {}, t(`wait.sent.${data.kind}`), " ", t(`wait.target.${data.target}`)),
        data.round === null ? null : h("p", { "data-wait-round": true }, t("wait.round", { round: data.round })),
        data.guidance ? h("p", { "data-wait-guidance": true }, t(`wait.guidance.${data.target}`)) : null,
      ),
      h("span", { class: "wait-dots", "aria-hidden": "true" },
        [0, 1, 2].map(() => h("span", { "data-wait-dot": true })),
      ),
    ));
  }
}
