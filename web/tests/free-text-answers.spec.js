import { test, expect } from "./fixtures.js";
import { question } from "./rounds.js";

async function setup(shoryo, page) {
  await shoryo.round({ subject: "Own answers", questions: [question("q1", "Your answer?"), question("q2", "Small choice?", { cls: "provisional" })] });
  await page.goto(shoryo.url);
}
const q = (page, id = "q1") => page.locator(`[data-card="${id}"], [data-provisional-row="${id}"]`);
const custom = scope => scope.locator('[data-field="free-text"] textarea');
const other = scope => scope.getByRole("radio", { name: "Other (write my answer)", exact: true });

test("Other is available folded and restores both drafts after radio switches", async ({ shoryo, page }) => {
  await setup(shoryo, page);
  const scope = q(page);
  await scope.locator('[data-field="note"] textarea').fill("choice note");
  await other(scope).check();
  await custom(scope).fill("  Own answer\nwith conditions  ");
  await expect(scope.locator("[data-consequence]")).toHaveCount(0);
  await expect(scope.locator('[data-field="note"]')).toHaveCount(0);
  await scope.getByRole("radio").first().check();
  await expect(scope.locator('[data-field="note"] textarea')).toHaveValue("choice note");
  await other(scope).check();
  await expect(custom(scope)).toHaveValue("  Own answer\nwith conditions  ");
  const row = q(page, "q2");
  await other(row).check();
  await expect(custom(row)).toBeVisible();
  await custom(row).fill("row answer");
  await row.locator('[data-action="open"]').click();
  await row.locator('[data-action="open"]').click();
  await expect(custom(row)).toHaveValue("row answer");
});

test("blank custom explains refusal but deferral may be stamped", async ({ shoryo, page }) => {
  await setup(shoryo, page);
  const scope = q(page);
  await other(scope).check();
  await custom(scope).fill("　\u0085");
  await scope.locator('[data-action="stamp"]').click();
  await expect(scope.locator('[data-action="stamp"]')).toHaveAttribute("data-stamp", "none");
  await expect(scope.locator("[data-empty-answer]")).toBeVisible();
  await scope.getByRole("switch").focus();
  await scope.getByRole("switch").press("Space");
  await scope.locator('[data-action="stamp"]').click();
  await expect(scope.locator('[data-action="stamp"]')).toHaveAttribute("data-stamp", "person");
});
