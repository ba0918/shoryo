// A decision as listed anywhere on the screen, with 見直す beside it
// (docs/spec/screen.md, "見直す").
import { h } from "../dom.js";
import { translator } from "../strings.js";

/// The review controls for one decision: ask for a review, or stop asking.
export function reviewControls(review, emit) {
  if (!review) return null;
  const t = translator(review.lang);
  return [
    review.inReview ? h("span", { class: "badge review", "data-mark": "in-review" }, t("card.in-review")) : null,
    review.showReview
      ? h(
          "button",
          {
            type: "button",
            class: "btn small",
            "data-action": "review",
            disabled: !review.canReview,
            onclick: () =>
              emit({ type: "confirm-review", decision: review.decision, resultWording: review.resultWording }),
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
    item.source
      ? h(
          "p",
          { class: "decision-source" },
          h("span", { class: "decision-source-arrow", "aria-hidden": "true" }, "←"),
          item.jump
            ? h(
                "button",
                {
                  type: "button",
                  class: "link",
                  "data-action": "go-to-source",
                  title: t("decision.go-to-source"),
                  onclick: () => emit({ type: "jump", target: item.jump }),
                },
                item.source,
              )
            : h("span", {}, item.source),
        )
      : null,
    h(
      "div",
      { class: "decision-meta" },
      item.revised ? h("span", { class: "badge revised", "data-mark": "revised" }, t("decision.revised")) : null,
      item.preApproved ? h("span", { class: "badge pre-approved", "data-mark": "pre-approved" }, t("card.pre-approved")) : null,
      h("span", { class: "decision-actions" }, reviewControls(item.review, emit)),
    ),
  );
}
