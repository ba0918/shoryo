import { test, expect } from "./fixtures.js";
import { question } from "./rounds.js";
import { action, card } from "./screen.js";

const body = "  first();  \n\tsecond();\n";
const parts = [
  { type: "text", body: "Before the sample" },
  { type: "code", title: "Sample", language: "unknown-language", role: "example", body },
  { type: "text", body: "After the sample" },
];

async function prepare(shoryo, page, content = parts) {
  const q = question("q1", "Which approach?");
  q.background = content;
  q.options[0].description = content;
  await shoryo.round({ subject: "Reading", questions: [q] });
  await page.goto(shoryo.url);
  await action(card(page, "q1"), "open").click();
  return card(page, "q1");
}

test("parts_read_in_order_in_each_location_and_history", async ({ shoryo, page }) => {
  const scope = await prepare(shoryo, page);
  for (const location of [scope.locator(".context"), scope.locator(".option-description").first()]) {
    await expect(location.locator("[data-part]")).toHaveCount(3);
    expect(await location.locator("[data-part]").evaluateAll(items => items.map(item => item.dataset.part))).toEqual(["text", "code", "text"]);
    await expect(location.locator("pre code")).toHaveText(body, { useInnerText: false });
  }
  await shoryo.op({ op: "ask", question: "q1", text: "Explain" });
  await shoryo.reply(1, { parts });
  await expect(scope.locator("[data-reply] [data-part]")).toHaveCount(3);
  await shoryo.submit();
  await shoryo.round({ subject: "Next", questions: [question("q2", "Next question?")] });
  await page.getByRole("tab", { name: "Past rounds" }).click();
  await action(page.locator("[data-past-question=q1]"), "open").click();
  await expect(page.getByText("Before the sample").first()).toBeVisible();
  await expect(page.locator("[data-reply] pre code")).toHaveText(body, { useInnerText: false });
});

test("copy_preserves_code_body_and_reports_denial", async ({ shoryo, page, context }) => {
  const scope = await prepare(shoryo, page);
  await context.grantPermissions(["clipboard-read", "clipboard-write"]);
  await scope.locator(".context").getByRole("button", { name: "Copy code", exact: true }).click();
  await expect.poll(() => page.evaluate(() => navigator.clipboard.readText())).toBe(body);
  await expect(scope.locator(".context [role=status]")).not.toBeEmpty();
  await page.evaluate(() => Object.defineProperty(navigator, "clipboard", { configurable: true, value: { writeText: () => Promise.reject(new DOMException("Denied", "NotAllowedError")) } }));
  await scope.locator(".context").getByRole("button", { name: "Copy code", exact: true }).click();
  await expect(scope.locator(".context [role=status]")).not.toBeEmpty();
  await scope.locator(".context").getByRole("button", { name: "Select code" }).click();
  expect(await page.evaluate(() => getSelection().getRangeAt(0).cloneContents().textContent)).toBe(body);
});

test("option_detail_controls_do_not_change_answers", async ({ shoryo, page }) => {
  const scope = await prepare(shoryo, page);
  await shoryo.op({ op: "choose", question: "q1", option: 1 });
  await shoryo.op({ op: "stamp", question: "q1", stamped: true });
  const detail = scope.locator(".option-description").first();
  await detail.getByRole("button", { name: "Copy code", exact: true }).click();
  await expect(scope.locator("[data-stamp=person]")).toHaveCount(1);
  await expect(scope.getByRole("radio").nth(1)).toBeChecked();
  await shoryo.submit();
  await expect(detail.getByRole("button", { name: "Copy code", exact: true })).toBeEnabled();
  await expect(scope.getByRole("radio").first()).toBeDisabled();
});

test("clipboard_unavailable_exposes_keyboard_reachable_manual_selection", async ({ shoryo, page }) => {
  const scope = await prepare(shoryo, page);
  await page.evaluate(() => Object.defineProperty(navigator, "clipboard", { configurable: true, value: undefined }));
  const copy = scope.locator(".context").getByRole("button", { name: "Copy code", exact: true });
  await copy.focus();
  await copy.press("Enter");
  await expect(scope.locator(".context [role=status]")).not.toBeEmpty();
  await copy.press("Tab");
  const select = scope.locator(".context").getByRole("button", { name: "Select code" });
  await expect(select).toBeFocused();
  await select.press("Enter");
  expect(await page.evaluate(() => getSelection().getRangeAt(0).cloneContents().textContent)).toBe(body);
});

for (const location of ["background", "option", "reply"]) {
  test(`manual_copy_fallback_and_focus_survive_live_redraw_in_${location}`, async ({ shoryo, page }) => {
    const content = [parts[1], { ...parts[1], body: "Different body with the same title" }];
    const scope = await prepare(shoryo, page, content);
    await shoryo.op({ op: "ask", question: "q1", text: "First" });
    await shoryo.reply(1, { parts: content });
    await expect(scope.locator("[data-reply] [data-part=code]")).toHaveCount(2);
    await page.evaluate(() => Object.defineProperty(navigator, "clipboard", { configurable: true, value: undefined }));
    const containers = { background: scope.locator(".context"), option: scope.locator(".option-description").first(), reply: scope.locator('[data-ask="1"] [data-reply]') };
    const container = containers[location];
    const code = container.locator("[data-part=code]").first();
    const copy = code.getByRole("button", { name: "Copy code", exact: true });
    await copy.focus();
    await copy.press("Enter");
    const feedback = await code.getByRole("status").textContent();
    expect(feedback).not.toBe("");
    await copy.press("Tab");
    const select = code.getByRole("button", { name: "Select code", exact: true });
    await expect(select).toBeFocused();
    await shoryo.op({ op: "ask", question: "q1", text: "Live update" });
    await expect(scope.locator('[data-ask="2"]')).toHaveCount(1);
    await expect(select).toBeVisible();
    await expect(select).toBeFocused();
    await shoryo.reply(2, { parts: [{ type: "text", body: "Arrived" }] });
    await expect(scope.locator('[data-ask="2"] [data-reply]')).toContainText("Arrived");
    await expect(select).toBeFocused();
    await expect(code.getByRole("status")).toHaveText(feedback);
    await select.press("Enter");
    expect(await page.evaluate(() => getSelection().getRangeAt(0).cloneContents().textContent)).toBe(body);
    await expect(container.locator("[data-part=code]").nth(1).getByRole("button", { name: "Select code", exact: true })).toBeHidden();
    for (const [name, other] of Object.entries(containers)) {
      if (name !== location) await expect(other.getByRole("status").first()).toBeEmpty();
    }
    await shoryo.submit();
    const next = question("q2", "Next?");
    next.background = [{ ...parts[1], body: "New round body" }];
    await shoryo.round({ subject: "Next", questions: [next] });
    await action(card(page, "q2"), "open").click();
    const nextCode = card(page, "q2").locator(".context [data-part=code]");
    await expect(nextCode.locator("pre code")).toHaveText("New round body");
    await expect(nextCode.getByRole("status")).toBeEmpty();
    await expect(nextCode.getByRole("button", { name: "Select code", exact: true })).toBeHidden();
  });
}

test("pending_clipboard_denial_updates_the_surviving_code_after_live_redraw", async ({ shoryo, page }) => {
  const scope = await prepare(shoryo, page);
  await page.evaluate(() => Object.defineProperty(navigator, "clipboard", { configurable: true, value: { writeText: () => new Promise((resolve, reject) => { window.denyCopy = () => reject(new DOMException("Denied", "NotAllowedError")); }) } }));
  const code = scope.locator(".context [data-part=code]");
  const copy = code.getByRole("button", { name: "Copy code", exact: true });
  await copy.focus();
  await copy.press("Enter");
  await expect(code.getByRole("status")).toBeEmpty();
  await shoryo.op({ op: "ask", question: "q1", text: "Live update" });
  await expect(scope.locator('[data-ask="1"]')).toHaveCount(1);
  await expect(copy).toBeFocused();
  await page.evaluate(() => window.denyCopy());
  await expect(code.getByRole("status")).not.toBeEmpty();
  await copy.press("Tab");
  await expect(code.getByRole("button", { name: "Select code", exact: true })).toBeFocused();
});

test("provisional_code_fallback_and_focus_survive_live_reply_and_language_change", async ({ shoryo, page }) => {
  const q = question("q1", "Which approach?");
  q.class = "provisional";
  q.background = [parts[1]];
  await shoryo.round({ subject: "Provisional", questions: [q] });
  await page.goto(shoryo.url);
  const scope = page.locator('[data-provisional-row="q1"]');
  await action(scope, "open").click();
  await page.evaluate(() => Object.defineProperty(navigator, "clipboard", { configurable: true, value: undefined }));
  const code = scope.locator(".context [data-part=code]");
  await code.getByRole("button", { name: "Copy code", exact: true }).click();
  const select = code.getByRole("button").nth(1);
  await select.focus();
  await shoryo.op({ op: "ask", question: "q1", text: "Live update" });
  await shoryo.reply(1, { parts: [{ type: "text", body: "Arrived" }] });
  await expect(scope.locator("[data-reply]")).toContainText("Arrived");
  await expect(select).toBeFocused();
  await page.locator('[data-language="ja"]').click();
  await expect(select).toBeVisible();
  await expect(code.getByRole("status")).not.toBeEmpty();
  await select.focus();
  await select.press("Enter");
  expect(await page.evaluate(() => getSelection().getRangeAt(0).cloneContents().textContent)).toBe(body);
});

test("legacy_first_parts_remain_folded_until_details_are_opened", async ({ shoryo, page }) => {
  const q = question("q1", "Read the diagram?");
  q.background = [{ type: "diagram", title: "An existing diagram", role: "proposal", source: "a = First\nb = Second\n| a | b |\na -> b : Next" }, ...parts];
  await shoryo.round({ subject: "Legacy", questions: [q] });
  await page.goto(shoryo.url);
  const scope = card(page, "q1");
  await expect(scope.locator(".context")).toHaveCount(0);
  await action(scope, "open").click();
  expect(await scope.locator(".context [data-part]").evaluateAll(items => items.map(item => item.dataset.part))).toEqual(["diagram", "text", "code", "text"]);
  await expect(scope.locator(".context svg")).toHaveCount(1);
  await expect(scope.locator(".context").getByText("An existing diagram")).toBeVisible();
  await expect(scope.locator(".context").getByRole("button", { name: /Enlarge/ })).toHaveCount(0);
});

test("roles_and_languages_are_visible", async ({ shoryo, page }) => {
  const content = ["proposal", "example", "confirmed"].map(role => ({ type: "code", body: "x", language: "pseudocode", role }));
  const scope = await prepare(shoryo, page, content);
  for (const role of ["Proposal", "Example", "Confirmed current state"]) await expect(scope.locator(".context").getByText(role, { exact: true })).toBeVisible();
  await expect(scope.locator(".context").getByText("Pseudocode", { exact: true })).toHaveCount(3);
});

test("first_part_identifies_follow_up_without_source_dump", async ({ shoryo, page }) => {
  const scope = await prepare(shoryo, page);
  await shoryo.op({ op: "ask", question: "q1", text: "Why?" });
  await shoryo.reply(1, { parts: [{ type: "code", body: "DO_NOT_QUOTE_BODY", language: "rust", role: "proposal", title: "Chosen sample" }] });
  await action(scope.locator("[data-ask='1']"), "follow-up").click();
  await expect(scope.locator(".following-text")).toHaveText("Chosen sample");
  await shoryo.op({ op: "ask", question: "q1", text: "Continue", follows: 1 });
  await expect(scope.locator("[data-quote]")).toHaveText("Chosen sample");
});

test("part_content_does_not_execute_or_fetch", async ({ shoryo, page }) => {
  const requested = [];
  page.on("request", request => { if (request.url().includes("untrusted.invalid")) requested.push(request.url()); });
  const source = '<img src="https://untrusted.invalid/image" onerror="window.partExecuted=true"><script>window.partExecuted=true</script>';
  const scope = await prepare(shoryo, page, [{ type: "text", body: source }, { type: "code", body: source, language: "html", role: "example" }]);
  await expect(scope.locator(".context [data-part=text]")).toHaveText(source);
  expect(await page.evaluate(() => window.partExecuted)).toBeUndefined();
  expect(requested).toEqual([]);
  await expect(scope.locator(".context img, .context script, .context a")).toHaveCount(0);
});

test("legacy_first_lists_remain_ordered_in_all_locations_and_history", async ({ shoryo, page }) => {
  const content = [{ type: "diagram", title: "Existing grid", role: "example", source: "a = First\nb = Second\n| a | b |\na -> b : Next" }, ...parts];
  const scope = await prepare(shoryo, page, content);
  await shoryo.op({ op: "ask", question: "q1", text: "Keep that order" });
  await shoryo.reply(1, { parts: content });
  const checkOrder = async container => {
    expect(await container.locator("[data-part]").evaluateAll(nodes => nodes.map(n => n.dataset.part))).toEqual(["diagram", "text", "code", "text"]);
  };
  await expect(scope.locator("[data-reply] [data-part]")).toHaveCount(4);
  for (const container of [scope.locator(".context"), scope.locator(".option-description").first(), scope.locator("[data-reply]")]) await checkOrder(container);
  await shoryo.submit();
  await shoryo.round({ subject: "Next", questions: [question("q2", "Next?")] });
  await page.getByRole("tab", { name: "Past rounds" }).click();
  const past = page.locator("[data-past-question=q1]");
  await action(past, "open").click();
  for (const container of [past.locator(".context"), past.locator(".option-description").first(), past.locator("[data-reply]")]) await checkOrder(container);
  await expect(past.locator(".option-description").first().getByRole("button", { name: "Copy code", exact: true })).toBeEnabled();
});

test("first_part_quotes_cover_every_kind_and_code_without_title", async ({ shoryo, page }) => {
  const scope = await prepare(shoryo, page, []);
  const text = "🦀".repeat(41);
  const cases = [
    [{ type: "text", body: text }, "🦀".repeat(40) + "…"],
    [{ type: "code", body: "not the quote", language: "unknown-language", role: "example" }, "unknown-language — Example"],
    [{ type: "diagram", title: "Legacy title", role: "proposal", source: "a = Label" }, "Legacy title"],
    [{ type: "sequence", title: "Sequence title", role: "proposal", participants: [{ id: "a", label: "A" }], events: [{ type: "message", from: "a", to: "a", label: "Send", kind: "call" }] }, "Sequence title"],
    [{ type: "flow", title: "Flow title", role: "confirmed", nodes: [{ id: "a", label: "A", kind: "process" }], edges: [] }, "Flow title"],
  ];
  for (const [index, [part, quote]] of cases.entries()) {
    const id = index * 2 + 1;
    await shoryo.op({ op: "ask", question: "q1", text: "Explain this part" });
    await shoryo.reply(id, { parts: [part, { type: "text", body: "Must not be quoted" }] });
    await action(scope.locator(`[data-ask="${id}"]`), "follow-up").click();
    await expect(scope.locator(".following-text")).toHaveText(quote);
    await shoryo.op({ op: "ask", question: "q1", text: "Continue", follows: id });
    await expect(scope.locator(`[data-ask="${id+1}"] [data-quote]`)).toHaveText(quote);
  }
});

test("part_roles_and_reserved_language_switch_without_changing_content", async ({ shoryo, page }) => {
  const content = ["proposal", "example", "confirmed"].map(role => ({ type: "code", language: "pseudocode", role, body: "same literal body" }));
  const scope = await prepare(shoryo, page, content);
  await page.locator('[data-language="ja"]').click();
  for (const role of ["案", "説明用の例", "確認済みの現状"]) await expect(scope.locator(".context").getByText(role, { exact: true })).toBeVisible();
  await expect(scope.locator(".context").getByText("疑似コード", { exact: true })).toHaveCount(3);
  await expect(scope.locator(".context pre code")).toHaveText(["same literal body", "same literal body", "same literal body"]);
});
