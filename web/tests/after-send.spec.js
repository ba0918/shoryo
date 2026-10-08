// What the screen says after sending (docs/spec/screen.md, "まとめて送る", "結果").
import { test, expect } from "./fixtures.js";
import { twoRounds } from "./rounds.js";
import { action, sendAll } from "./screen.js";

const notice = (page) => page.locator("[data-panel=current] [data-sent-notice]");

test("after_send_the_notice_says_what_happens_next", async ({ shoryo, page }) => {
  await twoRounds(shoryo);
  await page.goto(shoryo.url);

  await sendAll(page);

  await expect(notice(page)).toHaveText("Sent. The LLM is reading your answers; the next round will appear here.");
  const first = await page
    .locator("[data-panel=current]")
    .locator("[data-sent-notice], [data-card]")
    .evaluateAll((elements) => elements.map((element) => (element.dataset.card ? "card" : "notice")));
  expect(first[0]).toBe("notice");
});

test("after_proceeding_with_a_result_the_notice_does_not_promise_a_next_round", async ({ shoryo, page }) => {
  await twoRounds(shoryo);
  await shoryo.submit();
  await shoryo.round({ subject: "Result", questions: [], finished_picture: "a = Screen\n| a |" });
  await page.goto(shoryo.url);

  await action(page, "send").click();
  await page.locator("[data-confirm-send] [data-action=confirm-send]").click();

  await expect(notice(page)).toHaveText(
    "You proceeded with this result. The LLM now writes the specification; nothing more is needed on this screen.",
  );
  await expect(page.locator("[data-panel=current]")).not.toContainText("next round");
});
