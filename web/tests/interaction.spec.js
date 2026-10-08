import { test, expect } from "./fixtures.js";
import { twoRounds } from "./rounds.js";
import { action, card } from "./screen.js";

test.beforeEach(async ({ shoryo, page }) => {
  await twoRounds(shoryo);
  await page.goto(shoryo.url);
  await expect(card(page, "q2")).toBeVisible();
});

async function keyboardTo(page, selector) {
  for (let i = 0; i < 120; i++) {
    if (await page.evaluate(selector => document.activeElement.matches(selector), selector)) return;
    await page.keyboard.press("Tab");
  }
  throw new Error(`Keyboard cannot reach ${selector}`);
}

test("keyboard_alone_answers_stamps_asks_reviews_and_sends", async ({ shoryo, page }) => {
  await keyboardTo(page, '[data-card=q2] input[type=radio]:checked');
  await page.keyboard.press("ArrowDown");
  await expect(card(page, "q2").getByRole("radio", { name: "A database", exact: true })).toBeChecked();
  await keyboardTo(page, '[data-card=q2] [data-action=stamp]');
  await page.keyboard.press("Enter");
  await expect(action(card(page, "q2"), "stamp")).toHaveAttribute("aria-pressed", "true");
  await keyboardTo(page, '[data-card=q2] [data-field=ask]');
  await page.keyboard.type("Explain this choice");
  await page.keyboard.press("Enter");
  await expect(card(page, "q2").locator("[data-mark=writing]")).toBeVisible();
  const ask = (await shoryo.wait()).find(event => event.kind === "ask").ask;
  await keyboardTo(page, '[data-tab=decisions]');
  await page.keyboard.press("Enter");
  await keyboardTo(page, '[data-panel=decisions] [data-decision-item=d1] [data-action=review]');
  await page.keyboard.press("Enter");
  await expect(page.locator("[data-review-confirmation]")).toBeVisible();
  await keyboardTo(page, '[data-review-confirmation] [data-action=confirm-review]');
  await page.keyboard.press("Enter");
  await expect(page.locator('[data-panel=decisions] [data-decision-item=d1] [data-action=stop-review]')).toBeVisible();
  await shoryo.reply(ask, { text: "Keyboard reply" });
  await expect(page.locator("[data-arrival-count]")).toHaveText("1");
  await keyboardTo(page, '[data-action=arrivals]');
  await page.keyboard.press("Enter");
  await keyboardTo(page, '[data-arrival-entry]');
  await page.keyboard.press("Enter");
  await expect(card(page, "q2").getByText("Keyboard reply")).toBeInViewport();
  await keyboardTo(page, '[data-card=q4] [data-action=stamp]');
  await page.keyboard.press("Enter");
  await expect(page.locator('[data-panel=current] [data-action=send]')).toBeEnabled();
  await keyboardTo(page, '[data-panel=current] [data-action=send]');
  await page.keyboard.press("Enter");
  await keyboardTo(page, '[data-confirm-send] [data-action=confirm-send]');
  await page.keyboard.press("Enter");
  await expect(page.locator("[data-sent-notice]")).toBeVisible();
});

test("reply_arriving_while_typing_keeps_the_focus_in_the_field", async ({ shoryo, page }) => {
  const field = card(page, "q2").locator("[data-field=ask]");
  await field.fill("First ask");
  await field.press("Enter");
  const ask = (await shoryo.wait()).find(event => event.kind === "ask").ask;
  await field.fill("An unfinished second ask");
  await shoryo.reply(ask, { text: "Reply while typing" });
  await expect(field).toBeFocused();
  await expect(field).toHaveValue("An unfinished second ask");
  await page.getByRole("heading", { level: 1 }).click();
  await expect(card(page, "q2").getByText("Reply while typing")).toBeVisible();
});

test("reduced_motion_stops_the_status_dot_the_typing_dots_and_the_toasts", async ({ shoryo, page }) => {
  await page.emulateMedia({ reducedMotion: "reduce" });
  for (const question of ["q2", "q4"]) await shoryo.op({ op: "ask", question, text: "Explain" });
  const asks = (await shoryo.wait()).filter(event => event.kind === "ask");
  await page.locator('[data-tab=decisions]').click();
  await shoryo.reply(asks[0].ask, { text: "Motionless reply" });
  await expect(page.locator("[data-toast]")).toBeVisible();
  const styles = await page.evaluate(() => [
    getComputedStyle(document.querySelector("[data-agent-status]"), "::before"),
    getComputedStyle(document.querySelector(".typing-dots span")),
    getComputedStyle(document.querySelector("[data-toast]")),
  ].map(style => ({ animation: style.animationName, transition: style.transitionDuration })));
  for (const style of styles) {
    expect(style.animation).toBe("none");
    expect(style.transition.split(",").every(value => parseFloat(value) === 0)).toBe(true);
  }
});
