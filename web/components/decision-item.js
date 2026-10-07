// A decision as listed anywhere on the screen, with 見直したい beside it
// (docs/spec/screen.md, "見直したい").
import { h } from "../dom.js";
import { translator } from "../strings.js";

/// The review controls for one decision: ask for a review, or stop asking.
export function reviewControls(review, emit) {
  if (!review) return null;
  const t = translator(review.lang);
  return [
    review.inReview ? h("span", { class: "badge review", "data-mark": "in-review" }, t("card.in-review")) : null,
    review.canReview
      ? h(
          "button",
          {
            type: "button",
            class: "btn small",
            "data-action": "review",
            onclick: () => emit({ type: "op", op: { op: "request_review", decision: review.decision } }),
          },
          t("decision.review"),
        )
      : null,
    review.canStop
      ? h(
          "button",
          {
            type: "button",
            class: "btn quiet small",
            "data-action": "stop-review",
            onclick: () => emit({ type: "op", op: { op: "stop_review", decision: review.decision } }),
          },
          t("decision.stop-review"),
        )
      : null,
  ];
}

export function decisionItem(item, emit) {
  const t = translator(item.lang);
  return h(
    "div",
    { class: "decision-item", "data-decision-item": item.id },
    h("p", { class: "decision-name" }, item.name),
    h("p", { class: "decision-text" }, item.text),
    h(
      "div",
      { class: "decision-meta" },
      item.source
        ? [
            h("span", { class: "meta-label" }, t("decision.decided-in")),
            item.jump
              ? h(
                  "button",
                  {
                    type: "button",
                    class: "link",
                    "data-action": "go-to-source",
                    onclick: () => emit({ type: "jump", target: item.jump }),
                  },
                  item.source,
                )
              : h("span", {}, item.source),
          ]
        : null,
      item.revised ? h("span", { class: "badge revised", "data-mark": "revised" }, t("decision.revised")) : null,
      item.preApproved ? h("span", { class: "badge pre-approved", "data-mark": "pre-approved" }, t("card.pre-approved")) : null,
      h("span", { class: "decision-actions" }, reviewControls(item.review, emit)),
    ),
  );
}
