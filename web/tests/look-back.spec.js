// Past rounds, the decisions tab, review requests, review rounds and the ended state
// (docs/spec/screen.md, "過去のラウンド", "決まったこと", "見直したい", "直したこと";
// docs/spec/server.md, "終える").
import { test, expect } from "./fixtures.js";
import { decision, question, twoRounds } from "./rounds.js";
import { action, card, mark } from "./screen.js";

const tab = (page, id) => page.locator(`[data-tab="${id}"]`);
const pastRound = (page, number) => page.locator(`[data-past-round="${number}"]`);
const decisionItem = (scope, id) => scope.locator(`[data-decision-item="${id}"]`);

async function askAndReply(shoryo, page, id, text, diagram) {
  await card(page, id).locator("[data-field=ask]").fill("Show me");
  await action(card(page, id), "ask").click();
  let ask;
  await expect
    .poll(async () => {
      ask = (await shoryo.wait()).find((event) => event.kind === "ask" && event.question === id);
      return ask !== undefined;
    })
    .toBe(true);
  await shoryo.reply(ask.ask, { text, diagram });
  await expect(card(page, id).getByText(text)).toBeVisible();
}

const roundThree = {
  subject: "Access",
  questions: [question("q5", "Who may read the file?", { premises: ["d2"] })],
  records: {
    decisions: [decision("d2", "Data in one file", "q2"), decision("d3", "Only the person deletes", "q4")],
  },
};

test.beforeEach(async ({ shoryo, page }) => {
  await twoRounds(shoryo);
  await page.goto(shoryo.url);
});

test("past_round_shows_replies_and_diagrams_as_answered", async ({ shoryo, page }) => {
  await askAndReply(shoryo, page, "q2", "A file is easy to move.", "a = File\nb = Disk\n| a | b |\na -> b : lives on");
  await card(page, "q2").getByRole("radio", { name: /A database/ }).check();
  await card(page, "q2").locator("[data-field=note] textarea").fill("Only while testing.");
  await card(page, "q2").locator("[data-field=note] textarea").blur();
  await expect(card(page, "q2").locator("[data-field=note] textarea")).toHaveValue("Only while testing.");
  await action(page, "send").click();
  await shoryo.round(roundThree);

  await tab(page, "past").click();
  await action(page.locator('[data-round-choice="2"]'), "choose-round").click();

  const q2 = pastRound(page, 2).locator('[data-past-question="q2"]');
  await expect(q2.locator("[data-chosen]")).toContainText("A database");
  await expect(q2.locator("[data-recommended]")).toContainText("One JSON file");
  await expect(q2.getByText("Only while testing.")).toBeVisible();
  await expect(q2.getByText("A file is easy to move.")).toBeVisible();
  await expect(q2.locator('[data-diagram] [data-node="a"]')).toContainText("File");
});

test("past_round_shows_decision_as_of_that_round", async ({ shoryo, page }) => {
  await action(page, "send").click();
  await shoryo.round(roundThree);
  await shoryo.op({ op: "submit" });
  await shoryo.round({
    subject: "Revisit",
    questions: [question("q6", "Anything else?")],
    records: { decisions: [decision("d2", "Data in two files", "q2")] },
  });

  await tab(page, "past").click();
  await action(page.locator('[data-round-choice="2"]'), "choose-round").click();

  const item = decisionItem(pastRound(page, 2).locator('[data-past-question="q2"]'), "d2");
  await expect(item).toContainText("Data in one file");
  await expect(item).not.toContainText("Data in two files");
  await expect(mark(item, "revised")).toBeVisible();
});

test("answer_sent_unseen_is_marked_in_past_round_and_decisions_tab", async ({ shoryo, page }) => {
  await card(page, "q2").getByRole("radio", { name: /A database/ }).check();
  await action(page, "send").click();
  await shoryo.round(roundThree);

  await tab(page, "past").click();
  await action(page.locator('[data-round-choice="2"]'), "choose-round").click();
  const marks = (id) => pastRound(page, 2).locator(`[data-past-question="${id}"] [data-question-marks]`);
  await expect(mark(marks("q4"), "sent-unseen")).toBeVisible();
  await expect(mark(marks("q2"), "sent-unseen")).toHaveCount(0);

  await tab(page, "decisions").click();
  const decisions = page.locator("[data-panel=decisions]");
  await expect(mark(decisionItem(decisions, "d3"), "sent-unseen")).toBeVisible();
  await expect(mark(decisionItem(decisions, "d2"), "sent-unseen")).toHaveCount(0);
});

test("decisions_tab_lists_the_seven_kinds", async ({ shoryo, page }) => {
  await action(page, "send").click();
  await shoryo.round({
    ...roundThree,
    records: {
      ...roundThree.records,
      not_building: ["A mobile app"],
      rejected: [{ text: "A database", reason: "Needs a server", question: "q2" }],
      undecided: [{ text: "Backup schedule", decider: "the person" }],
      delegated: [{ text: "The file name", reason: "Any name works the same" }],
    },
  });
  await shoryo.op({ op: "submit" });
  await shoryo.round({
    subject: "Revisit",
    questions: [question("q6", "Anything else?")],
    records: { decisions: [decision("d2", "Data in two files", "q2")] },
  });
  await shoryo.op({ op: "request_review", decision: "d3" });

  await tab(page, "decisions").click();

  const decisions = page.locator("[data-panel=decisions]");
  for (const [kind, text] of [
    ["decisions", "Person reads cards"],
    ["not-building", "A mobile app"],
    ["rejected", "Needs a server"],
    ["undecided", "the person"],
    ["delegated", "Any name works the same"],
    ["revisions", "Data in two files"],
    ["in-review", "Only the person deletes"],
  ]) {
    await expect(decisions.locator(`[data-record="${kind}"]`)).toContainText(text);
  }
});

test("review_request_can_be_stopped_before_next_round", async ({ shoryo, page }) => {
  await tab(page, "decisions").click();
  const item = decisionItem(page.locator("[data-panel=decisions]"), "d1");

  await action(item, "review").click();
  await expect(mark(item, "in-review")).toBeVisible();
  await action(item, "stop-review").click();

  await expect(mark(item, "in-review")).toHaveCount(0);
  const kinds = (await shoryo.wait()).map((event) => event.kind);
  expect(kinds).toContain("review_requested");
  expect(kinds).toContain("review_stopped");
});

test("decision_in_review_is_marked_in_chains", async ({ page }) => {
  await tab(page, "decisions").click();
  await action(decisionItem(page.locator("[data-panel=decisions]"), "d1"), "review").click();

  await tab(page, "current").click();

  await expect(mark(card(page, "q2").locator("[data-chain]"), "in-review")).toBeVisible();
});

const reviewRound = {
  subject: "Review",
  review: true,
  fixes: [
    { text: "Named the person who reads", decision: "d1" },
    { text: "Fixed a typo in the background" },
  ],
  questions: [question("q7", "Is the review complete?")],
  records: { decisions: [decision("d1", "The person reads cards alone", "q1")] },
};

test("review_round_lists_fixed_items_before_cards", async ({ shoryo, page }) => {
  await action(page, "send").click();
  await shoryo.round(reviewRound);

  const order = await page
    .locator("[data-panel=current] [data-fixes], [data-panel=current] [data-card]")
    .evaluateAll((elements) => elements.map((e) => (e.dataset.fixes !== undefined ? "fixes" : e.dataset.card)));
  expect(order).toEqual(["fixes", "q7"]);
  await expect(page.locator("[data-fixes]")).toContainText("Named the person who reads");
});

test("fixed_item_that_changed_a_decision_offers_review_request", async ({ shoryo, page }) => {
  await action(page, "send").click();
  await shoryo.round(reviewRound);

  const items = page.locator("[data-fixes] [data-fix]");
  await expect(items.nth(0)).toContainText("Person reads cards");
  await expect(items.nth(0)).toContainText("The person reads cards alone");
  await expect(action(items.nth(1), "review")).toHaveCount(0);

  await action(items.nth(0), "review").click();

  await expect(mark(items.nth(0), "in-review")).toBeVisible();
});

test("ended_topic_blocks_all_input", async ({ shoryo, page }) => {
  await action(page, "send").click();
  await shoryo.end();

  await expect(page.locator("[data-ended]")).toBeVisible();
  await expect(action(page, "send")).toBeDisabled();
  await expect(card(page, "q2").locator("[data-field=ask]")).toHaveCount(0);
  await expect(action(card(page, "q2"), "swap")).toBeDisabled();
  await tab(page, "decisions").click();
  await expect(action(page.locator("[data-panel=decisions]"), "review")).toHaveCount(0);
});
