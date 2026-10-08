import { test, expect } from "./fixtures.js";
import { twoRounds, question } from "./rounds.js";
import { action, card } from "./screen.js";

const toast = page => page.locator("[data-toast]");
const badge = page => page.locator("[data-arrival-count]");
const tab = (page, id) => page.locator(`[data-tab="${id}"]`);

test.beforeEach(async ({ shoryo, page }) => {
  page.on("pageerror", error => console.error(error.message));
  await twoRounds(shoryo);
  await page.goto(shoryo.url);
  await expect(card(page, "q2")).toBeVisible();
  await page.clock.install();
});

async function ask(shoryo, page, text = "Explain", question = "q2") {
  await shoryo.op({ op: "ask", question, text });
  const events = await shoryo.wait();
  const id = events.filter(e => e.kind === "ask").at(-1).ask;
  await expect(page.locator(`[data-panel=current] [data-ask="${id}"]`)).toBeAttached();
  return id;
}

async function unseenReply(shoryo, page, text = "New reply") {
  const id = await ask(shoryo, page);
  await tab(page, "decisions").click();
  await shoryo.reply(id, { text });
  await expect(action(toast(page).last(), "view-arrival")).toBeVisible();
  return id;
}

test("reply_to_an_unseen_card_shows_a_toast_whose_view_lands_on_the_reply_and_back_returns", async ({ shoryo, page }) => {
  const id = await unseenReply(shoryo, page);
  await expect(badge(page)).toHaveText("1");
  await action(toast(page), "view-arrival").click();
  await expect(card(page, "q2").locator(`[data-ask="${id}"] [data-reply]`)).toBeInViewport();
  await expect(badge(page)).toHaveCount(0);
  await action(page, "back").click();
  await expect(tab(page, "decisions")).toHaveAttribute("aria-selected", "true");
});

test("reply_visible_on_arrival_shows_no_toast_and_is_not_counted", async ({ shoryo, page }) => {
  const id = await ask(shoryo, page);
  await card(page, "q2").locator(`[data-ask="${id}"]`).scrollIntoViewIfNeeded();
  await shoryo.reply(id, { text: "Visible reply" });
  await expect(card(page, "q2").getByText("Visible reply")).toBeVisible();
  await page.clock.runFor(100);
  await expect(toast(page)).toHaveCount(0);
  await expect(badge(page)).toHaveCount(0);
});

test("reply_in_another_tab_shows_a_toast", async ({ shoryo, page }) => {
  await unseenReply(shoryo, page);
  await expect(toast(page)).toHaveCount(1);
});

test("view_on_a_reply_inside_an_elided_middle_expands_and_shows_it", async ({ shoryo, page }) => {
  const ids = [];
  for (let i = 0; i < 6; i++) {
    await shoryo.op({ op: "ask", question: "q2", text: `Ask ${i}` });
    ids.push((await shoryo.wait()).filter(e => e.kind === "ask").at(-1).ask);
  }
  await tab(page, "decisions").click();
  await shoryo.reply(ids[2], { text: "Middle reply" });
  await expect(toast(page)).toHaveCount(1);
  await action(toast(page), "view-arrival").click();
  await expect(card(page, "q2").locator(`[data-ask="${ids[2]}"] [data-reply]`)).toBeInViewport();
});

test("next_round_and_result_show_their_toasts", async ({ shoryo, page }) => {
  await page.locator('[data-language="ja"]').click();
  await tab(page, "map").click();
  await shoryo.submit();
  await shoryo.round({ subject: "Next", questions: [question("q5", "Next question")] });
  await expect(toast(page)).toContainText("第 3 ラウンドが届きました");
  await shoryo.submit();
  await shoryo.round({ subject: "Result", questions: [] });
  await expect(toast(page).last()).toContainText("結果が届きました");
  await expect(action(toast(page).last(), "view-arrival")).toHaveText("見る");
});

test("toast_vanishes_after_six_seconds_but_not_while_hovered_or_focused", async ({ shoryo, page }) => {
  await unseenReply(shoryo, page);
  await toast(page).hover();
  await page.clock.runFor(6100);
  await expect(toast(page)).toHaveCount(1);
  await action(toast(page), "view-arrival").focus();
  await page.mouse.move(0, 0);
  await page.clock.runFor(6100);
  await expect(toast(page)).toHaveCount(1);
  await tab(page, "decisions").focus();
  await expect(toast(page)).toHaveCount(0);
  await unseenReply(shoryo, page, "Expires");
  await page.mouse.move(0, 0);
  await page.clock.runFor(6100);
  await expect(toast(page)).toHaveCount(0);
});

test("fourth_toast_removes_the_oldest_unheld_one", async ({ shoryo, page }) => {
  const ids = [];
  for (let i = 0; i < 4; i++) ids.push(await ask(shoryo, page, `Ask ${i}`));
  await tab(page, "decisions").click();
  for (let i = 0; i < 4; i++) {
    await shoryo.reply(ids[i], { text: `Reply ${i}` });
    await expect(badge(page)).toHaveText(String(i + 1));
  }
  await expect(toast(page)).toHaveCount(3);
  await expect(toast(page).first()).toHaveAttribute("data-toast", `reply:${ids[1]}`);
});

test("notification_badge_counts_unseen_arrivals_and_clears_when_the_list_opens", async ({ shoryo, page }) => {
  await unseenReply(shoryo, page);
  await expect(badge(page)).toHaveText("1");
  await action(page, "arrivals").click();
  await expect(badge(page)).toHaveCount(0);
  await expect(page.locator("[data-arrival-entry]")).toHaveCount(1);
});

test("unseen_count_is_a_red_white_number_at_the_bell_corner_in_both_themes", async ({ shoryo, page }) => {
  await expect(badge(page)).toHaveCount(0);
  await unseenReply(shoryo, page);
  for (const theme of ["light", "dark"]) {
    await page.emulateMedia({ colorScheme: theme });
    const colors = await badge(page).evaluate(element => {
      const style = getComputedStyle(element);
      return [style.backgroundColor, style.color].map(color => color.match(/[\d.]+/g).map(Number));
    });
    const [red, green, blue, alpha = 1] = colors[0];
    expect(red).toBeGreaterThan(green * 2);
    expect(red).toBeGreaterThan(blue * 2);
    expect(alpha).toBe(1);
    expect(colors[1].slice(0, 3).every(channel => channel >= 250)).toBe(true);
    const count = await badge(page).boundingBox();
    const bell = await action(page, "arrivals").boundingBox();
    expect(count.x + count.width / 2).toBeGreaterThan(bell.x + bell.width / 2);
    expect(count.y + count.height / 2).toBeLessThan(bell.y + bell.height / 2);
    await expect(badge(page)).toHaveText("1");
  }
  await action(page, "arrivals").click();
  await expect(badge(page)).toHaveCount(0);
});

test("arrival_menu_separates_localized_types_from_unchanged_bodies_without_narrow_window_overflow", async ({ shoryo, page }) => {
  await tab(page, "decisions").click();
  await shoryo.submit();
  await shoryo.round({ subject: "Next", questions: [question("q5", "W".repeat(60))] });
  await expect(badge(page)).toHaveText("1");
  const bodies = [await toast(page).last().locator("p").textContent()];
  const id = await ask(shoryo, page, "Explain", "q5");
  await shoryo.reply(id, { text: "New reply" });
  await expect(badge(page)).toHaveText("2");
  bodies.push(await toast(page).last().locator("p").textContent());
  await shoryo.submit();
  await shoryo.round({ subject: "Result", questions: [] });
  await expect(badge(page)).toHaveText("3");
  bodies.push(await toast(page).last().locator("p").textContent());
  await action(page, "arrivals").click();
  const menu = page.locator('[data-arrival-list]');
  const entries = menu.locator('[data-arrival-entry]');
  await expect(menu.getByRole("heading", { level: 2 })).toBeVisible();
  await expect(entries).toHaveCount(3);
  await expect(entries.locator('[data-arrival-body]')).toHaveText(bodies.reverse());
  const englishTypes = await entries.locator('[data-arrival-type]').allTextContents();
  expect(new Set(englishTypes).size).toBe(3);
  expect(englishTypes.every(text => text.trim().length > 0)).toBe(true);

  await page.setViewportSize({ width: 390, height: 700 });
  const menuColors = [];
  for (const theme of ["light", "dark"]) {
    await page.emulateMedia({ colorScheme: theme });
    const frame = await menu.boundingBox();
    expect(frame.x).toBeGreaterThanOrEqual(0);
    expect(frame.x + frame.width).toBeLessThanOrEqual(page.viewportSize().width);
    menuColors.push(await menu.evaluate(element => getComputedStyle(element).backgroundColor));
    for (const entry of await entries.all()) {
      const row = await entry.boundingBox();
      const type = await entry.locator('[data-arrival-type]').boundingBox();
      const body = await entry.locator('[data-arrival-body]').boundingBox();
      expect(type.y + type.height).toBeLessThanOrEqual(body.y);
      expect(type.y - row.y).toBeGreaterThan(type.height / 4);
      expect(body.y + body.height).toBeLessThan(row.y + row.height);
      expect(body.x + body.width).toBeLessThanOrEqual(frame.x + frame.width);
      expect(await entry.evaluate(element => element.scrollWidth <= element.clientWidth)).toBe(true);
      expect(await entry.evaluate(element => getComputedStyle(element).backgroundColor)).toBe(menuColors.at(-1));
    }
  }
  expect(menuColors[0]).not.toBe(menuColors[1]);
  await page.locator('[data-language=ja]').click();
  await expect(menu.getByRole("heading", { level: 2 })).toHaveText("通知");
  const japaneseTypes = await entries.locator('[data-arrival-type]').allTextContents();
  expect(new Set(japaneseTypes).size).toBe(3);
  expect(japaneseTypes.every((text, index) => text.length > 0 && text !== englishTypes[index])).toBe(true);
});

test("flat_arrival_rows_show_hover_and_keyboard_focus_and_keep_navigation", async ({ shoryo, page }) => {
  const id = await unseenReply(shoryo, page);
  await page.mouse.move(0, 0);
  await action(page, "arrivals").focus();
  await page.keyboard.press("Enter");
  const entry = page.locator(`[data-arrival-entry="reply:${id}"]`);
  const appearance = () => entry.evaluate(element => {
    const style = getComputedStyle(element);
    return [style.backgroundColor, style.outlineStyle, style.outlineWidth, style.boxShadow];
  });
  const resting = await appearance();
  await entry.hover();
  expect(await appearance()).not.toEqual(resting);
  await page.mouse.move(0, 0);
  await page.keyboard.press("Tab");
  await expect(entry).toBeFocused();
  expect(await appearance()).not.toEqual(resting);
  await shoryo.op({ op: "ask", question: "q2", text: "Explain another part" });
  const next = (await shoryo.wait()).filter(event => event.kind === "ask").at(-1).ask;
  await shoryo.reply(next, { text: "Another reply" });
  await expect(page.locator('[data-arrival-entry]')).toHaveCount(2);
  await expect(entry).toBeFocused();
  await page.keyboard.press("Enter");
  await expect(card(page, "q2").locator(`[data-ask="${id}"] [data-reply]`)).toBeInViewport();
  await expect(badge(page)).toHaveText("1");
  await action(page, "back").click();
  await expect(tab(page, "decisions")).toHaveAttribute("aria-selected", "true");
});

test("choosing_from_the_notification_list_moves_like_view", async ({ shoryo, page }) => {
  const id = await unseenReply(shoryo, page);
  await action(page, "arrivals").click();
  await page.locator("[data-arrival-entry]").click();
  await expect(card(page, "q2").locator(`[data-ask="${id}"] [data-reply]`)).toBeInViewport();
  await expect(badge(page)).toHaveCount(0);
  await action(page, "back").click();
  await expect(tab(page, "decisions")).toHaveAttribute("aria-selected", "true");
});

test("reply_after_sending_lands_in_its_past_round", async ({ shoryo, page }) => {
  const id = await ask(shoryo, page);
  await shoryo.submit();
  await tab(page, "decisions").click();
  await shoryo.reply(id, { text: "Late reply" });
  await expect(toast(page)).toHaveCount(1);
  await action(toast(page), "view-arrival").click();
  await expect(tab(page, "past")).toHaveAttribute("aria-selected", "true");
  await expect(page.locator(`[data-panel=past] [data-ask="${id}"] [data-reply]`)).toBeInViewport();
});

async function positionPendingReply(page, id, { submitted = true, exposed = false } = {}) {
  await page.setViewportSize({ width: 900, height: 500 });
  const panel = submitted ? "past" : "current";
  const pending = page.locator(`[data-panel=${panel}] [data-ask="${id}"] [data-mark=writing]`);
  await expect(pending).toBeAttached();
  await pending.evaluate((element, { submitted, exposed }) => {
    const edge = submitted ? document.querySelector('[data-wait-footer]').getBoundingClientRect().top : innerHeight;
    window.scrollBy(0, element.getBoundingClientRect().top - (edge + (exposed ? -16 : 2)));
  }, { submitted, exposed });
}

test("late_reply_fully_covered_by_the_wait_footer_is_announced_and_counted", async ({ shoryo, page }) => {
  const id = await ask(shoryo, page, "Explain before submission", "q4");
  await shoryo.submit();
  await tab(page, "past").click();
  await expect(page.locator('[data-wait-footer]')).toBeVisible();
  await positionPendingReply(page, id);
  await shoryo.reply(id, { text: "Reply behind the wait footer" });
  const reply = page.locator(`[data-panel=past] [data-ask="${id}"] [data-reply]`);
  await expect(reply).toBeAttached();
  const box = await reply.boundingBox();
  const footer = await page.locator('[data-wait-footer]').boundingBox();
  expect(box.y).toBeGreaterThanOrEqual(footer.y);
  expect(box.y).toBeLessThan(page.viewportSize().height);
  expect(box.y + box.height).toBeLessThanOrEqual(footer.y + footer.height);
  await page.clock.runFor(100);
  await expect(toast(page)).toHaveAttribute('data-toast', `reply:${id}`);
  await expect(badge(page)).toHaveText("1");
  await action(toast(page), "view-arrival").click();
  const revealed = await reply.boundingBox();
  const bottom = (await page.locator('[data-wait-footer]').boundingBox()).y;
  expect(revealed.y).toBeLessThan(bottom);
  expect(revealed.y + revealed.height).toBeGreaterThan((await page.getByRole('banner').boundingBox()).height);
});

for (const submitted of [true, false]) {
  test(`partly_exposed_reply_is_seen_${submitted ? "above_the_wait_footer" : "at_viewport_bottom_without_a_footer"}`, async ({ shoryo, page }) => {
    const id = await ask(shoryo, page, "Explain near the bottom", "q4");
    if (submitted) {
      await shoryo.submit();
      await tab(page, "past").click();
      await expect(page.locator('[data-wait-footer]')).toBeVisible();
    } else {
      await expect(page.locator('[data-wait-footer]')).toHaveCount(0);
    }
    await positionPendingReply(page, id, { submitted, exposed: true });
    await shoryo.reply(id, { text: "Partly exposed reply" });
    const reply = page.locator(`[data-panel=${submitted ? "past" : "current"}] [data-ask="${id}"] [data-reply]`);
    await expect(reply).toBeAttached();
    const box = await reply.boundingBox();
    const edge = submitted ? (await page.locator('[data-wait-footer]').boundingBox()).y : page.viewportSize().height;
    expect(box.y).toBeLessThan(edge);
    expect(box.y + box.height).toBeGreaterThan(edge);
    await page.clock.runFor(100);
    await expect(toast(page)).toHaveCount(0);
    await expect(badge(page)).toHaveCount(0);
  });
}

test("late_reply_partly_exposed_below_the_header_is_seen", async ({ shoryo, page }) => {
  const id = await ask(shoryo, page, "Explain near the header", "q4");
  await shoryo.submit();
  await tab(page, "past").click();
  await page.setViewportSize({ width: 900, height: 500 });
  const pending = page.locator(`[data-panel=past] [data-ask="${id}"] [data-mark=writing]`);
  await expect(pending).toBeAttached();
  await pending.evaluate(element => {
    const headerBottom = document.querySelector('header.topbar').getBoundingClientRect().bottom;
    scrollBy(0, element.getBoundingClientRect().top - (headerBottom - 16));
  });
  await shoryo.reply(id, { text: "Reply at the header edge" });
  const reply = page.locator(`[data-panel=past] [data-ask="${id}"] [data-reply]`);
  await expect(reply).toBeAttached();
  const box = await reply.boundingBox();
  const header = await page.getByRole('banner').boundingBox();
  expect(box.y).toBeGreaterThanOrEqual(0);
  expect(box.y).toBeLessThan(header.y + header.height);
  expect(box.y + box.height).toBeGreaterThan(header.y + header.height);
  await page.clock.runFor(100);
  await expect(toast(page)).toHaveCount(0);
  await expect(badge(page)).toHaveCount(0);
});

test("held_toasts_survive_overflow_until_the_oldest_is_released", async ({ shoryo, page }) => {
  const ids = [];
  for (let i = 0; i < 4; i++) ids.push(await ask(shoryo, page, `Ask ${i}`));
  await tab(page, "decisions").click();
  for (let i = 0; i < 3; i++) {
    await shoryo.reply(ids[i], { text: `Reply ${i}` });
    const item = page.locator(`[data-toast="reply:${ids[i]}"]`);
    await expect(item).toBeVisible();
    await item.dispatchEvent("pointerenter");
  }
  await shoryo.reply(ids[3], { text: "Fourth" });
  await expect(toast(page)).toHaveCount(4);
  await toast(page).first().dispatchEvent("pointerleave");
  await expect(toast(page)).toHaveCount(3);
  await expect(toast(page).first()).toHaveAttribute("data-toast", `reply:${ids[1]}`);
});

test("arrival_comparison_keeps_coalesced_replies_and_ignores_the_initial_view", async ({ page }) => {
  const result = await page.evaluate(async () => {
    const { arrivalsBetween } = await import("./view-data.js");
    const previous = { topic: { rounds: [{ number: 1, questions: [{ id: "q", text: "Question" }], asks: [
      { id: 1, question: "q", state: { status: "waiting" } },
      { id: 2, question: "q", state: { status: "waiting" } },
    ] }] } };
    const next = structuredClone(previous);
    next.topic.rounds[0].asks.forEach(ask => { ask.state = { status: "replied", text: "Reply" }; });
    return [arrivalsBetween(null, next), arrivalsBetween(previous, next), arrivalsBetween(next, next)];
  });
  expect(result[0]).toEqual([]);
  expect(result[1].map(entry => entry.ask)).toEqual([1, 2]);
  expect(result[2]).toEqual([]);
});

test("reconnecting_announces_each_round_received_while_disconnected_and_can_open_both", async ({ shoryo, page, request }) => {
  const initial = await (await request.get(`${shoryo.url}api/view`)).json();
  let release;
  const reconnect = new Promise(resolve => { release = resolve; });
  let first = true;
  // Browser offline mode can leave an existing stream open; gate reconnect with real server views instead.
  await page.route("**/api/events", async route => {
    let snapshot;
    if (first) {
      first = false;
      snapshot = initial;
    } else {
      await reconnect;
      snapshot = await (await request.get(`${shoryo.url}api/view`)).json();
    }
    await route.fulfill({ contentType: "text/event-stream", body: `retry: 50\nevent: view\ndata: ${JSON.stringify(snapshot)}\n\n` });
  });
  await page.reload();
  await expect(card(page, "q2")).toBeVisible();
  await tab(page, "decisions").click();
  await expect(toast(page)).toHaveCount(0);
  await expect(badge(page)).toHaveCount(0);
  await shoryo.submit();
  await shoryo.round({ subject: "Disconnected question round", questions: [question("q5", "Question delivered while disconnected")] });
  await shoryo.submit();
  await shoryo.round({ subject: "Disconnected result", questions: [] });
  release();
  await expect(badge(page)).toHaveText("2", { timeout: 10000 });
  await expect(toast(page)).toHaveCount(2);
  await action(page, "arrivals").click();
  const entries = page.locator("[data-arrival-entry]");
  await expect(entries).toHaveCount(2);
  await entries.last().click();
  await expect(tab(page, "past")).toHaveAttribute("aria-selected", "true");
  await expect(page.locator('[data-panel=past]').getByText("Question delivered while disconnected", { exact: true })).toBeInViewport();
  await action(page, "arrivals").click();
  await entries.first().click();
  await expect(tab(page, "current")).toHaveAttribute("aria-selected", "true");
  await expect(page.locator('[data-panel=current] [data-result]')).toBeInViewport();
});

test("reply_below_the_viewport_shows_a_toast_and_back_restores_the_scroll_anchor", async ({ shoryo, page }) => {
  await page.setViewportSize({ width: 900, height: 500 });
  const id = await ask(shoryo, page, "Explain", "q4");
  await page.evaluate(() => scrollTo(0, 0));
  const before = await card(page, "q2").boundingBox();
  await shoryo.reply(id, { text: "Below the fold" });
  await expect(toast(page)).toHaveCount(1);
  await action(toast(page), "view-arrival").click();
  await expect(card(page, "q4").locator("[data-reply]")).toBeInViewport();
  await action(page, "back").click();
  expect((await card(page, "q2").boundingBox()).y).toBeCloseTo(before.y);
});

test("view_opens_a_provisional_row_and_its_closed_thread", async ({ shoryo, page }) => {
  await shoryo.op({ op: "ask", question: "q3", text: "Explain" });
  const id = (await shoryo.wait()).filter(e => e.kind === "ask").at(-1).ask;
  await tab(page, "decisions").click();
  await shoryo.reply(id, { text: "Provisional reply" });
  await expect(toast(page)).toHaveCount(1);
  await action(toast(page), "view-arrival").click();
  await expect(page.locator(`[data-panel=current] [data-provisional-row=q3] [data-ask="${id}"] [data-reply]`)).toBeInViewport();
});

test("notification_list_keeps_the_last_ten_arrivals_only_until_reload", async ({ shoryo, page }) => {
  const ids = [];
  for (let i = 0; i < 11; i++) {
    await shoryo.op({ op: "ask", question: "q2", text: `Ask ${i}` });
    ids.push((await shoryo.wait()).filter(e => e.kind === "ask").at(-1).ask);
  }
  await tab(page, "decisions").click();
  for (let i = 0; i < 11; i++) {
    await shoryo.reply(ids[i], { text: `Reply ${i}` });
    await expect(badge(page)).toHaveText(String(i + 1));
  }
  await action(page, "arrivals").click();
  await expect(page.locator("[data-arrival-entry]")).toHaveCount(10);
  await expect(page.locator(`[data-arrival-entry="reply:${ids[0]}"]`)).toHaveCount(0);
  await page.reload();
  await action(page, "arrivals").click();
  await expect(page.locator("[data-arrival-entry]")).toHaveCount(0);
  await expect(badge(page)).toHaveCount(0);
});
