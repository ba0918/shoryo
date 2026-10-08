// Locators for the screen's parts, by their data-* hooks, and the steps tests repeat.
import { expect } from "@playwright/test";

export const card = (scope, id) => scope.locator(`[data-card="${id}"]`);
export const row = (scope, id) => scope.locator(`[data-provisional-row="${id}"]`);
export const action = (scope, name) => scope.locator(`[data-action="${name}"]`);
export const mark = (scope, name) => scope.locator(`[data-mark="${name}"]`);

/// Stamps every question still without a stamp, presses "Send all" and confirms.
export async function sendAll(page) {
  const unstamped = page.locator('[data-panel=current] [data-action=stamp][aria-pressed="false"]');
  while ((await unstamped.count()) > 0) {
    const before = await unstamped.count();
    await unstamped.first().click();
    await expect(unstamped).toHaveCount(before - 1);
  }
  await page.locator("[data-panel=current] [data-action=send]").click();
  await page.locator("[data-confirm-send] [data-action=confirm-send]").click();
}

/// A stored UTC time as the browser shows it in `timeZone`: its `M/D`, and a pattern for the
/// tooltip — the four-digit year, then the hour and the two-digit minute, in that order.
export function shownIn(iso, timeZone) {
  const format = new Intl.DateTimeFormat("en-US", {
    timeZone,
    year: "numeric",
    month: "numeric",
    day: "numeric",
    hour: "numeric",
    minute: "2-digit",
    hourCycle: "h23",
  });
  const part = Object.fromEntries(format.formatToParts(new Date(iso)).map(({ type, value }) => [type, value]));
  return {
    date: `${part.month}/${part.day}`,
    time: new RegExp(`${part.year}[\\s\\S]*\\b0?${Number(part.hour)}\\D+${part.minute}\\b`),
  };
}

/// The topic as the server stores it now.
export async function storedTopic(shoryo) {
  return (await (await fetch(`${shoryo.url}api/view`)).json()).topic;
}

export const stampDate = (stamp) => stamp.locator("[data-stamp-date]");
export const stampTime = (stamp) => stamp.locator("xpath=..").locator("[data-stamp-time]");

export const reviewConfirmation = (page) => page.locator("[data-review-confirmation]");

/// Presses 見直す in `scope` and confirms it in the dialog that opens.
export async function reviewAndConfirm(page, scope) {
  await action(scope, "review").click();
  await action(reviewConfirmation(page), "confirm-review").click();
}
