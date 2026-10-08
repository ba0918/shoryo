// A map that can be read and moved around: labels inside their boxes, a details box that is
// never clipped, zoom and pan, and going back (docs/spec/screen.md, "地図", "点と線",
// "道筋と全部", "詳しい情報の出し方", "移動と現在地").
import { test, expect } from "./fixtures.js";
import { decision, question, twoRounds } from "./rounds.js";
import { action } from "./screen.js";

const tab = (page, id) => page.locator(`[data-tab="${id}"]`);
const node = (page, key) => page.locator(`[data-map] [data-map-node="${key}"]`);
const details = (page) => page.locator("[data-map-selection]");

const LONG_NAME = "人が読むカードは、たたんだままでも問いと選択肢と推奨と結果が一目でわかるようにする";

/// Round 3 adds d2 (from q2, on d1) with a long Japanese name and d3 (from q4), and rejects an
/// option of q2; it asks q5 on d2.
const roundThree = {
  subject: "Access",
  questions: [question("q5", "Who may read the file?", { premises: ["d2"] })],
  records: {
    decisions: [decision("d2", LONG_NAME, "q2"), decision("d3", "Only the person deletes", "q4")],
    rejected: [{ text: "A database", reason: "Needs a server", question: "q2" }],
  },
};

function inside(inner, outer) {
  return (
    inner.x >= outer.x - 0.5 &&
    inner.y >= outer.y - 0.5 &&
    inner.x + inner.width <= outer.x + outer.width + 0.5 &&
    inner.y + inner.height <= outer.y + outer.height + 0.5
  );
}

function intersects(p, q) {
  return p.x < q.x + q.width && q.x < p.x + p.width && p.y < q.y + q.height && q.y < p.y + p.height;
}

const nodeWidth = async (page, key) => (await node(page, key).locator("rect").boundingBox()).width;

test.describe("with a mouse", () => {
  test.beforeEach(async ({ shoryo, page }) => {
    await twoRounds(shoryo);
    await shoryo.submit();
    await shoryo.round(roundThree);
    await page.goto(shoryo.url);
    await tab(page, "map").click();
    await expect(page.locator("[data-map]")).toBeVisible();
  });

  test("long_japanese_label_stays_inside_its_node_box", async ({ page }) => {
    const box = await node(page, "d:d2").locator("rect").boundingBox();

    const lines = await node(page, "d:d2").locator("text").all();
    expect(lines.length).toBeGreaterThan(0);
    for (const line of lines) expect(inside(await line.boundingBox(), box)).toBe(true);
  });

  test("hover_box_is_fully_visible_and_has_no_null_line", async ({ page }) => {
    await page.setViewportSize({ width: 900, height: 600 });
    for (const key of ["q:q5", "d:d2"]) {
      await node(page, key).hover();
      const hover = page.locator("[data-map-hover]");
      await expect(hover).toBeVisible();
      await expect(hover).toBeInViewport({ ratio: 1 });
      await expect(hover).not.toContainText("null");
    }
  });

  test("map_zooms_and_moves_inside_its_area", async ({ page }) => {
    const area = page.locator("[data-map]");
    const before = await node(page, "d:d1").locator("rect").boundingBox();
    const scrolled = await page.evaluate(() => window.scrollY);
    const centre = await area.boundingBox();

    await page.mouse.move(centre.x + centre.width / 2, centre.y + centre.height / 2);
    await page.mouse.wheel(0, -400);
    await expect.poll(() => nodeWidth(page, "d:d1")).toBeGreaterThan(before.width);
    expect(await page.evaluate(() => window.scrollY)).toBe(scrolled);
    const zoomed = await node(page, "d:d1").locator("rect").boundingBox();
    await page.mouse.move(centre.x + 20, centre.y + centre.height - 20);
    await page.mouse.down();
    await page.mouse.move(centre.x + 120, centre.y + centre.height - 60, { steps: 5 });
    await page.mouse.up();

    const moved = await node(page, "d:d1").locator("rect").boundingBox();
    expect(moved.x).toBeGreaterThan(zoomed.x + 50);
    await action(page, "map-fit").click();
    await expect(node(page, "q:q5")).toBeInViewport();
    await expect(node(page, "d:d1")).toBeInViewport();
  });

  test("focusing_a_point_shows_its_details", async ({ page }) => {
    const before = await page.locator("[data-map]").boundingBox();
    await node(page, "d:d2").focus();
    await expect(page.locator("[data-map-hover]")).toBeVisible();
    await expect(page.locator("[data-map-hover]")).toContainText(`${LONG_NAME}, written out in full.`);
    expect(await page.locator("[data-map]").boundingBox()).toEqual(before);
    await tab(page, "map").focus();
    await expect(page.locator("[data-map-hover]")).toBeHidden();
  });

  test("focusing_a_point_outside_the_view_pans_it_into_view", async ({ page }) => {
    await page.setViewportSize({ width: 650, height: 600 });
    const before = await node(page, "q:q5").locator("rect").boundingBox();
    expect(inside(before, await page.locator("[data-map]").boundingBox())).toBe(false);
    await node(page, "q:q5").focus();
    await expect.poll(async () => inside(await node(page, "q:q5").locator("rect").boundingBox(), await page.locator("[data-map]").boundingBox())).toBe(true);
  });

  test("path_range_nodes_and_edge_labels_do_not_overlap_at_default_zoom", async ({ page }) => {
    await node(page, "d:d2").click();
    await action(details(page), "show-path").click();
    await expect(node(page, "d:d3")).toHaveCount(0);

    const boxes = [];
    for (const shape of await page.locator("[data-map] .map-node rect, [data-map] .edge-label").all()) {
      if (await shape.isVisible()) boxes.push(await shape.boundingBox());
    }
    expect(await page.locator("[data-map] .edge-label").count()).toBeGreaterThan(0);
    for (let i = 0; i < boxes.length; i++) {
      for (let j = i + 1; j < boxes.length; j++) expect(intersects(boxes[i], boxes[j])).toBe(false);
    }
  });
});

test.describe("with touch", () => {
  test.use({ hasTouch: true });

  test.beforeEach(async ({ shoryo, page }) => {
    await twoRounds(shoryo);
    await shoryo.submit();
    await shoryo.round(roundThree);
    await page.goto(shoryo.url);
    await tab(page, "map").click();
    await expect(page.locator("[data-map]")).toBeVisible();
  });

  test("tapping_a_node_opens_details_without_changing_the_range", async ({ page }) => {
    await node(page, "d:d2").tap();

    await expect(details(page)).toBeVisible();
    await expect(details(page)).toContainText(`${LONG_NAME}, written out in full.`);
    await expect(action(details(page), "show-path")).toBeVisible();
    await expect(page.locator("[data-map-range=all]")).toHaveAttribute("aria-pressed", "true");
    await expect(node(page, "d:d3")).toBeVisible();
  });
});
