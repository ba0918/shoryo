import { test, expect } from "./fixtures.js";
import { question, decision } from "./rounds.js";

async function setup(shoryo, page) {
  await shoryo.round({ subject: "Own answers", questions: [question("q1", "Your answer?"), question("q2", "Small choice?", { cls: "provisional" })] });
  await page.goto(shoryo.url);
}
const q = (page, id = "q1") => page.locator(`[data-card="${id}"], [data-provisional-row="${id}"]`);
const custom = scope => scope.locator('[data-field="free-text"] textarea');
const other = scope => scope.getByRole("radio", { name: "Other (write my answer)", exact: true });
const confirm = page => page.locator("[data-confirm-send]");

test("restamping includes a reedit made while another question's save is held", async ({ shoryo, page }) => {
  await setup(shoryo, page); await other(q(page)).check();
  await custom(q(page)).fill("first"); await custom(q(page)).blur();
  await expect.poll(async () => (await (await fetch(`${shoryo.url}api/view`)).json()).topic.rounds[0].questions[0].answer.free_text).toBe("first");
  await q(page, "q2").locator('[data-action="open"]').click();
  let release, began;
  const held = new Promise(resolve => { release = resolve; });
  const started = new Promise(resolve => { began = resolve; });
  await page.route("**/api/op", async route => {
    const op = route.request().postDataJSON();
    if (op.op === "note" && op.question === "q2") { began(); await held; }
    await route.continue();
  });
  await q(page, "q2").locator('[data-field="note"] textarea').fill("held note");
  await q(page).locator('[data-action="stamp"]').click(); await started;
  await custom(q(page)).fill("latest while waiting");
  release();
  await expect.poll(async () => (await (await fetch(`${shoryo.url}api/view`)).json()).expected_answers?.[0]?.free_text).toBe("latest while waiting");
  await page.waitForTimeout(450);
  const view = await (await fetch(`${shoryo.url}api/view`)).json();
  expect(view.topic.rounds[0].questions[0].answer.stamp).toBe("person");
});

for (const refused of [false, true]) {
  test(`a stamp waits for a preceding provisional choice ${refused ? "refusal" : "success"}`, async ({ shoryo, page }) => {
    await setup(shoryo, page);
    let release, began;
    const held = new Promise(resolve => { release = resolve; });
    const started = new Promise(resolve => { began = resolve; });
    await page.route("**/api/op", async route => {
      if (route.request().postDataJSON().op === "choose") {
        began(); await held;
        if (refused) { await route.fulfill({ status: 409, contentType: "application/json", body: JSON.stringify({ error: "choice refused" }) }); return; }
      }
      await route.continue();
    });
    const scope = q(page, "q2");
    await scope.getByRole("radio").nth(1).check(); await started;
    await scope.locator('[data-action="stamp"]').click(); release();
    if (refused) await expect(page.getByRole("alert")).toBeVisible();
    await expect(scope.locator('[data-action="stamp"]')).toHaveAttribute("data-stamp", refused ? "pre_approved" : "person");
    await page.unroute("**/api/op");
    if (refused) {
      await scope.locator('[data-action="stamp"]').click();
      await expect(scope.locator('[data-action="stamp"]')).toHaveAttribute("data-stamp", "none");
    }
  });
}

test("a failed focused note is recoverable but is never displayed as another client's sent note", async ({ shoryo, page }) => {
  await setup(shoryo, page);
  await page.route("**/api/op", async route => {
    if (route.request().postDataJSON().op === "note") await route.abort("failed"); else await route.continue();
  });
  await q(page).locator('[data-field="note"] textarea').fill("not actually sent");
  await expect(page.getByRole("alert")).toBeVisible();
  await shoryo.submit();
  const sent = q(page.locator('[data-panel="current"]'));
  await expect(sent.locator('[data-action="stamp"]')).toHaveClass(/fixed/);
  await expect(sent.locator('[data-field="note"] textarea')).toHaveValue("");
  await expect(page.getByRole("textbox").and(page.locator("[readonly]"))).toHaveValue("not actually sent");
  await page.getByRole("tab", { name: "Past rounds", exact: true }).click();
  const past = page.locator('[data-panel="past"] [data-past-question="q1"]');
  await expect(past.locator('[data-field="note"]')).toHaveCount(0);
});

for (const initiallyRequested of [false, true]) {
  test(`result confirmation reflects a review ${initiallyRequested ? "withdrawn" : "added"} while open`, async ({ shoryo, page }) => {
    await shoryo.round({ subject: "First", questions: [question("q1", "First answer?")] });
    await shoryo.submit();
    await shoryo.round({ subject: "Result", records: { decisions: [decision("d1", "Visible policy", "q1")] } });
    if (initiallyRequested) await shoryo.op({ op: "request_review", decision: "d1" });
    await page.goto(shoryo.url); await page.locator('[data-action="send"]').click();
    await expect(confirm(page)).toBeVisible();
    await shoryo.op({ op: initiallyRequested ? "stop_review" : "request_review", decision: "d1" });
    if (initiallyRequested) await expect(confirm(page)).not.toContainText("Visible policy");
    else await expect(confirm(page)).toContainText("Visible policy");
    await confirm(page).locator('[data-action="confirm-send"]').click();
    await expect(page.locator("[data-wait-footer]")).toBeVisible();
    const view = await (await fetch(`${shoryo.url}api/view`)).json();
    expect(view.topic.rounds.at(-1).sent_as).toBe(initiallyRequested ? "proceeded" : "review_requested");
  });
}

test("a failed custom save survives another tab moving its question between regions", async ({ shoryo, page, context }) => {
  await setup(shoryo, page);
  await other(q(page)).check();
  const second = await context.newPage(); await second.goto(shoryo.url);
  await page.route("**/api/op", async route => {
    if (route.request().postDataJSON().op === "free_text") await route.abort("failed");
    else await route.continue();
  });
  await custom(q(page)).fill("recover across regions"); await custom(q(page)).blur();
  await expect(page.getByRole("alert")).toBeVisible();
  await q(second).locator('[data-action="swap"]').click();
  await expect(page.locator('[data-provisional-row="q1"]')).toBeVisible();
  await expect(custom(q(page))).toHaveValue("recover across regions");
  await page.unroute("**/api/op");
  await q(page).locator('[data-action="stamp"]').click();
  await expect(q(page).locator('[data-action="stamp"]')).toHaveAttribute("data-stamp", "person");
  expect((await (await fetch(`${shoryo.url}api/view`)).json()).expected_answers[0].free_text).toBe("recover across regions");
});

test("another client's submission retains a separate recoverable draft without leaking it into the next round", async ({ shoryo, page }) => {
  await setup(shoryo, page);
  await shoryo.op({ op: "answer_method", question: "q1", method: "custom" });
  await shoryo.op({ op: "free_text", question: "q1", text: "actually saved" });
  await page.route("**/api/op", async route => { await route.abort("failed"); });
  await custom(q(page)).fill("never sent recoverable body"); await custom(q(page)).blur();
  await expect(page.getByRole("alert")).toBeVisible();
  await shoryo.submit();
  await expect(page.locator('[data-panel="current"] [data-sent-answer]')).toHaveText("actually saved");
  await expect(page.getByRole("textbox").and(page.locator("[readonly]"))).toHaveValue("never sent recoverable body");
  await shoryo.round({ subject: "Next", questions: [question("q3", "Fresh question?")] });
  await expect(q(page, "q3")).toBeVisible();
  await expect(q(page, "q3")).not.toContainText("never sent recoverable body");
  await expect(page.getByRole("textbox").and(page.locator("[readonly]"))).toHaveValue("never sent recoverable body");
});

test("save completion is awaited before stamping and timers cannot lift the new stamp", async ({ shoryo, page }) => {
  await setup(shoryo, page);
  await other(q(page)).check();
  let release;
  const held = new Promise(resolve => { release = resolve; });
  let began;
  const started = new Promise(resolve => { began = resolve; });
  const ops = [];
  await page.route("**/api/op", async route => {
    const body = route.request().postDataJSON(); ops.push(body.op);
    if (body.op === "free_text") { began(); await held; }
    await route.continue();
  });
  await custom(q(page)).fill("delayed body");
  await q(page).locator('[data-action="stamp"]').click();
  await started;
  expect(ops).not.toContain("stamp");
  release();
  await expect(q(page).locator('[data-action="stamp"]')).toHaveAttribute("data-stamp", "person");
  await page.waitForTimeout(450); // Cross the pending debounce deadline to detect a late unstamping save.
  const view = await (await fetch(`${shoryo.url}api/view`)).json();
  expect(view.topic.rounds[0].questions[0].answer.stamp).toBe("person");
});

test("network failure preserves a dirty editor and stops its dependent action", async ({ shoryo, page }) => {
  await setup(shoryo, page);
  await other(q(page)).check();
  const ops = [];
  await page.route("**/api/op", async route => {
    const body = route.request().postDataJSON(); ops.push(body.op);
    if (body.op === "free_text") await route.abort("failed"); else await route.continue();
  });
  await custom(q(page)).fill("recoverable");
  await q(page).locator('[data-action="stamp"]').click();
  await expect(page.getByRole("alert")).toContainText("could not be reached");
  expect(ops).not.toContain("stamp");
  await expect(custom(q(page))).toHaveValue("recoverable");
});

test("a dirty custom editor keeps focus and caret across unrelated SSE", async ({ shoryo, page }) => {
  await setup(shoryo, page);
  await other(q(page)).check();
  await custom(q(page)).fill("local draft");
  await custom(q(page)).evaluate(el => { el.setSelectionRange(3, 3); window.retainedCustomEditor = el; });
  await shoryo.op({ op: "ask", question: "q1", text: "unrelated update" });
  await expect(q(page)).toContainText("unrelated update");
  expect(await custom(q(page)).evaluate(el => [el === window.retainedCustomEditor, el === document.activeElement, el.selectionStart])).toEqual([true, true, 3]);
  await expect(custom(q(page))).toHaveValue("local draft");
});

test("a clean tab never writes its older field over a newer tab when confirming", async ({ shoryo, page, context }) => {
  await setup(shoryo, page);
  await q(page).locator('[data-action="stamp"]').click();
  const second = await context.newPage(); await second.goto(shoryo.url);
  await q(second).locator('[data-field="note"] textarea').fill("newer note");
  await q(second).locator('[data-field="note"] textarea').blur();
  await expect.poll(async () => (await (await fetch(`${shoryo.url}api/view`)).json()).expected_answers[0].note).toBe("newer note");
  await page.locator('[data-action="send"]').click();
  await expect(confirm(page)).toContainText("newer note");
  await confirm(page).locator('[data-action="confirm-send"]').click();
  await expect(page.locator("[data-wait-footer]")).toBeVisible();
  expect((await shoryo.wait()).find(e => e.kind === "submitted").answers[0].note).toBe("newer note");
});

test("immediate custom stamp and send use the exact body without inactive notes", async ({ shoryo, page }) => {
  await setup(shoryo, page);
  const scope = q(page);
  await scope.locator('[data-field="note"] textarea').fill("hidden note");
  await other(scope).check();
  await custom(scope).fill("  answer\ncondition  ");
  await scope.locator('[data-action="stamp"]').click();
  await expect(scope.locator('[data-action="stamp"]')).toHaveAttribute("data-stamp", "person");
  await page.locator('[data-action="send"]').click();
  await expect(confirm(page)).toContainText("answer\ncondition");
  await expect(confirm(page)).not.toContainText("hidden note");
  await confirm(page).locator('[data-action="confirm-send"]').click();
  await expect(page.locator("[data-wait-footer]")).toBeVisible();
  const sent = (await shoryo.wait()).find(e => e.kind === "submitted").answers[0];
  expect(sent.free_text).toBe("  answer\ncondition  ");
  expect(sent.note).toBe("");
});

test("a refused draft save stops stamping and preserves the unsaved body", async ({ shoryo, page }) => {
  await setup(shoryo, page);
  await other(q(page)).check();
  const operations = [];
  await page.route("**/api/op", async route => {
    const body = route.request().postDataJSON(); operations.push(body.op);
    if (body.op === "free_text") await route.fulfill({ status: 409, contentType: "application/json", body: JSON.stringify({ error: "save refused" }) });
    else await route.continue();
  });
  await custom(q(page)).fill("unsaved body");
  await q(page).locator('[data-action="stamp"]').click();
  await expect(page.getByRole("alert")).toContainText("save refused");
  expect(operations).not.toContain("stamp");
  await expect(custom(q(page))).toHaveValue("unsaved body");
});

test("confirmation stays frozen across a second page edit and stale send requires explicit reconfirmation", async ({ shoryo, page, context }) => {
  await setup(shoryo, page);
  await other(q(page)).check();
  await custom(q(page)).fill("original body");
  await q(page).locator('[data-action="stamp"]').click();
  await page.locator('[data-action="send"]').click();
  await expect(confirm(page)).toContainText("original body");
  const second = await context.newPage();
  await second.goto(shoryo.url);
  await custom(q(second)).fill("latest body");
  await q(second).locator('[data-action="stamp"]').click();
  await expect(q(second).locator('[data-action="stamp"]')).toHaveAttribute("data-stamp", "person");
  await expect(confirm(page)).toContainText("original body");
  await expect(confirm(page)).not.toContainText("latest body");
  await confirm(page).locator('[data-action="confirm-send"]').click();
  await expect(page.getByRole("alert")).toContainText("Answers changed");
  await expect(confirm(page)).toContainText("latest body");
  const view = await (await fetch(`${shoryo.url}api/view`)).json();
  expect(view.topic.rounds[0].submitted).toBe(false);
  await confirm(page).locator('[data-action="confirm-send"]').click();
  await expect(page.locator("[data-wait-footer]")).toBeVisible();
  expect((await shoryo.wait()).find(e => e.kind === "submitted").answers[0].free_text).toBe("latest body");
});

test("a held Submit still refuses a second page edit committed before server acceptance", async ({ shoryo, page, context }) => {
  await setup(shoryo, page);
  await other(q(page)).check(); await custom(q(page)).fill("before");
  await q(page).locator('[data-action="stamp"]').click();
  await page.locator('[data-action="send"]').click();
  let release, began;
  const held = new Promise(resolve => { release = resolve; });
  const started = new Promise(resolve => { began = resolve; });
  await page.route("**/api/op", async route => {
    if (route.request().postDataJSON().op === "submit") { began(); await held; }
    await route.continue();
  });
  await confirm(page).locator('[data-action="confirm-send"]').click();
  await started;
  const second = await context.newPage(); await second.goto(shoryo.url);
  await custom(q(second)).fill("after");
  await q(second).locator('[data-action="stamp"]').click();
  await expect.poll(async () => (await (await fetch(`${shoryo.url}api/view`)).json()).expected_answers?.[0]?.free_text).toBe("after");
  release();
  await expect(confirm(page)).toContainText("after");
  expect((await (await fetch(`${shoryo.url}api/view`)).json()).topic.events.some(e => e.kind === "submitted")).toBe(false);
});

test("custom card and provisional editors draw supplied view data and emit edits", async ({ shoryo, page }) => {
  await page.goto(shoryo.url);
  await page.evaluate(async () => {
    const { Card } = await import("./components/card.js");
    const { ProvisionalRow } = await import("./components/provisional.js");
    window.componentEdits = [];
    const emit = async event => { window.componentEdits.push(event); return true; };
    const data = { lang: "en", round: 99, id: "isolated", text: "Isolated answer?", why_now: "Now", background: [], chains: [], premises: [], options: [{ index: 0, text: "Choice", recommended: true, description: [], consequence: "Choice consequence" }], selected: 0, method: "custom", free_text: "supplied\nbody", emptyCustom: false, note: "inactive", deferred: false, stamp: null, stampedAt: null, asks: [], locked: false };
    for (const [cls, id] of [[Card, "isolated-card"], [ProvisionalRow, "isolated-row"]]) {
      const component = new cls(emit);
      document.body.appendChild(component.el);
      component.update({ ...data, id });
    }
  });
  for (const id of ["isolated-card", "isolated-row"]) {
    const scope = q(page, id);
    await expect(custom(scope)).toHaveValue("supplied\nbody");
    await expect(scope.locator("[data-consequence]")).toHaveCount(0);
    await custom(scope).fill("edited body");
    await custom(scope).blur();
  }
  expect(await page.evaluate(() => window.componentEdits.filter(e => e.type === "save-draft").map(e => e.op.text))).toEqual(["edited body", "edited body"]);
});

test("changing another tab without restamping returns the sender to answering", async ({ shoryo, page }) => {
  await setup(shoryo, page);
  await other(q(page)).check(); await custom(q(page)).fill("old");
  await q(page).locator('[data-action="stamp"]').click();
  await page.locator('[data-action="send"]').click();
  await shoryo.op({ op: "free_text", question: "q1", text: "not yet confirmed" });
  await confirm(page).locator('[data-action="confirm-send"]').click();
  await expect(confirm(page)).toHaveCount(0);
  await expect(q(page).locator('[data-action="stamp"]')).toHaveAttribute("data-stamp", "none");
  await expect(page.locator('[data-action="send"]')).toBeDisabled();
});

test("keyboard Other selection keeps the custom field through saved SSE and class moves", async ({ shoryo, page }) => {
  await setup(shoryo, page);
  await other(q(page)).focus(); await other(q(page)).press("Space");
  await custom(q(page)).focus(); await custom(q(page)).pressSequentially("keyboard body");
  await q(page).locator('[data-action="swap"]').focus();
  await q(page).locator('[data-action="swap"]').press("Enter");
  await expect(page.locator('[data-provisional-row="q1"]')).toBeVisible();
  await expect(custom(q(page))).toHaveValue("keyboard body");
  await expect(q(page).locator('[data-action="stamp"]')).toHaveAttribute("data-stamp", "none");
  await q(page).locator('[data-action="stamp"]').focus(); await q(page).locator('[data-action="stamp"]').press("Enter");
  await expect(q(page).locator('[data-action="stamp"]')).toHaveAttribute("data-stamp", "person");
});

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

test("sent custom bodies stay literal and read-only in history and both map contexts after acknowledgment and restart", async ({ shoryo, page }) => {
  await setup(shoryo, page);
  const body = "  <img src=x onerror=alert(1)>\nsecond line  ";
  await q(page).locator('[data-field="note"] textarea').fill("inactive note");
  await other(q(page)).check(); await custom(q(page)).fill(body);
  await q(page).locator('[data-action="stamp"]').click();
  await page.locator('[data-action="send"]').click();
  await confirm(page).locator('[data-action="confirm-send"]').click();
  await expect(page.locator("[data-wait-footer]")).toBeVisible();
  await expect(q(page.locator('[data-panel="current"]')).locator("[data-sent-answer]")).toHaveText(body);
  expect(await q(page.locator('[data-panel="current"]')).locator("[data-sent-answer]").textContent()).toBe(body);
  await expect(custom(q(page.locator('[data-panel="current"]')))).toHaveCount(0);
  await page.getByRole("tab", { name: "Map", exact: true }).click();
  await page.locator('[data-map-node="q:q1"]').focus();
  await expect(page.locator("[data-map-hover]")).toContainText(body);
  const received = await shoryo.wait();
  await shoryo.waitAfter(received.map(e => e.id), 0);
  await shoryo.round({ subject: "Next", questions: [question("q3", "Next question?")], records: { decisions: [decision("d1", "Own answer decision", "q1")] } });
  await shoryo.restart(); await page.goto(shoryo.url);
  await page.getByRole("tab", { name: "Past rounds", exact: true }).click();
  const history = page.locator('[data-panel="past"] [data-past-question="q1"]');
  await expect(history.locator("[data-chosen] dd")).toHaveText(body);
  expect(await history.locator("[data-chosen] dd").textContent()).toBe(body);
  await expect(history).not.toContainText("inactive note");
  await expect(history.locator("img")).toHaveCount(0);
  await expect(history.locator("[data-chosen] dd")).toHaveCSS("white-space", "pre-wrap");
  await page.getByRole("tab", { name: "Map", exact: true }).click();
  await page.locator('[data-map-node="d:d1"]').focus();
  await expect(page.locator("[data-map-hover]")).toContainText(body);
  await expect(page.locator("[data-map-hover] img")).toHaveCount(0);
});

test("UI blank feedback agrees with Rust for U0085 and UFEFF", async ({ shoryo, page }) => {
  await setup(shoryo, page);
  await other(q(page)).check(); await custom(q(page)).fill("\u0085");
  await q(page).locator('[data-action="stamp"]').click();
  await expect(q(page).locator("[data-empty-answer]")).toBeVisible();
  await expect(q(page).locator('[data-action="stamp"]')).toHaveAttribute("data-stamp", "none");
  await custom(q(page)).fill("\ufeff");
  await q(page).locator('[data-action="stamp"]').click();
  await expect(q(page).locator('[data-action="stamp"]')).toHaveAttribute("data-stamp", "person");
  await expect(q(page).locator("[data-empty-answer]")).toHaveCount(0);
});

test("custom deferral confirmation and sent display exclude retained body and note", async ({ shoryo, page }) => {
  await setup(shoryo, page);
  await q(page).locator('[data-field="note"] textarea').fill("inactive note");
  await other(q(page)).check(); await custom(q(page)).fill("retained custom draft");
  await q(page).getByRole("switch").focus(); await q(page).getByRole("switch").press("Space");
  await q(page).locator('[data-action="stamp"]').click();
  await page.locator('[data-action="send"]').click();
  await expect(confirm(page)).not.toContainText("retained custom draft");
  await expect(confirm(page)).not.toContainText("inactive note");
  await expect(confirm(page).locator('[data-mark="deferred"]')).toBeVisible();
  await confirm(page).locator('[data-action="confirm-send"]').click();
  await expect(page.locator("[data-wait-footer]")).toBeVisible();
  await expect(q(page.locator('[data-panel="current"]'))).not.toContainText("retained custom draft");
  await expect(custom(q(page.locator('[data-panel="current"]')))).toHaveCount(0);
});
