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
  await action(card(page, "q2"), "quick-ask").first().click();
  await askFreely(card(page, "q4"), "Who else could delete it?");

  await expect(mark(card(page, "q2"), "writing")).toBeVisible();
  await expect(mark(card(page, "q4"), "writing")).toBeVisible();
  await card(page, "q2").getByRole("radio", { name: /A database/ }).check();
  await expect(card(page, "q2").locator("[data-consequence]")).toHaveText(
    "With A database, the topic goes this way.",
  );
  await row(page, "q3").getByRole("combobox").selectOption({ label: "q3 no" });
  await expect(row(page, "q3").locator("[data-consequence]")).toHaveText(
    "With q3 no, the topic goes this way.",
  );
  await askIds(shoryo, 2);
});

test("ask_sent_with_enter_shows_writing_then_the_reply_without_leaving_the_field", async ({ shoryo, page }) => {
  const field = card(page, "q2").locator("[data-field=ask]");
  await field.fill("Why a file?");

  await field.press("Enter");
  await expect(mark(card(page, "q2"), "writing")).toBeVisible();
  await expect(card(page, "q2").locator("[data-field=ask]")).toHaveValue("");
  await card(page, "q2").locator("[data-field=ask]").press("Enter");
  const [ask] = await askIds(shoryo, 1);
  await shoryo.reply(ask, { text: "Because a file moves with the person." });

  await expect(card(page, "q2").getByText("Because a file moves with the person.")).toBeVisible();
  await expect(card(page, "q2").locator("[data-field=ask]")).toBeFocused();
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

test("thread_starts_latest_only_and_switches_to_all_and_hidden", async ({ shoryo, page }) => {
  const q2 = card(page, "q2");
  await askFreely(q2, "First question");
  const [first] = await askIds(shoryo, 1);
  await shoryo.reply(first, { text: "First reply" });
  await expect(q2.getByText("First reply")).toBeVisible();
  await askFreely(q2, "Second question");
  const [, second] = await askIds(shoryo, 2);
  await shoryo.reply(second, { text: "Second reply" });

  await expect(q2.getByText("Second reply")).toBeVisible();
  await expect(q2.getByText("First reply")).toHaveCount(0);

  await q2.locator("[data-thread-mode=all]").click();
  await expect(q2.getByText("First reply")).toBeVisible();
  await expect(q2.getByText("Second reply")).toBeVisible();

  await q2.locator("[data-thread-mode=hidden]").click();
  await expect(q2.getByText("First reply")).toHaveCount(0);
  await expect(q2.getByText("Second reply")).toHaveCount(0);
});

test("follow_up_attaches_to_chosen_reply", async ({ shoryo, page }) => {
  const q2 = card(page, "q2");
  await askFreely(q2, "First question");
  const [first] = await askIds(shoryo, 1);
  await shoryo.reply(first, { text: "First reply" });
  await askFreely(q2, "Unrelated question");
  const [, other] = await askIds(shoryo, 2);
  await shoryo.reply(other, { text: "Unrelated reply" });
  await q2.locator("[data-thread-mode=all]").click();

  await action(q2.locator(`[data-ask="${first}"]`), "follow-up").click();
  await askFreely(q2, "And after that?");

  const asks = await askEvents(shoryo, 3);
  expect(asks[2].follows).toBe(first);
  await expect(
    q2.locator(`[data-ask="${first}"] [data-ask]`).getByText("And after that?"),
  ).toBeVisible();
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
