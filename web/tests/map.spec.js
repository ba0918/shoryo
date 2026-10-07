// The map, the finished picture, and moving around without losing one's place
// (docs/spec/screen.md, "地図", "完成図", "画面の構成").
import { test, expect } from "./fixtures.js";
import { decision, question, twoRounds } from "./rounds.js";
import { action, mark } from "./screen.js";

const tab = (page, id) => page.locator(`[data-tab="${id}"]`);
const node = (page, key) => page.locator(`[data-map] [data-map-node="${key}"]`);

/// Round 3 adds d2 (from q2, on d1) and d3 (from q4, on nothing), and rejects an option of q2.
const roundThree = {
  subject: "Access",
  questions: [question("q5", "Who may read the file?", { premises: ["d2"] })],
  records: {
    decisions: [decision("d2", "Data in one file", "q2"), decision("d3", "Only the person deletes", "q4")],
    rejected: [{ text: "A database", reason: "Needs a server", question: "q2" }],
  },
  finished_picture: "a = Screen\nb = One file\nc = ? Who reads\n| a | b | c |\na -> b : saves",
};

test.beforeEach(async ({ shoryo, page }) => {
  await twoRounds(shoryo);
  await shoryo.submit();
  await shoryo.round(roundThree);
  await page.goto(shoryo.url);
  await tab(page, "map").click();
  await expect(page.locator("[data-map]")).toBeVisible();
});

test("hover_details_do_not_move_the_map", async ({ page }) => {
  const map = page.locator("[data-map]");
  const heading = page.getByRole("heading", { level: 1 });
  const before = [await map.boundingBox(), await heading.boundingBox()];

  await node(page, "d:d2").hover();

  const detail = page.locator("[data-map-hover]");
  await expect(detail).toBeVisible();
  await expect(detail).toContainText("Data in one file, written out in full.");
  await expect(detail).toContainText("Where does the data live?");
  await expect(detail).toContainText("One JSON file");
  expect([await map.boundingBox(), await heading.boundingBox()]).toEqual(before);
});

test("jump_highlights_the_destination_card", async ({ page }) => {
  await node(page, "d:d2").click();
  await action(page, "jump").click();

  await expect(page.locator("[data-panel=past]")).toBeVisible();
  await expect(page.locator('[data-past-round="2"] [data-past-question="q2"]')).toHaveAttribute("data-landed", "");
});

test("back_from_past_round_restores_map_range_and_selection", async ({ page }) => {
  await node(page, "d:d2").click();
  await page.locator("[data-map-range=path]").click();
  await action(page, "jump").click();
  await expect(page.locator("[data-panel=past]")).toBeVisible();

  await action(page, "back").click();

  await expect(page.locator("[data-panel=map]")).toBeVisible();
  await expect(page.locator("[data-map-range=path]")).toHaveAttribute("aria-pressed", "true");
  await expect(node(page, "d:d2")).toHaveAttribute("data-selected", "");
});

test("rejected_options_are_dashed", async ({ page }) => {
  const rejected = node(page, "r:0");
  await expect(rejected).toContainText("A database");
  const dashes = await rejected.locator("rect").evaluate((r) => getComputedStyle(r).strokeDasharray);
  expect(dashes).not.toBe("none");
  const edge = page.locator('[data-map] [data-map-edge="d:d1->r:0"] path');
  expect(await edge.evaluate((p) => getComputedStyle(p).strokeDasharray)).not.toBe("none");
  const solid = page.locator('[data-map] [data-map-edge="d:d1->d:d2"] path');
  expect(await solid.evaluate((p) => getComputedStyle(p).strokeDasharray)).toBe("none");
});

test("decision_in_review_is_marked_on_the_map", async ({ shoryo, page }) => {
  await shoryo.op({ op: "request_review", decision: "d1" });

  await expect(mark(node(page, "d:d1"), "in-review")).toBeVisible();
  await expect(mark(node(page, "d:d2"), "in-review")).toHaveCount(0);
});

test("finished_picture_opens_from_every_tab", async ({ page }) => {
  for (const id of ["current", "past", "map", "decisions"]) {
    await tab(page, id).click();
    await action(page, "finished-picture").click();
    const picture = page.locator("[data-finished-picture]");
    await expect(picture.locator('[data-node="b"]')).toContainText("One file");
    await action(picture, "close").click();
    await expect(picture).toHaveCount(0);
  }
});

test("path_range_shows_only_prerequisites", async ({ page }) => {
  await node(page, "d:d2").click();

  await page.locator("[data-map-range=path]").click();

  await expect(node(page, "d:d2")).toBeVisible();
  await expect(node(page, "d:d1")).toBeVisible();
  await expect(node(page, "r:0")).toBeVisible();
  await expect(node(page, "d:d3")).toHaveCount(0);
  await expect(node(page, "q:q5")).toHaveCount(0);
});

test("review_marks_appear_on_dependents", async ({ shoryo, page }) => {
  await shoryo.submit();
  await shoryo.round({
    subject: "Revisit",
    questions: [question("q6", "Anything else?")],
    records: { decisions: [decision("d1", "Person and agent read cards", "q1")] },
  });

  await expect(mark(node(page, "d:d2"), "review")).toBeVisible();
  await expect(mark(node(page, "d:d3"), "review")).toHaveCount(0);
  await expect(mark(node(page, "d:d1"), "review")).toHaveCount(0);
});

test("finished_picture_draws_empty_slots_dashed", async ({ page }) => {
  await action(page, "finished-picture").click();

  const slot = page.locator('[data-finished-picture] [data-node="c"] rect');
  await expect(slot).toBeVisible();
  expect(await slot.evaluate((r) => getComputedStyle(r).strokeDasharray)).not.toBe("none");
  const filled = page.locator('[data-finished-picture] [data-node="b"] rect');
  expect(await filled.evaluate((r) => getComputedStyle(r).strokeDasharray)).toBe("none");
});

test("decision_source_link_jumps_to_its_question", async ({ page }) => {
  await page.locator('[data-tab="decisions"]').click();
  const item = page.locator('[data-panel=decisions] [data-record=decisions] [data-decision-item="d2"]');

  await action(item, "go-to-source").click();

  await expect(page.locator('[data-past-round="2"] [data-past-question="q2"]')).toHaveAttribute("data-landed", "");
  await action(page, "back").click();
  await expect(page.locator("[data-panel=decisions]")).toBeVisible();
});
