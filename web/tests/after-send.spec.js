// What the screen says after sending, and what the LLM is doing
// (docs/spec/screen.md, "まとめて送る", "結果"; docs/spec/server.md, "待つ").
import { test, expect } from "./fixtures.js";
import { twoRounds } from "./rounds.js";
import { action, card, sendAll } from "./screen.js";

const banner = (page) => page.getByRole("banner");
const notice = (page) => page.locator("[data-panel=current] [data-sent-notice]");
const agentStatus = (page) => banner(page).locator("[data-agent-status]");

test("after_send_the_notice_says_what_happens_next", async ({ shoryo, page }) => {
  await twoRounds(shoryo);
  await page.goto(shoryo.url);

  await sendAll(page);

  await expect(notice(page)).toBeVisible();
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

  await expect(notice(page)).toBeVisible();
  await expect(page.locator("[data-panel=current]")).not.toContainText("next round");
});

test("header_shows_waiting_while_the_agent_waits_and_working_after_an_event_is_delivered", async ({ shoryo, page }) => {
  await twoRounds(shoryo);
  await page.goto(shoryo.url);
  await expect(agentStatus(page)).toHaveAttribute("data-agent-status", "working");
  const earlier = await shoryo.wait();

  const waiting = shoryo.waitAfter(
    earlier.map((event) => event.id),
    20,
  );
  await expect(agentStatus(page)).toHaveAttribute("data-agent-status", "waiting");
  await expect(agentStatus(page)).toHaveText("LLM waiting");
  await card(page, "q2").locator("[data-field=ask]").fill("Why a file?");
  await action(card(page, "q2"), "ask").click();

  expect((await waiting).map((event) => event.kind)).toEqual(["ask"]);
  await expect(agentStatus(page)).toHaveAttribute("data-agent-status", "working");
  await expect(agentStatus(page)).toHaveText("LLM working");
});

test("header_shows_not_responding_after_ten_minutes_without_a_wait_or_reply", async ({ shoryo, page }) => {
  await page.clock.install();
  await twoRounds(shoryo);
  await page.goto(shoryo.url);
  await expect(agentStatus(page)).toHaveAttribute("data-agent-status", "working");

  await page.clock.fastForward("09:30");
  await expect(agentStatus(page)).toHaveAttribute("data-agent-status", "working");
  await page.clock.fastForward("01:00");

  await expect(agentStatus(page)).toHaveAttribute("data-agent-status", "not-responding");
  await expect(agentStatus(page)).toHaveText("LLM not responding");
});

test("a_change_of_the_llm_status_leaves_the_opened_request_open_and_the_focus_in_place", async ({ shoryo, page }) => {
  await twoRounds(shoryo);
  await page.goto(shoryo.url);
  await expect(agentStatus(page)).toHaveAttribute("data-agent-status", "working");
  const earlier = await shoryo.wait();
  const request = banner(page).locator("details");
  await request.locator("summary").click();
  const picture = action(banner(page), "finished-picture");
  await picture.focus();

  const waiting = shoryo.waitAfter(
    earlier.map((event) => event.id),
    1,
  );
  await expect(agentStatus(page)).toHaveAttribute("data-agent-status", "waiting");

  await expect(request).toHaveJSProperty("open", true);
  await expect(picture).toBeFocused();
  await waiting;
});

test("after_proceeding_with_a_result_the_header_keeps_working_past_ten_minutes", async ({ shoryo, page }) => {
  await page.clock.install();
  await twoRounds(shoryo);
  await shoryo.submit();
  await shoryo.round({ subject: "Result", questions: [], finished_picture: "a = Screen\n| a |" });
  await page.goto(shoryo.url);
  await action(page, "send").click();
  await page.locator("[data-confirm-send] [data-action=confirm-send]").click();
  await expect(notice(page)).toBeVisible();

  await page.clock.fastForward("10:30");

  await expect(agentStatus(page)).toHaveAttribute("data-agent-status", "working");
});
