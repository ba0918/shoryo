// Locators for the screen's parts, by their data-* hooks, and the steps tests repeat.
import { expect } from "@playwright/test";

export const card = (scope, id) => scope.locator(`[data-card="${id}"]`);
export const row = (scope, id) => scope.locator(`[data-provisional-row="${id}"]`);
export const action = (scope, name) => scope.locator(`[data-action="${name}"]`);
export const mark = (scope, name) => scope.locator(`[data-mark="${name}"]`);

/// Stamps every question still without a stamp, then presses "Send all".
export async function sendAll(page) {
  const unstamped = page.locator('[data-panel=current] [data-action=stamp][aria-pressed="false"]');
  while ((await unstamped.count()) > 0) {
    const before = await unstamped.count();
    await unstamped.first().click();
    await expect(unstamped).toHaveCount(before - 1);
  }
  await page.locator("[data-panel=current] [data-action=send]").click();
}
