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
  await field.evaluate(input => input.setSelectionRange(3, 13, "backward"));
  await shoryo.reply(ask, { text: "Reply while typing" });
  await expect(card(page, "q2").getByText("Reply while typing")).toBeVisible();
  await expect(field).toBeFocused();
  await expect(field).toHaveValue("An unfinished second ask");
  expect(await field.evaluate(input => [input.selectionStart, input.selectionEnd, input.selectionDirection])).toEqual([3, 13, "backward"]);
  await page.getByRole("heading", { level: 1 }).click();
  await expect(field).toHaveValue("An unfinished second ask");
  await field.press("Enter");
  expect((await shoryo.wait()).filter(event => event.kind === "ask").at(-1).text).toBe("An unfinished second ask");
});

test("reply_arriving_during_input_conversion_keeps_the_editor_and_does_not_send_the_conversion", async ({ shoryo, page }) => {
  const field = card(page, "q2").locator("[data-field=ask]");
  await field.fill("First ask");
  await field.press("Enter");
  const ask = (await shoryo.wait()).find(event => event.kind === "ask").ask;
  await field.fill("変換中");
  await field.evaluate(input => {
    window.editorBlurred = false;
    input.addEventListener("blur", () => { window.editorBlurred = true; }, { once: true });
    input.dispatchEvent(new CompositionEvent("compositionstart", { bubbles: true, data: "変換中" }));
  });
  await shoryo.reply(ask, { text: "Reply during conversion" });
  await expect(card(page, "q2").getByText("Reply during conversion")).toBeVisible();
  await expect(field).toBeFocused();
  expect(await page.evaluate(() => window.editorBlurred)).toBe(false);
  await field.dispatchEvent("keydown", { key: "Enter", isComposing: true });
  await expect(field).toHaveValue("変換中");
  await field.dispatchEvent("compositionend", { data: "変換済み" });
  await field.fill("変換済み");
  await field.press("Enter");
  expect((await shoryo.wait()).filter(event => event.kind === "ask").map(event => event.text)).toEqual(["First ask", "変換済み"]);
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

test("follow_up_keeps_keyboard_focus_when_opened_and_when_another_reply_arrives", async ({ shoryo, page }) => {
  for (const text of ["First ask", "Second ask"]) await shoryo.op({ op: "ask", question: "q2", text });
  const asks = (await shoryo.wait()).filter(event => event.kind === "ask");
  await shoryo.reply(asks[0].ask, { text: "First reply" });
  const follow = card(page, "q2").getByRole("button", { name: "Follow up on this reply" });
  await follow.focus();
  await page.keyboard.press("Enter");
  await expect(card(page, "q2").locator(".following")).toBeVisible();
  await expect(follow).toBeFocused();
  await shoryo.reply(asks[1].ask, { text: "Second reply" });
  await expect(card(page, "q2").getByText("Second reply", { exact: true })).toBeVisible();
  await expect(follow.first()).toBeFocused();
});

test("retained_map_details_controls_keep_focus_when_an_answer_changes", async ({ shoryo, page }) => {
  await page.locator('[data-tab=map]').click();
  await page.locator('[data-map-node="d:d1"]').click();
  const details = page.locator("[data-map-selection]");
  let option = 0;
  for (const name of ["Close", "Show the path to this", "Go to its question", "Review this"]) {
    const control = details.getByRole("button", { name, exact: true });
    await control.focus();
    option = 1 - option;
    await shoryo.op({ op: "choose", question: "q2", option });
    await expect(card(page, "q2").getByRole("radio", { name: option ? "A database" : "One JSON file", includeHidden: true })).toBeChecked();
    await page.evaluate(() => new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve))));
    await expect(control).toBeFocused();
  }
  await shoryo.op({ op: "request_review", decision: "d1" });
  const stop = details.getByRole("button", { name: "Stop review", exact: true });
  await stop.focus();
  await shoryo.op({ op: "choose", question: "q2", option: 1 });
  await expect(card(page, "q2").getByRole("radio", { name: "A database", exact: true, includeHidden: true })).toBeChecked();
  await page.evaluate(() => new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve))));
  await expect(stop).toBeFocused();
});

test("provisional_row_keeps_keyboard_focus_on_its_toggle_when_opened_and_folded", async ({ page }) => {
  const selector = '[data-panel=current] [data-provisional-row=q3] [data-action=open]';
  const toggle = page.locator(selector);
  await keyboardTo(page, selector);
  await page.keyboard.press("Enter");
  await expect(toggle).toHaveAttribute("aria-expanded", "true");
  await expect(toggle).toBeFocused();
  await page.keyboard.press("Space");
  await expect(toggle).toHaveAttribute("aria-expanded", "false");
  await expect(toggle).toBeFocused();
});

test("reply_during_note_editing_does_not_overwrite_the_completed_note_with_older_text", async ({ shoryo, page, request }) => {
  await shoryo.op({ op: "ask", question: "q2", text: "Explain this answer" });
  const ask = (await shoryo.wait()).find(event => event.kind === "ask").ask;
  await expect(card(page, "q2").locator("[data-mark=writing]")).toBeVisible();
  await page.clock.install({ time: new Date("2026-10-08T12:00:00Z") });
  await page.clock.pauseAt(new Date("2026-10-08T12:00:01Z"));
  const pending = new Set();
  page.on("request", incoming => {
    if (incoming.url().endsWith("/api/op")) pending.add(incoming);
  });
  const finished = incoming => pending.delete(incoming);
  page.on("requestfinished", finished);
  page.on("requestfailed", finished);
  const note = card(page, "q2").locator('[data-field=note] textarea');
  await note.fill("A");
  await page.clock.runFor(100);
  await shoryo.reply(ask, { text: "Reply during note editing" });
  await expect(card(page, "q2").getByText("Reply during note editing")).toBeVisible();
  await expect(note).toBeFocused();
  await expect(note).toHaveValue("A");
  await note.fill("AB");
  await page.clock.runFor(100);
  await page.getByRole("heading", { level: 1 }).click();
  const storedNote = async () => {
    const view = await (await request.get(`${shoryo.url}api/view`)).json();
    return view.topic.rounds.at(-1).questions.find(question => question.id === "q2").answer.note;
  };
  await expect.poll(storedNote).toBe("AB");
  await page.clock.runFor(1000);
  await expect.poll(() => pending.size).toBe(0);
  await expect.poll(storedNote).toBe("AB");
  await page.reload();
  await expect(note).toHaveValue("AB");
});
