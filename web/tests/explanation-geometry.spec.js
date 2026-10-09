import { test, expect } from "./fixtures.js";
import { question } from "./rounds.js";
import { action, card } from "./screen.js";

const message = (from, to, label = "Send", extra = {}) => ({ type: "message", from, to, label, kind: "call", ...extra });
const sequence = (extra = {}) => ({ type: "sequence", title: "Exchange", role: "example", participants: [{ id: "a", label: "First" }, { id: "b", label: "Second" }], events: [message("a", "b")], ...extra });

async function show(shoryo, page, parts) {
  const q = question("q1", "Read this explanation?");
  q.background = parts;
  await shoryo.round({ subject: "Geometry", questions: [q] });
  await page.goto(shoryo.url);
  await page.addStyleTag({ content: ":root { font-size: 16px; }" });
  await action(card(page, "q1"), "open").click();
  return card(page, "q1").locator(".context");
}

const box = locator => locator.evaluate(element => {
  const b = element.getBBox();
  return { x: b.x, y: b.y, width: b.width, height: b.height };
});

test("sequence_order_and_explicit_spacing_are_preserved", async ({ shoryo, page }) => {
  const scope = await show(shoryo, page, [sequence({ participants: [{ id: "a", label: "First", x: 150 }, { id: "b", label: "Second", x: 450 }], events: [message("a", "b", "One", { gap_after: 100 }), message("b", "a", "Two")], layout: { event_gap: 80 } })]);
  const svg = scope.locator("svg");
  await expect(svg).toBeVisible();
  expect(await svg.locator("[data-header]").evaluateAll(nodes => nodes.map(n => Number(n.getAttribute("x"))))).toEqual([70, 370]);
  const paths = await svg.locator("[data-message]").evaluateAll(nodes => nodes.map(n => n.getAttribute("d")));
  expect(paths[0]).toBe("M 150 150 L 450 150");
  expect(paths[1]).toBe("M 450 280 L 150 280");
});

test("sequence_returns_self_calls_and_frames_are_distinct", async ({ shoryo, page }) => {
  const scope = await show(shoryo, page, [sequence({ events: [message("a", "b"), message("b", "a", "Returned", { kind: "return" }), { type: "alt", branches: [{ condition: "A long condition repeated to wrap without losing its words or its trailing spaces   ".repeat(3), messages: [message("a", "a", "Self")] }, { condition: "Otherwise", messages: [message("a", "b", "Other")] }] }, { type: "loop", condition: "Again", messages: [message("b", "a", "Loop")] }] })]);
  const svg = scope.locator("svg");
  await expect(svg).toBeVisible();
  await expect(svg.locator('[data-message][stroke-dasharray="6 4"]')).toHaveCount(1);
  const self = await svg.locator("[data-message]").nth(2).getAttribute("d");
  expect(self).toMatch(/M 124 (\S+) L 172 \1 L 172 (\S+) L 124 \2/);
  await expect(svg.locator("[data-frame]")).toHaveCount(2);
  await expect(svg.locator("[data-frame-marker]")).toHaveText(["alt", "alt", "loop"]);
  const frame = await box(svg.locator('[data-frame="alt"]'));
  const condition = await box(svg.locator("[data-condition]").first());
  expect(Number(await svg.locator("[data-condition]").first().getAttribute("x"))).toBeCloseTo(frame.x + 16, 1);
  expect(condition.x + condition.width).toBeLessThanOrEqual(frame.x + frame.width - 16 + 0.5);
});

test("sequence_defaults_are_stable_across_viewport_widths", async ({ shoryo, page }) => {
  const scope = await show(shoryo, page, [sequence()]);
  await expect(scope.locator("svg")).toBeVisible();
  const before = await scope.locator("[data-message]").getAttribute("d");
  await page.setViewportSize({ width: 400, height: 700 });
  await expect(scope.locator("[data-message]")).toHaveAttribute("d", before);
  expect(await scope.locator("[data-header]").evaluateAll(nodes => nodes.map(n => Number(n.getAttribute("x"))))).toEqual([44, 264]);
});

test("sequence_labels_wrap_without_loss", async ({ shoryo, page }) => {
  const label = "日本語ABC".repeat(20) + "  \n\nLast  ";
  const scope = await show(shoryo, page, [sequence({ events: [message("a", "b", label)] })]);
  await expect(scope.locator("svg")).toBeVisible();
  const text = scope.locator("[data-message-label]");
  expect(await text.evaluate(n => [...n.children].map(c => c.textContent).join(""))).toBe(label.replaceAll("\n", ""));
  expect(await text.locator("tspan").count()).toBeGreaterThan(4);
  expect(await text.getAttribute("data-original")).toBe(label);
  expect(await text.textContent()).not.toContain("…");
});
