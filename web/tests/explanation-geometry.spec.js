import { test, expect } from "./fixtures.js";
import { question } from "./rounds.js";
import { action, card } from "./screen.js";

const message = (from, to, label = "Send", extra = {}) => ({ type: "message", from, to, label, kind: "call", ...extra });
const sequence = (extra = {}) => ({ type: "sequence", title: "Exchange", role: "example", participants: [{ id: "a", label: "First" }, { id: "b", label: "Second" }], events: [message("a", "b")], ...extra });
const flow = (extra = {}) => ({ type: "flow", title: "Path", role: "proposal", nodes: [{ id: "a", kind: "process", label: "First" }, { id: "b", kind: "process", label: "Second" }], edges: [{ from: "a", to: "b" }], ...extra });

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

test("flow_shapes_and_conditions_preserve_connections", async ({ shoryo, page }) => {
  const scope = await show(shoryo, page, [flow({ nodes: [{ id: "a", kind: "start", label: "Same" }, { id: "b", kind: "decision", label: "Choose" }, { id: "c", kind: "process", label: "Work" }, { id: "d", kind: "end", label: "Same" }], edges: [{ from: "a", to: "b" }, { from: "b", to: "c", label: { text: "Yes" } }, { from: "c", to: "d" }] })]);
  await expect(scope.locator("[data-node]")).toHaveCount(4);
  expect(await scope.locator("[data-node]").evaluateAll(nodes => nodes.map(n => [n.tagName, n.dataset.kind]))).toEqual([["rect", "start"], ["polygon", "decision"], ["rect", "process"], ["rect", "end"]]);
  await expect(scope.locator("[data-terminal-marker]")).toHaveText(["Start", "End"]);
  await expect(scope.locator("[data-node-label]")).toHaveText(["Same", "Choose", "Work", "Same"]);
  expect(await scope.locator('[data-node="a"]').getAttribute("height")).toBe("68");
  await page.locator('[data-language="ja"]').click();
  await expect(scope.locator("[data-terminal-marker]")).toHaveText(["開始", "終了"]);
  await expect(scope.locator("[data-node-label]").first()).toHaveText("Same");
});

test("flow_explicit_positions_ports_and_routes_are_preserved", async ({ shoryo, page }) => {
  const scope = await show(shoryo, page, [flow({ nodes: [{ id: "a", kind: "process", label: "A", position: { x: 200, y: 150 }, width: 200, height: 100 }, { id: "b", kind: "decision", label: "B", position: { x: 600, y: 350 }, width: 240, height: 120 }], edges: [{ from: "a", to: "b", from_port: "east", to_port: "west", via: [{ x: 400, y: 150 }, { x: 400, y: 350 }], label: { text: "Route", position: { x: 360, y: 100 } } }] })]);
  await expect(scope.locator("[data-edge]")).toHaveAttribute("d", "M 300 150 L 400 150 L 400 350 L 480 350");
  const label = await box(scope.locator("[data-edge-label]"));
  expect(label.x + label.width / 2).toBeCloseTo(360, 0);
  const before = await scope.locator("[data-edge]").getAttribute("d");
  await page.setViewportSize({ width: 390, height: 700 });
  await expect(scope.locator("[data-edge]")).toHaveAttribute("d", before);
});

test("flow_defaults_and_label_ties_are_deterministic", async ({ shoryo, page }) => {
  const scope = await show(shoryo, page, [flow({ edges: [{ from: "a", to: "b", label: { text: "First\nSecond" } }] }), flow({ nodes: [{ id: "a", kind: "decision", label: "Pick" }, { id: "b", kind: "process", label: "Do" }], edges: [{ from: "a", to: "b", label: { text: "Yes" } }] })]);
  const edges = scope.locator("[data-edge]");
  await expect(edges.nth(0)).toHaveAttribute("d", "M 160 132 L 160 180 L 160 228");
  await expect(edges.nth(1)).toHaveAttribute("d", "M 160 148 L 160 188 L 160 228");
  const label = scope.locator("[data-edge-label]").first();
  await expect(label).toHaveAttribute("x", "168");
  await expect(label).toHaveAttribute("y", "138");
  await expect(label.locator("tspan").nth(1)).toHaveAttribute("y", "156");
});

test("flow_return_and_merge_keep_supplied_routes", async ({ shoryo, page }) => {
  const scope = await show(shoryo, page, [flow({ nodes: [{ id: "a", kind: "process", label: "A", position: { x: 200, y: 100 } }, { id: "b", kind: "process", label: "B", position: { x: 200, y: 400 } }, { id: "c", kind: "process", label: "C", position: { x: 600, y: 400 } }], edges: [{ from: "a", to: "b" }, { from: "c", to: "b", from_port: "west", to_port: "east", via: [] }, { from: "b", to: "a", from_port: "west", to_port: "west", via: [{ x: 40, y: 400 }, { x: 40, y: 100 }], label: { text: "Repeat", position: { x: 70, y: 250 } } }] })]);
  await expect(scope.locator("[data-edge]").nth(1)).toHaveAttribute("d", "M 510 400 L 290 400");
  await expect(scope.locator("[data-edge]").nth(2)).toHaveAttribute("d", "M 110 400 L 40 400 L 40 100 L 110 100");
});

test("independent_boxes_and_labels_fail_on_overlap", async ({ shoryo, page }) => {
  const crowded = flow({ nodes: [{ id: "a", kind: "process", label: "A", position: { x: 200, y: 100 } }, { id: "b", kind: "process", label: "B", position: { x: 240, y: 120 } }], edges: [] });
  const labels = flow({ edges: [{ from: "a", to: "b", label: { text: "One", position: { x: 400, y: 200 } } }, { from: "a", to: "b", label: { text: "Two", position: { x: 400, y: 200 } } }] });
  const scope = await show(shoryo, page, [crowded, labels, flow()]);
  await expect(scope.locator("[data-part=flow]").nth(0).locator("[data-layout-failure]")).toContainText(/a.*b|b.*a/);
  await expect(scope.locator("[data-part=flow]").nth(1).locator("[data-layout-failure]")).toContainText(/label/i);
  await expect(scope.locator("[data-part=flow]").nth(2).locator("svg")).toBeVisible();
});

test("owned_labels_ports_self_calls_and_frame_enclosure_are_legal", async ({ shoryo, page }) => {
  const scope = await show(shoryo, page, [sequence({ events: [{ type: "loop", condition: "A readable condition that needs several lines but keeps its words ".repeat(3), messages: [message("a", "a", "Self")] }] }), flow({ nodes: [{ id: "a", kind: "start", label: "Same" }, { id: "b", kind: "end", label: "Same" }], edges: [{ from: "a", to: "b" }] }), flow({ edges: [{ from: "a", to: "b", label: { text: "First\nSecond" } }] })]);
  await expect(scope.locator("svg")).toHaveCount(3);
  await expect(scope.locator("[data-layout-failure]")).toHaveCount(0);
});

test("arrows_entering_unrelated_shapes_fail_but_crossings_are_legal", async ({ shoryo, page }) => {
  const nodes = [{ id: "a", kind: "process", label: "A", position: { x: 150, y: 200 } }, { id: "b", kind: "process", label: "B", position: { x: 650, y: 200 } }, { id: "c", kind: "process", label: "C", position: { x: 400, y: 200 } }];
  const through = flow({ nodes, edges: [{ from: "a", to: "b", from_port: "east", to_port: "west", via: [] }] });
  const clear = flow({ nodes, edges: [{ from: "a", to: "b", from_port: "south", to_port: "south", via: [{ x: 150, y: 350 }, { x: 650, y: 350 }] }, { from: "a", to: "b", from_port: "south", to_port: "south", via: [{ x: 200, y: 400 }, { x: 600, y: 250 }] }] });
  const scope = await show(shoryo, page, [through, clear]);
  await expect(scope.locator("[data-part=flow]").nth(0).locator("[data-layout-failure]")).toContainText(/arrow.*c/i);
  await expect(scope.locator("[data-part=flow]").nth(1).locator("svg")).toBeVisible();
});

test("diamond_and_capsule_containment_use_actual_shapes", async ({ shoryo, page }) => {
  const smallDiamond = flow({ nodes: [{ id: "a", kind: "decision", label: "Wide text", width: 120, height: 32 }], edges: [] });
  const smallTerminal = flow({ nodes: [{ id: "a", kind: "start", label: "Same", height: 32 }], edges: [] });
  const scope = await show(shoryo, page, [smallDiamond, smallTerminal, flow({ nodes: [{ id: "a", kind: "decision", label: "Wide text", width: 220 }], edges: [] })]);
  for (const i of [0, 1]) await expect(scope.locator("[data-part=flow]").nth(i).locator("[data-layout-failure]")).toContainText(/contain|outside/i);
  await expect(scope.locator("[data-part=flow]").nth(2).locator("svg")).toBeVisible();
});

test("canvas_and_text_clipping_fail_only_the_affected_part", async ({ shoryo, page }) => {
  const clipped = flow({ canvas: { width: 100, height: 100 } });
  const negative = flow({ nodes: [{ id: "a", kind: "process", label: '<script>window.executed=true</script>', position: { x: 20, y: 100 } }], edges: [] });
  const overflow = sequence({ participants: [{ id: "a", label: "A", x: 8192 }], events: [message("a", "a")] });
  const scope = await show(shoryo, page, [{ type: "text", body: "Still readable" }, clipped, negative, overflow, flow()]);
  await expect(scope.locator("[data-layout-failure]")).toHaveCount(3);
  await expect(scope.locator("svg")).toHaveCount(1);
  await expect(scope.getByText("Still readable")).toBeVisible();
  expect(JSON.parse(await scope.locator(".explanation-failure").nth(1).textContent())).toEqual(negative);
  expect(await page.evaluate(() => window.executed)).toBeUndefined();
});

test("font_and_theme_changes_revalidate_lossless_labels", async ({ shoryo, page }) => {
  const scope = await show(shoryo, page, [flow({ nodes: [{ id: "a", kind: "process", label: "Fits at baseline", height: 40 }], edges: [] })]);
  await expect(scope.locator("svg")).toBeVisible();
  await page.addStyleTag({ content: ":root { font-size: 32px; }" });
  await expect(scope.locator("[data-layout-failure]")).toBeVisible();
  await page.addStyleTag({ content: ":root { font-size: 16px; }" });
  await expect(scope.locator("svg")).toBeVisible();
  await page.locator('[data-action="theme"]').click();
  await expect(scope.locator("[data-node-label]")).toHaveText("Fits at baseline");
});

test("malformed_view_is_bounded_before_geometry_measurement", async ({ shoryo, page }) => {
  await show(shoryo, page, [flow()]);
  await page.evaluate(async () => {
    const { explanationDiagram } = await import("./components/explanation-diagram.js");
    const bad = { type: "flow", title: "Malformed", role: "example", nodes: Array.from({ length: 33 }, (_, i) => ({ id: `n${i}`, kind: "process", label: "Node" })), edges: [] };
    const host = explanationDiagram(bad, "en");
    host.id = "malformed-view";
    document.body.append(host);
  });
  await expect(page.locator("#malformed-view [data-layout-failure]")).toContainText(/nodes.*32/i);
  await expect(page.locator("#malformed-view svg")).toHaveCount(0);
});

test("boundary_contact_is_legal_but_connected_routes_cannot_enter_nodes", async ({ shoryo, page }) => {
  const nodes = [{ id: "a", kind: "process", label: "A", position: { x: 200, y: 100 } }, { id: "b", kind: "process", label: "B", position: { x: 380, y: 100 } }];
  const contact = flow({ nodes, edges: [] });
  const backwards = flow({ edges: [{ from: "a", to: "b", from_port: "north", to_port: "north", via: [] }] });
  const scope = await show(shoryo, page, [contact, backwards]);
  await expect(scope.locator("[data-part=flow]").nth(0).locator("svg")).toBeVisible();
  await expect(scope.locator("[data-part=flow]").nth(1).locator("[data-layout-failure]")).toContainText(/arrow.*a/i);
});

test("circular_terminal_capsules_do_not_overlap_at_a_distance", async ({ shoryo, page }) => {
  const scope = await show(shoryo, page, [flow({ nodes: [{ id: "a", kind: "start", label: "A", position: { x: 150, y: 100 }, width: 80, height: 80 }, { id: "b", kind: "end", label: "B", position: { x: 150, y: 300 }, width: 80, height: 80 }], edges: [] })]);
  await expect(scope.locator("svg")).toBeVisible();
  await expect(scope.locator("[data-layout-failure]")).toHaveCount(0);
});
