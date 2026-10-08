// Stamping each question, confirming before sending, and the result of a converged topic
// (docs/spec/screen.md, "判子", "仮決めの一覧", "まとめて送る", "結果").
import { test, expect } from "./fixtures.js";
import { decision, question, twoRounds } from "./rounds.js";
import {
  action,
  card,
  mark,
  reviewAndConfirm,
  reviewConfirmation,
  row,
  sendAll,
  shownIn,
  stampDate,
  stampTime,
  storedTopic,
} from "./screen.js";

const stamp = (scope) => action(scope, "stamp");
const confirmation = (page) => page.locator("[data-confirm-send]");

async function submittedEvents(shoryo, round) {
  return (await shoryo.wait()).filter((event) => event.kind === "submitted" && event.round === round);
}

test.describe("on a round with questions", () => {
  test.beforeEach(async ({ shoryo, page }) => {
    await twoRounds(shoryo);
    await page.goto(shoryo.url);
  });

  test("send_is_disabled_until_every_question_is_stamped", async ({ page }) => {
    await expect(action(page, "send")).toBeDisabled();
    await stamp(card(page, "q2")).click();
    await expect(action(page, "send")).toBeDisabled();

    await stamp(card(page, "q4")).click();

    await expect(action(page, "send")).toBeEnabled();
  });

  test("unstamped_count_jumps_to_first_unstamped_question", async ({ page }) => {
    await page.setViewportSize({ width: 1280, height: 420 });
    await stamp(card(page, "q2")).click();
    const count = page.locator("[data-unstamped-count]");
    await expect(count).toContainText("1");

    await count.click();

    await expect(card(page, "q4")).toHaveAttribute("data-landed", "");
    await expect(stamp(card(page, "q4"))).toBeInViewport();
  });

  test("provisional_row_arrives_pre_approved_and_can_be_sent_back", async ({ page }) => {
    const q3 = row(page, "q3");
    await expect(stamp(q3)).toHaveAttribute("data-stamp", "pre_approved");

    await stamp(q3).click();
    await expect(stamp(q3)).toHaveAttribute("data-stamp", "none");
    await stamp(q3).click();
    await expect(stamp(q3)).toHaveAttribute("data-stamp", "person");
    await q3.getByRole("radio", { name: /q3 no/ }).check();

    await expect(stamp(q3)).toHaveAttribute("data-stamp", "none");
  });

  test("send_all_opens_a_confirmation_listing_each_answer_and_its_stamp", async ({ shoryo, page }) => {
    await card(page, "q2").getByRole("radio", { name: /A database/ }).check();
    await card(page, "q2").locator("[data-field=note] textarea").fill("Only while testing.");
    await card(page, "q2").locator("[data-field=note] textarea").blur();
    await stamp(card(page, "q2")).click();
    await stamp(card(page, "q4")).click();

    await action(page, "send").click();

    const listed = confirmation(page);
    await expect(listed.locator('[data-confirm-question="q2"]')).toContainText("A database");
    await expect(listed.locator('[data-confirm-question="q2"]')).toContainText("Only while testing.");
    await expect(mark(listed.locator('[data-confirm-question="q3"]'), "pre-approved")).toBeVisible();
    await expect(mark(listed.locator('[data-confirm-question="q2"]'), "pre-approved")).toHaveCount(0);
    await action(listed, "cancel-send").click();
    await expect(listed).toHaveCount(0);
    expect(await submittedEvents(shoryo, 2)).toHaveLength(0);

    await action(page, "send").click();
    await action(confirmation(page), "confirm-send").click();

    await expect(page.locator("[data-wait-footer]")).toBeVisible();
    expect(await submittedEvents(shoryo, 2)).toHaveLength(1);
  });
});

test.describe("with dated stamps", () => {
  // UTC+14: the local date differs from the UTC date for 14 hours a day.
  const zone = "Pacific/Kiritimati";
  test.use({ timezoneId: zone });

  test.beforeEach(async ({ shoryo, page }) => {
    await twoRounds(shoryo);
    await page.goto(shoryo.url);
  });

  const current = (page) => page.locator("[data-panel=current]");

  const answerOf = (topic, round, id) => topic.rounds[round - 1].questions.find((q) => q.id === id).answer;

  test("pressed_stamp_shows_its_month_and_day_and_the_year_and_time_on_hover_and_focus", async ({ shoryo, page }) => {
    const q2 = stamp(card(current(page), "q2"));
    await q2.click();
    await expect(q2).toHaveAttribute("data-stamp", "person");
    const shown = shownIn(answerOf(await storedTopic(shoryo), 2, "q2").stamped_at, zone);

    await expect(stampDate(q2)).toHaveText(shown.date);
    await page.mouse.move(0, 0);
    await q2.evaluate((element) => element.blur());
    await expect(stampTime(q2)).toBeHidden();
    await q2.hover();
    await expect(stampTime(q2)).toBeVisible();
    await expect(stampTime(q2)).toHaveText(shown.time);
    await page.mouse.move(0, 0);
    await expect(stampTime(q2)).toBeHidden();
    await card(current(page), "q2").locator("[data-field=note] textarea").focus();
    await page.keyboard.press("Tab");
    await expect(q2).toBeFocused();
    await expect(stampTime(q2)).toBeVisible();
  });

  test("pre_approved_stamp_has_no_date_until_sent_and_then_the_sent_date", async ({ shoryo, page }) => {
    const q3 = stamp(row(current(page), "q3"));
    await expect(q3).toHaveAttribute("data-stamp", "pre_approved");
    await expect(stampDate(q3)).toHaveCount(0);

    await sendAll(page);

    const shown = shownIn((await storedTopic(shoryo)).rounds[1].sent_at, zone);
    await expect(stampDate(q3)).toHaveText(shown.date);
    await q3.hover();
    await expect(stampTime(q3)).toHaveText(shown.time);
  });

  test("stamps_of_a_sent_current_round_keep_their_dates", async ({ shoryo, page }) => {
    const q2 = stamp(card(current(page), "q2"));
    await q2.click();
    await expect(q2).toHaveAttribute("data-stamp", "person");
    const pressed = answerOf(await storedTopic(shoryo), 2, "q2").stamped_at;

    await sendAll(page);
    await expect(page.locator("[data-wait-footer]")).toBeVisible();
    await q2.click();
    await page.mouse.move(0, 0);
    await q2.evaluate((element) => element.blur());

    await expect(q2).toHaveAttribute("data-stamp", "person");
    await expect(stampDate(q2)).toHaveText(shownIn(pressed, zone).date);
    await expect(stampTime(q2)).toBeVisible();
    await expect(stampTime(q2)).toHaveText(shownIn(pressed, zone).time);
  });

  test("tapping_an_unsent_stamp_presses_it_and_shows_no_time", async ({ page }) => {
    const q2 = stamp(card(current(page), "q2"));

    await q2.click();
    await page.mouse.move(0, 0);
    await q2.evaluate((element) => element.blur());

    await expect(q2).toHaveAttribute("data-stamp", "person");
    await expect(stampTime(q2)).toBeHidden();
  });
});

const resultRound = {
  subject: "Result",
  records: {
    decisions: [decision("d2", "Data in one file", "q2"), decision("d4", "File named state.json", "q3")],
    not_building: ["A mobile app"],
    rejected: [{ text: "A database", reason: "Needs a server", question: "q2" }],
    undecided: [{ text: "Backup schedule", decider: "the person" }],
    delegated: [{ text: "The file name", reason: "Any name works the same" }],
  },
  finished_picture: "a = Screen\nb = One file\n| a | b |\na -> b : saves",
};

test.describe("on a round without questions", () => {
  test.beforeEach(async ({ shoryo, page }) => {
    await twoRounds(shoryo);
    await shoryo.submit();
    await shoryo.round(resultRound);
    await page.goto(shoryo.url);
  });

  test("round_without_questions_shows_the_result", async ({ shoryo, page }) => {
    const result = page.locator("[data-panel=current] [data-result]");
    await expect(result.locator('[data-node="b"]')).toContainText("One file");
    for (const text of ["Data in one file", "A mobile app", "Needs a server", "Backup schedule", "Any name works the same"]) {
      await expect(result).toContainText(text);
    }
    await expect(mark(result.locator('[data-decision-item="d4"]'), "pre-approved")).toBeVisible();
    await expect(mark(result.locator('[data-decision-item="d2"]'), "pre-approved")).toHaveCount(0);
    await expect(action(page, "send")).toHaveText("Proceed with this result");

    await action(page, "send").click();
    await action(confirmation(page), "confirm-send").click();

    await expect.poll(async () => (await submittedEvents(shoryo, 3)).map((event) => event.answers)).toEqual([[]]);
  });

  test("result_with_pending_review_request_sends_review_requests_instead_of_proceed", async ({ page }) => {
    const result = page.locator("[data-panel=current] [data-result]");

    await reviewAndConfirm(page, result.locator('[data-decision-item="d2"]'));

    await expect(action(page, "send")).toHaveText("Send review requests");
    await action(page, "send").click();
    await expect(confirmation(page)).toContainText("Data in one file");
  });

  test("current_result_uses_the_result_confirmation_and_a_past_result_the_ordinary_one", async ({ shoryo, page }) => {
    const declared = "見直しを頼むで送ると、次のラウンドで問い直されます";
    await page.getByRole("banner").locator('[data-language="ja"]').click();
    const result = page.locator("[data-panel=current] [data-result]");

    await action(result.locator('[data-decision-item="d2"]'), "review").click();
    await expect(reviewConfirmation(page)).toContainText(declared);
    await action(reviewConfirmation(page), "cancel-review").click();
    await sendAll(page);
    await shoryo.round({ subject: "After the result", questions: [question("q9", "Anything else?")] });
    await page.locator('[data-tab="past"]').click();
    await action(page.locator('[data-round-choice="3"]'), "choose-round").click();
    await action(page.locator('[data-past-round="3"] [data-result] [data-decision-item="d2"]'), "review").click();

    await expect(reviewConfirmation(page)).toBeVisible();
    await expect(reviewConfirmation(page)).not.toContainText(declared);
  });
});
