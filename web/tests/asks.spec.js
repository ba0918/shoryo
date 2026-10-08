// Asking back about a question, the replies, and the diagrams in them
// (docs/spec/screen.md, "聞き返し", "図"). The agent's replies come from the reply command.
import { test, expect } from "./fixtures.js";
import { twoRounds } from "./rounds.js";
import { action, card, mark, row } from "./screen.js";

test.beforeEach(async ({ shoryo, page }) => {
  await twoRounds(shoryo);
  await page.goto(shoryo.url);
});

async function askFreely(scope, text) {
  await scope.locator("[data-field=ask]").fill(text);
  await action(scope, "ask").click();
}

/// The ask events the agent has received so far, oldest first, once there are `count`.
async function askEvents(shoryo, count) {
  let asks = [];
  await expect
    .poll(async () => {
      asks = (await shoryo.wait()).filter((event) => event.kind === "ask");
      return asks.length;
    })
    .toBe(count);
  return asks;
}

async function askIds(shoryo, count) {
  return (await askEvents(shoryo, count)).map((event) => event.ask);
}

test("two_asks_pending_while_other_cards_stay_usable", async ({ shoryo, page }) => {
  await askFreely(card(page, "q2"), "Why a file?");
  await askFreely(card(page, "q4"), "Who else could delete it?");

  await expect(mark(card(page, "q2"), "writing")).toBeVisible();
  await expect(mark(card(page, "q4"), "writing")).toBeVisible();
  await card(page, "q2").getByRole("radio", { name: /A database/ }).check();
  await expect(card(page, "q2").locator("[data-consequence]")).toHaveText(
    "With A database, the topic goes this way.",
  );
  await row(page, "q3").getByRole("radio", { name: /q3 no/ }).check();
  await expect(row(page, "q3").locator("[data-consequence]")).toHaveText(
    "With q3 no, the topic goes this way.",
  );
  await askIds(shoryo, 2);
});

test("ask_sent_with_enter_shows_writing_then_the_reply", async ({ shoryo, page }) => {
  const field = card(page, "q2").locator("[data-field=ask]");
  await field.fill("Why a file?");

  await field.press("Enter");
  await expect(mark(card(page, "q2"), "writing")).toBeVisible();
  const [ask] = await askIds(shoryo, 1);
  await shoryo.reply(ask, { text: "Because a file moves with the person." });

  await expect(card(page, "q2").getByText("Because a file moves with the person.")).toBeVisible();
  expect((await shoryo.wait()).filter((event) => event.kind === "ask")).toHaveLength(1);
});

test("reply_appears_under_its_card", async ({ shoryo, page }) => {
  await askFreely(card(page, "q2"), "Why a file?");
  await askFreely(card(page, "q4"), "Who else?");
  const [first, second] = await askIds(shoryo, 2);

  await shoryo.reply(first, { text: "Because a file moves with the person." });

  await expect(card(page, "q2").getByText("Because a file moves with the person.")).toBeVisible();
  await expect(mark(card(page, "q2"), "writing")).toHaveCount(0);
  await expect(card(page, "q4").getByText("Because a file moves")).toHaveCount(0);
  await expect(mark(card(page, "q4"), "writing")).toBeVisible();
  expect(second).not.toBe(first);
});

const exchangeTexts = (scope) => scope.locator("[data-ask] .asked-text").allTextContents();

test("thread_opens_and_closes_and_starts_open_when_there_are_asks", async ({ shoryo, page }) => {
  const q2 = card(page, "q2");
  await expect(action(q2, "thread-toggle")).toHaveCount(0);
  await askFreely(q2, "First question");
  const [first] = await askIds(shoryo, 1);
  await shoryo.reply(first, { text: "First reply" });

  const toggle = action(q2, "thread-toggle");
  await expect(toggle).toHaveText("Ask back (1)");
  await expect(toggle).toHaveAttribute("aria-expanded", "true");
  await expect(q2.getByText("First reply")).toBeVisible();

  await toggle.click();
  await expect(toggle).toHaveAttribute("aria-expanded", "false");
  await expect(q2.getByText("First reply")).toHaveCount(0);
  await toggle.click();
  await expect(q2.getByText("First reply")).toBeVisible();

  await page.reload();
  await expect(card(page, "q2").getByText("First reply")).toBeVisible();
  await expect(action(card(page, "q4"), "thread-toggle")).toHaveCount(0);
});

const longReply = "The first reply runs on well past forty characters, so its quote is cut.";

test("follow_up_is_placed_in_time_order_with_a_quote_of_the_reply_it_continues", async ({ shoryo, page }) => {
  const q2 = card(page, "q2");
  await askFreely(q2, "First question");
  const [first] = await askIds(shoryo, 1);
  await shoryo.reply(first, { text: longReply });
  await askFreely(q2, "Unrelated question");
  const [, other] = await askIds(shoryo, 2);
  await shoryo.reply(other, { text: "Unrelated reply" });

  await action(q2.locator(`[data-ask="${first}"]`), "follow-up").click();
  await askFreely(q2, "And after that?");

  const asks = await askEvents(shoryo, 3);
  expect(asks[2].follows).toBe(first);
  await expect.poll(() => exchangeTexts(q2)).toEqual(["First question", "Unrelated question", "And after that?"]);
  await expect(q2.locator("[data-ask] [data-ask]")).toHaveCount(0);
  const quote = q2.locator(`[data-ask="${asks[2].ask}"] [data-quote]`);
  await expect(quote).toHaveText(`${[...longReply].slice(0, 40).join("")}…`);

  await page.setViewportSize({ width: 1280, height: 300 });
  await quote.scrollIntoViewIfNeeded();
  await page.mouse.wheel(0, 400);
  const quoted = q2.locator(`[data-ask="${first}"] [data-reply]`);
  await expect(quoted).not.toBeInViewport();
  await quote.click();
  await expect(quoted).toBeInViewport();
});

test("more_than_five_exchanges_collapse_the_middle_and_expand_on_click", async ({ shoryo, page }) => {
  const q2 = card(page, "q2");
  const texts = ["one", "two", "three", "four", "five", "six"];
  for (const text of texts) await askFreely(q2, text);
  await askIds(shoryo, 6);

  await expect.poll(() => exchangeTexts(q2)).toEqual(["one", "five", "six"]);
  const more = action(q2, "show-all-exchanges");
  await expect(more).toContainText("3");

  await more.click();
  await expect.poll(() => exchangeTexts(q2)).toEqual(texts);
  await expect(action(q2, "show-all-exchanges")).toHaveCount(0);
});

test("preset_ask_button_fills_the_input_without_sending", async ({ shoryo, page }) => {
  const q2 = card(page, "q2");
  await action(q2, "quick-ask").first().click();

  await expect(q2.locator("[data-field=ask]")).toHaveValue("Explain more");
  await expect(q2.locator("[data-field=ask]")).toBeFocused();
  await expect(mark(q2, "writing")).toHaveCount(0);
  expect((await shoryo.wait()).filter((event) => event.kind === "ask")).toHaveLength(0);
});

test("send_button_and_enter_send_the_ask", async ({ shoryo, page }) => {
  const q2 = card(page, "q2");
  const field = q2.locator("[data-field=ask]");
  await field.fill("Sent with the button");
  await q2.getByRole("button", { name: "Send the ask" }).click();
  await field.fill("Sent with Enter");
  await field.press("Enter");

  const asks = await askEvents(shoryo, 2);
  expect(asks.map((ask) => ask.text)).toEqual(["Sent with the button", "Sent with Enter"]);
  await expect(mark(q2, "writing")).toHaveCount(2);
});

test("shift_enter_inserts_a_newline_and_enter_sends", async ({ shoryo, page }) => {
  const field = card(page, "q2").locator("[data-field=ask]");
  await field.click();
  await field.pressSequentially("First line");
  await field.press("Shift+Enter");
  await field.pressSequentially("Second line");
  await expect(field).toHaveValue("First line\nSecond line");

  await field.press("Enter");

  const [ask] = await askEvents(shoryo, 1);
  expect(ask.text).toBe("First line\nSecond line");
});

const gridDiagram = [
  "a = Screen",
  "b = Server",
  "c = Data file",
  "| a | b |",
  "| . | c |",
  "a -> b : sends answers",
  "b -> a : pushes replies",
  "b -> c : saves",
].join("\n");

async function replyWithDiagram(shoryo, page) {
  await askFreely(card(page, "q2"), "Show me");
  const [ask] = await askIds(shoryo, 1);
  await shoryo.reply(ask, { text: "Here is the picture.", diagram: gridDiagram });
  const diagram = card(page, "q2").locator("[data-diagram]");
  await expect(diagram).toBeVisible();
  return diagram;
}

test("diagram_nodes_sit_at_given_grid_positions", async ({ shoryo, page }) => {
  const diagram = await replyWithDiagram(shoryo, page);

  const box = async (id) => diagram.locator(`[data-node="${id}"]`).boundingBox();
  const [a, b, c] = [await box("a"), await box("b"), await box("c")];
  expect(a.x).toBeLessThan(b.x);
  expect(Math.abs(a.y - b.y)).toBeLessThan(1);
  expect(Math.abs(b.x - c.x)).toBeLessThan(1);
  expect(c.y).toBeGreaterThan(b.y + b.height);
  await expect(diagram.locator('[data-node="a"]')).toContainText("Screen");
});

function intersects(p, q) {
  return p.x < q.x + q.width && q.x < p.x + p.width && p.y < q.y + q.height && q.y < p.y + p.height;
}

test("reverse_edges_do_not_overlap", async ({ shoryo, page }) => {
  const diagram = await replyWithDiagram(shoryo, page);

  const thereLabel = await diagram.locator('[data-edge="a->b"]').getByText("sends answers").boundingBox();
  const backLabel = await diagram.locator('[data-edge="b->a"]').getByText("pushes replies").boundingBox();
  expect(intersects(thereLabel, backLabel)).toBe(false);
});
