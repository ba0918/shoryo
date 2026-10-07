// A decision as listed anywhere on the screen, with 見直したい beside it
// (docs/spec/screen.md, "見直したい").
import { h } from "../dom.js";

/// The review controls for one decision: ask for a review, or stop asking.
export function reviewControls(review, emit) {
  if (!review) return null;
  return [
    review.inReview ? h("span", { class: "mark review", "data-mark": "in-review" }, "In review") : null,
    review.canReview
      ? h(
          "button",
          {
            type: "button",
            "data-action": "review",
            onclick: () => emit({ type: "op", op: { op: "request_review", decision: review.decision } }),
          },
          "Review this",
        )
      : null,
    review.canStop
      ? h(
          "button",
          {
            type: "button",
            "data-action": "stop-review",
            onclick: () => emit({ type: "op", op: { op: "stop_review", decision: review.decision } }),
          },
          "Stop review",
        )
      : null,
  ];
}

export function decisionItem(item, emit) {
  return h(
    "div",
    { class: "decision-item", "data-decision-item": item.id },
    h("p", { class: "decision-line" }, h("strong", {}, item.name), ": ", item.text),
    h(
      "p",
      { class: "decision-meta" },
      item.source
        ? [
            h("span", { class: "label" }, "Decided in: "),
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
              : item.source,
            " ",
          ]
        : null,
      item.revised ? h("span", { class: "mark revised", "data-mark": "revised" }, "Revised later") : null,
      item.preApproved ? h("span", { class: "mark pre-approved", "data-mark": "pre-approved" }, "Pre-approved") : null,
      reviewControls(item.review, emit),
    ),
  );
}
