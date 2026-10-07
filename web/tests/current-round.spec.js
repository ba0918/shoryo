// The current round: reading and answering it (docs/spec/screen.md, "今のラウンド").
//
// Controls are found by their data-* hooks, not by their wording: the specification fixes
// that the screen's own text is English, not what it says.
import { test, expect } from "./fixtures.js";
import { roundOne, twoRounds } from "./rounds.js";
import { action, card, mark, row } from "./screen.js";

const printableAscii = /^[\x20-\x7E]+$/;

test.beforeEach(async ({ shoryo, page }) => {
  await twoRounds(shoryo);
  await page.goto(shoryo.url);
});

test("four_tabs_with_english_fixed_text", async ({ page }) => {
  const tabs = page.getByRole("tab");
  await expect(tabs).toHaveCount(4);
  const fixed = [
    ...(await tabs.all()),
    action(page, "send"),
    action(page, "finished-picture"),
    action(card(page, "q2"), "open"),
    action(card(page, "q2"), "swap"),
    mark(card(page, "q2"), "human"),
    mark(card(page, "q2"), "recommended"),
    mark(card(page, "q2"), "unopened"),
  ];
  for (const element of fixed) {
    await expect(element).toBeVisible();
    expect((await element.textContent()).trim()).toMatch(printableAscii);
  }
  await expect(page.getByRole("heading", { level: 1 })).toHaveText(roundOne.title);
});

test("cards_come_before_the_provisional_list_and_send", async ({ page }) => {
  const order = await page
    .locator("[data-panel=current]")
    .locator("[data-card], [data-provisional-list], [data-action=send]")
    .evaluateAll((elements) =>
      elements.map((e) => e.dataset.card ?? (e.dataset.action === "send" ? "send" : "list")),
    );
  expect(order).toEqual(["q2", "q4", "list", "send"]);
});

test("card_starts_folded_and_opens_to_premise_and_option_details", async ({ page }) => {
  const q2 = card(page, "q2");
  await expect(q2.getByText("Where does the data live?")).toBeVisible();
  await expect(mark(q2, "human")).toBeVisible();
  await expect(q2.getByText("Later questions rest on q2.")).toBeVisible();
  await expect(q2.locator("[data-chain]")).toContainText("Person reads cards");
  await expect(mark(q2, "recommended")).toBeVisible();
  await expect(q2.getByText("With One JSON file, the topic goes this way.")).toBeVisible();
  await expect(mark(q2, "unopened")).toBeVisible();
  const details = [
    q2.locator("[data-premise-text]").getByText("Person reads cards, written out in full."),
    q2.getByText("Background for q2: the terms it uses."),
    q2.getByText("A database: what it means in practice."),
  ];
  for (const detail of details) await expect(detail).toBeHidden();

  await action(q2, "open").click();

  for (const detail of details) await expect(detail).toBeVisible();
});

test("recommended_option_is_preselected", async ({ page }) => {
  await expect(card(page, "q2").getByRole("radio", { name: /One JSON file/ })).toBeChecked();
  await expect(card(page, "q4").getByRole("radio", { name: /q4 yes/ })).toBeChecked();
});

test("consequence_follows_selected_option", async ({ page }) => {
  const q2 = card(page, "q2");

  await q2.getByRole("radio", { name: /A database/ }).check();

  await expect(q2.locator("[data-consequence]")).toHaveText(
    "With A database, the topic goes this way.",
  );
});

test("chain_shows_short_names_and_opens_full_text", async ({ page }) => {
  const link = card(page, "q2")
    .locator("[data-chain]")
    .getByRole("button", { name: "Person reads cards" });

  await link.click();

  const detail = page.locator("[data-decision-detail]");
  await expect(detail).toContainText("Person reads cards, written out in full.");
  await expect(detail).toContainText("Who reads the screen?");
});

test("provisional_row_opens_into_a_card", async ({ page }) => {
  const q3 = row(page, "q3");
  await expect(q3.getByText("What is the file called?")).toBeVisible();
  await q3.getByRole("combobox").selectOption({ label: "q3 no" });
  await expect(q3.locator("[data-consequence]")).toHaveText("With q3 no, the topic goes this way.");
  await expect(q3.locator("[data-field=defer]")).toBeHidden();

  await action(q3, "open").click();

  await expect(q3.locator("[data-field=defer]")).toBeVisible();
  await expect(q3.locator("[data-field=note]")).toBeVisible();
});

test("swap_moves_question_between_cards_and_list", async ({ page }) => {
  await action(card(page, "q2"), "swap").click();
  await expect(row(page, "q2")).toBeVisible();
  await expect(card(page, "q2")).toHaveCount(0);

  await action(row(page, "q3"), "swap").click();

  await expect(card(page, "q3")).toBeVisible();
  await expect(row(page, "q3")).toHaveCount(0);
});

test("unopened_mark_and_count_disappear_when_touched", async ({ page }) => {
  const count = page.locator("[data-unopened-count]");
  await expect(count).toContainText("2");

  await card(page, "q2").getByRole("radio", { name: /A database/ }).check();

  await expect(mark(card(page, "q2"), "unopened")).toHaveCount(0);
  await expect(count).toContainText("1");
});

test("send_locks_the_round", async ({ page }) => {
  await action(page, "send").click();

  await expect(page.locator("[data-sent-notice]")).toBeVisible();
  await expect(card(page, "q2").getByRole("radio", { name: /A database/ })).toBeDisabled();
  await expect(row(page, "q3").getByRole("combobox")).toBeDisabled();
  await expect(action(page, "send")).toBeDisabled();
});

test("note_typed_is_kept_when_the_page_is_reopened_without_leaving_the_field", async ({ page }) => {
  const note = card(page, "q2").locator("[data-field=note] textarea");
  await note.fill("Only while testing.");
  await page.waitForTimeout(1000);

  await page.reload();

  await expect(card(page, "q2").locator("[data-field=note] textarea")).toHaveValue("Only while testing.");
});
