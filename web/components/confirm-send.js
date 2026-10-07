// The confirmation "Send all" opens: what is about to be sent, question by question, with
// each stamp, and the review requests going with it (docs/spec/screen.md, "まとめて送る").
import { h } from "../dom.js";
import { translator } from "../strings.js";

export function confirmSend(data, emit) {
  const t = translator(data.lang);
  const title = data.result ? t("confirm.result-title") : t("confirm.title");
  return {
    label: title,
    attrs: { "data-confirm-send": true },
    body: [
      h("h2", { class: "dialog-title" }, title),
      data.questions.length > 0
        ? h(
            "ol",
            { class: "confirm-list" },
            data.questions.map((question) =>
              h(
                "li",
                { class: "confirm-item", "data-confirm-question": question.id },
                h("p", { class: "confirm-question" }, question.text),
                h(
                  "p",
                  { class: "confirm-answer" },
                  question.deferred
                    ? h("span", { class: "badge deferred", "data-mark": "deferred" }, t("confirm.deferred"))
                    : [h("span", { class: "meta-label" }, t("confirm.answer")), h("span", {}, question.answer)],
                  question.stamp === "pre_approved"
                    ? h("span", { class: "badge pre-approved", "data-mark": "pre-approved" }, t("card.pre-approved"))
                    : h("span", { class: "badge stamped", "data-mark": "approved" }, t("confirm.stamp.person")),
                ),
                question.note
                  ? h("p", { class: "confirm-note" }, h("span", { class: "meta-label" }, t("confirm.note")), h("span", {}, question.note))
                  : null,
              ),
            ),
          )
        : h("p", { class: "empty" }, data.reviews.length > 0 ? t("confirm.no-questions") : t("confirm.result-proceed")),
      data.reviews.length > 0
        ? h(
            "div",
            { class: "confirm-reviews" },
            h("h3", {}, t("confirm.review-requests")),
            h("ul", {}, data.reviews.map((review) => h("li", { "data-confirm-review": review.id }, review.name))),
          )
        : null,
      h(
        "div",
        { class: "dialog-actions" },
        h(
          "button",
          { type: "button", class: "btn", "data-action": "cancel-send", "data-focus": "cancel-send", onclick: () => emit({ type: "close-dialog" }) },
          t("confirm.back"),
        ),
        h(
          "button",
          {
            type: "button",
            class: "btn primary",
            "data-action": "confirm-send",
            "data-focus": "confirm-send",
            onclick: () => emit({ type: "send", round: data.round }),
          },
          t("confirm.send"),
        ),
      ),
    ],
  };
}
