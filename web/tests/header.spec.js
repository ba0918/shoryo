// The fixed header, the language and the theme (docs/spec/screen.md, "ヘッダーと切替",
// "言語", "テーマ", "切替は設定に残る", "画面の構成").
import { test, expect } from "./fixtures.js";
import { option, question, roundOne, twoRounds } from "./rounds.js";
import { action, card } from "./screen.js";

const printableAscii = /^[\x20-\x7E]+$/;
const banner = (page) => page.getByRole("banner");
const tab = (page, id) => page.locator(`[data-tab="${id}"]`);
const background = (page) => page.evaluate(() => getComputedStyle(document.body).backgroundColor);

test.describe("with a readable config", () => {
  test.beforeEach(async ({ shoryo, page }) => {
    await twoRounds(shoryo);
    await page.goto(shoryo.url);
  });

  test("switching_to_ja_translates_fixed_text_and_leaves_agent_text", async ({ page }) => {
    await banner(page).locator('[data-language="ja"]').click();

    await expect(tab(page, "current")).toHaveText("今のラウンド");
    await expect(tab(page, "map")).toHaveText("地図");
    await expect(action(page, "send")).toHaveText("まとめて送る");
    await expect(card(page, "q2").getByRole("heading")).toHaveText("Where does the data live?");
    await expect(page.getByRole("heading", { level: 1 })).toHaveText(roundOne.title);
  });

  test("refusal_of_an_action_is_shown_in_the_chosen_language", async ({ shoryo, page }) => {
    await banner(page).locator('[data-language="ja"]').click();
    const note = card(page, "q2").locator("[data-field=note] textarea");
    await page.clock.install();
    await note.fill("Written while the round is sent from another tab.");
    await shoryo.submit();
    await expect(page.locator("[data-sent-notice]")).toBeAttached();

    await note.blur();

    const refusal = page.getByRole("alert");
    await expect(refusal).toBeVisible();
    await expect(refusal).toHaveText(/[\u3040-\u30ff\u4e00-\u9fff]/);
    await expect(refusal).not.toHaveText(/[A-Za-z]{3,}/);
  });

  test("ja_screen_says_question_and_review_in_the_glossary_words", async ({ shoryo, page }) => {
    await banner(page).locator('[data-language="ja"]').click();
    await shoryo.op({ op: "ask", question: "q2", text: "Why one file?" });
    await shoryo.reply(1, { text: "Because it moves with the person." });
    const q2 = card(page, "q2");
    const pageText = () => page.locator("body").innerText();

    await expect(action(q2, "thread-toggle")).toHaveText(/^LLM への質問/);
    await expect(action(q2, "follow-up")).toHaveText("この返事に続けて質問");
    const currentText = await pageText();
    await tab(page, "decisions").click();
    await expect(action(page, "review").first()).toHaveText("見直す");
    const decisionsText = await pageText();

    for (const text of [currentText, decisionsText]) {
      expect(text).not.toContain("聞き返し");
      expect(text).not.toContain("見直したい");
    }
  });

  test("default_language_is_english", async ({ page }) => {
    await expect(banner(page).locator('[data-language="en"]')).toHaveAttribute("aria-pressed", "true");
    for (const id of ["current", "past", "map", "decisions"]) {
      expect((await tab(page, id).textContent()).trim()).toMatch(printableAscii);
    }
  });

  test("theme_icon_cycles_light_dark_system", async ({ page }) => {
    await page.emulateMedia({ colorScheme: "dark" });
    const theme = banner(page).locator("[data-action=theme]");
    const osDark = await background(page);

    await theme.click();
    const light = await background(page);
    await theme.click();
    const dark = await background(page);
    await theme.click();
    const system = await background(page);
    await page.emulateMedia({ colorScheme: "light" });
    const systemNowLight = await background(page);

    expect(light).not.toBe(osDark);
    expect(dark).toBe(osDark);
    expect(system).toBe(osDark);
    expect(systemNowLight).toBe(light);
  });

  test("theme_follows_os_when_unset", async ({ page }) => {
    await page.emulateMedia({ colorScheme: "light" });
    const light = await background(page);

    await page.emulateMedia({ colorScheme: "dark" });

    expect(await background(page)).not.toBe(light);
  });

  test("original_request_is_reachable_from_the_header_on_every_tab", async ({ page }) => {
    await page.setViewportSize({ width: 1280, height: 360 });
    for (const id of ["current", "past", "map", "decisions"]) {
      await tab(page, id).click();
      const scrolled = await page.evaluate(() => {
        window.scrollTo(0, document.documentElement.scrollHeight);
        return window.scrollY;
      });
      if (id === "current") expect(scrolled).toBeGreaterThan(0);
      const request = banner(page).locator("details");
      await expect(request).toBeInViewport();
      await request.locator("summary").click();
      await expect(request).toContainText(roundOne.original_request);
      await request.locator("summary").click();
    }
  });

  test("finished_picture_opens_from_the_tab_row_on_every_tab", async ({ page }) => {
    const tabRow = page.locator("[data-tab-row]");
    for (const id of ["current", "past", "map", "decisions"]) {
      await tab(page, id).click();

      await action(tabRow, "finished-picture").click();
      await expect(page.locator("[data-finished-picture]")).toBeVisible();
      await page.keyboard.press("Escape");
      await expect(page.locator("[data-finished-picture]")).toHaveCount(0);
    }
    await expect(action(banner(page), "finished-picture")).toHaveCount(0);
  });

  test("page_title_is_the_topic_title", async ({ page }) => {
    await expect(page).toHaveTitle(roundOne.title);
  });
});

test("back_control_is_in_the_header_and_only_when_there_is_history", async ({ shoryo, page }) => {
  await twoRounds(shoryo);
  await page.goto(shoryo.url);
  await expect(action(page, "back")).toHaveCount(0);

  await tab(page, "decisions").click();
  await action(page.locator('[data-panel=decisions] [data-decision-item="d1"]'), "go-to-source").click();
  const back = action(banner(page), "back");
  await expect(back).toBeVisible();
  await page.mouse.wheel(0, 4000);
  await expect(back).toBeInViewport();

  await back.click();
  await expect(action(page, "back")).toHaveCount(0);
});

test("header_stays_fixed_while_scrolling", async ({ shoryo, page }) => {
  await shoryo.round({
    ...roundOne,
    questions: Array.from({ length: 8 }, (_, index) =>
      question(`q${index + 1}`, `Question number ${index + 1}?`, {
        options: [option(`Yes ${index}`, true), option(`No ${index}`, false)],
      }),
    ),
  });
  await page.goto(shoryo.url);
  await expect(card(page, "q8")).toBeAttached();

  await page.mouse.wheel(0, 4000);

  await expect(card(page, "q1")).not.toBeInViewport();
  const box = await banner(page).boundingBox();
  expect(box.y).toBe(0);
  await expect(page.getByRole("heading", { level: 1 })).toBeInViewport();
});

test.describe("with a broken config", () => {
  test.use({ configText: 'language = "ja\nnot toml at all' });

  test.beforeEach(async ({ shoryo, page }) => {
    await twoRounds(shoryo);
    await page.goto(shoryo.url);
  });

  test("unreadable_config_is_shown_in_the_header", async ({ page }) => {
    await expect(banner(page).locator("[data-config-unreadable]")).toBeVisible();
  });

  test("header_lists_back_title_status_config_notice_theme_and_language_in_order", async ({ page }) => {
    await tab(page, "decisions").click();
    await action(page.locator('[data-panel=decisions] [data-decision-item="d1"]'), "go-to-source").click();
    const parts = [
      action(banner(page), "back"),
      page.getByRole("heading", { level: 1 }),
      action(banner(page), "arrivals"),
      banner(page).locator("[data-agent-status]"),
      banner(page).locator("[data-config-unreadable]"),
      action(banner(page), "theme"),
      banner(page).locator('[data-language="en"]'),
    ];
    const lefts = [];
    for (const part of parts) {
      await expect(part).toBeVisible();
      lefts.push((await part.boundingBox()).x);
    }

    expect(lefts).toEqual([...lefts].sort((a, b) => a - b));
  });

  test("switches_still_work_for_the_page_when_config_is_unreadable", async ({ page }) => {
    await banner(page).locator('[data-language="ja"]').click();

    await expect(tab(page, "current")).toHaveText("今のラウンド");
  });
});
