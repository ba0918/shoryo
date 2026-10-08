// What the screen says after sending, and what the LLM is doing
// (docs/spec/screen.md, "まとめて送る", "結果"; docs/spec/server.md, "待つ").
import { test, expect } from "./fixtures.js";
import { question, twoRounds } from "./rounds.js";
import { action, card, sendAll, reviewAndConfirm, storedTopic } from "./screen.js";

const banner = (page) => page.getByRole("banner");
const notice = (page) => page.locator("[data-wait-footer]");
const agentStatus = (page) => banner(page).locator("[data-agent-status]");
const guidance = (page) => notice(page).locator('[data-wait-guidance]');

for (const lang of ['en', 'ja']) {
  for (const reviewing of [false, true]) {
    test(`current_result_heading_describes_unsent_sent_and_ended_states_${lang}_${reviewing ? 'review' : 'proceed'}`, async ({ shoryo, page }) => {
      await twoRounds(shoryo);
      await shoryo.submit();
      await shoryo.round({ subject: 'Result', questions: [], finished_picture: 'a = Screen\n| a |' });
      await page.goto(shoryo.url);
      await banner(page).locator(`[data-language=${lang}]`).click();
      const result = page.locator('[data-panel=current] [data-result]');
      const hint = result.locator('.result-head .list-hint');
      await expect(hint).toContainText(lang === 'ja' ? '「見直す」を押してください' : /ask for a review/i);
      if (reviewing) await reviewAndConfirm(page, result.locator('[data-decision-item=d1]'));
      await sendAll(page);
      await expect(hint).toContainText(lang === 'ja' ? '結果を送りました。完成図と決まったことは引き続き確認できます。' : /result.*sent/i);
      await expect(hint).not.toContainText(lang === 'ja' ? '押してください' : /ask for a review/i);
      if (lang === 'en') await expect(hint).toContainText(/can.*check.*finished picture.*decisions/i);
      await expect(result.locator('[data-node=a]')).toContainText('Screen');
      await expect(result.locator('[data-decision-item=d1]')).toContainText('Person reads cards');
      const reviewAction = action(result.locator('[data-decision-item=d1]'), reviewing ? 'stop-review' : 'review');
      if (reviewing) await expect(reviewAction).toBeEnabled();
      else await expect(reviewAction).toBeDisabled();
      await shoryo.end();
      await expect(hint).toContainText(lang === 'ja' ? 'この議題は終了しました。完成図と決まったことを確認できます。' : /topic.*ended/i);
      if (lang === 'en') await expect(hint).toContainText(/can.*check.*finished picture.*decisions/i);
      await expect(hint).not.toContainText(lang === 'ja' ? '押してください' : /ask for a review/i);
      await expect(result.locator('[data-node=a]')).toContainText('Screen');
      await expect(result.locator('[data-decision-item=d1]')).toContainText('Person reads cards');
      await expect(action(result, 'review')).toHaveCount(0);
    });
  }
}

test("accepted_answers_replace_current_notice_and_send_button_with_global_next_round_wait", async ({ shoryo, page }) => {
  await twoRounds(shoryo);
  await page.goto(shoryo.url);

  await expect(notice(page)).not.toBeVisible();
  await sendAll(page);
  await expect(notice(page)).toBeVisible();
  await expect(notice(page)).toHaveAttribute("data-sent-kind", "answers");
  await expect(notice(page)).toHaveAttribute("data-wait-target", "next_round");
  await expect(page.locator("[data-panel=current] [data-sent-notice]")).toHaveCount(0);
  await expect(action(page, "send")).not.toBeVisible();
  for (const tab of ["past", "map", "decisions", "current"]) {
    await page.locator(`[data-tab=${tab}]`).click();
    await expect(notice(page)).toBeInViewport();
    if (tab === "past") {
      await action(page.locator('[data-round-choice="1"]'), "choose-round").click();
      await expect(notice(page).locator("[data-wait-round]")).toContainText("2");
    }
  }
});

test("after_proceeding_with_a_result_the_notice_does_not_promise_a_next_round", async ({ shoryo, page }) => {
  await twoRounds(shoryo);
  await shoryo.submit();
  await shoryo.round({ subject: "Result", questions: [], finished_picture: "a = Screen\n| a |" });
  await page.goto(shoryo.url);

  await action(page, "send").click();
  await page.locator("[data-confirm-send] [data-action=confirm-send]").click();

  await expect(notice(page)).toBeVisible();
  await expect(notice(page)).toHaveAttribute("data-sent-kind", "proceeded");
  await expect(notice(page)).toHaveAttribute("data-wait-target", "end");
  await expect(action(page, "send")).not.toBeVisible();
  await shoryo.end();
  await expect(notice(page)).not.toBeVisible();
  await expect(agentStatus(page)).toHaveCount(0);
});

test("header_only_shows_confirmed_operation_wait_until_an_event_is_delivered", async ({ shoryo, page }) => {
  await twoRounds(shoryo);
  await page.goto(shoryo.url);
  await expect(agentStatus(page)).toHaveCount(0);
  await banner(page).locator('[data-language="ja"]').click();
  const earlier = await shoryo.wait();

  const waiting = shoryo.waitAfter(
    earlier.map((event) => event.id),
    20,
  );
  await expect(agentStatus(page)).toHaveAttribute("data-agent-status", "waiting");
  await expect(agentStatus(page)).toHaveText("操作待ち");
  expect(await agentStatus(page).evaluate(el => getComputedStyle(el, "::before").animationName)).toBe("none");
  await card(page, "q2").locator("[data-field=ask]").fill("Why a file?");
  await action(card(page, "q2"), "ask").click();

  expect((await waiting).map((event) => event.kind)).toEqual(["ask"]);
  await expect(agentStatus(page)).toHaveCount(0);
});

test("review_request_wait_survives_stopping_review_and_history_keeps_how_the_result_was_sent", async ({ shoryo, page }) => {
  await twoRounds(shoryo);
  await shoryo.submit();
  await shoryo.round({ subject: "Result", questions: [] });
  await page.goto(shoryo.url);
  await reviewAndConfirm(page, page.locator('[data-panel=current] [data-decision-item="d1"]'));
  await sendAll(page);
  await expect(notice(page)).toHaveAttribute("data-sent-kind", "review_requested");
  await expect(notice(page)).toHaveAttribute("data-wait-target", "next_round");
  await page.locator('[data-tab=decisions]').click();
  await action(page.locator('[data-panel=decisions] [data-decision-item="d1"]'), "stop-review").click();
  await expect(notice(page)).toHaveAttribute("data-sent-kind", "review_requested");
  await page.locator('[data-tab=past]').click();
  await expect(page.locator('[data-panel=past] [data-sent-as]')).toHaveAttribute("data-sent-as", "review_requested");
  await shoryo.round({ subject: "Revisit", questions: [question("q5", "Anything else?")] });
  await expect(notice(page)).not.toBeVisible();
  await page.locator('[data-tab=current]').click();
  await expect(action(page, "send")).toBeVisible();
});

test("delayed_reply_and_agent_wait_do_not_clear_the_latest_submission_wait", async ({ shoryo, page }) => {
  await twoRounds(shoryo);
  await page.goto(shoryo.url);
  await card(page, "q2").locator('[data-field=ask]').fill("Explain before I send");
  await action(card(page, "q2"), "ask").click();
  const ask = (await shoryo.wait()).find(event => event.kind === "ask");
  await sendAll(page);
  await expect(notice(page)).toBeVisible();
  await shoryo.reply(ask.ask, { text: "A delayed reply" });
  await expect(card(page.locator('[data-panel=current]'), "q2").getByText("A delayed reply")).toBeVisible();
  await expect(notice(page)).toHaveAttribute("data-wait-target", "next_round");
  const events = await shoryo.wait();
  const waiting = shoryo.waitAfter(events.map(event => event.id), 1);
  await expect(agentStatus(page)).toHaveAttribute("data-agent-status", "waiting");
  await expect(notice(page)).toBeVisible();
  await waiting;
  await expect(agentStatus(page)).toHaveCount(0);
  await expect(notice(page)).toBeVisible();
  await shoryo.round({ subject: "Next", questions: [question("q5", "Continue?")] });
  await expect(notice(page)).not.toBeVisible();
});

test("new_round_clears_end_wait_after_proceeding", async ({ shoryo, page }) => {
  await twoRounds(shoryo);
  await shoryo.submit();
  await shoryo.round({ subject: "Result", questions: [] });
  await page.goto(shoryo.url);
  await sendAll(page);
  await expect(notice(page)).toHaveAttribute("data-wait-target", "end");
  await shoryo.round({ subject: "Reopened", questions: [question("q5", "One more question?")] });
  await expect(notice(page)).not.toBeVisible();
  await expect(card(page, "q5")).toBeVisible();
});

test("confirmation_is_not_submission_and_footer_is_readable_at_the_viewport_bottom_in_both_themes", async ({ shoryo, page }) => {
  await twoRounds(shoryo);
  await page.goto(shoryo.url);
  for (const id of ["q2", "q4"]) await action(card(page, id), "stamp").click();
  await action(page, "send").click();
  await expect(page.locator('[data-confirm-send]')).toBeVisible();
  await expect(notice(page)).not.toBeVisible();
  await action(page.locator('[data-confirm-send]'), "confirm-send").click();
  await expect(notice(page)).toBeVisible();
  await page.setViewportSize({ width: 360, height: 640 });
  for (const lang of ["en", "ja"]) {
    await banner(page).locator(`[data-language=${lang}]`).click();
    for (const theme of ["light", "dark"]) {
      while (await action(banner(page), "theme").getAttribute("data-theme-value") !== theme) await action(banner(page), "theme").click();
      await page.evaluate(() => window.scrollTo(0, document.documentElement.scrollHeight));
      const footer = await notice(page).boundingBox();
      expect(footer.y + footer.height).toBeCloseTo(640, 0);
      const text = notice(page).locator('[data-wait-text]');
      await expect(text).toBeInViewport({ ratio: 0.99 });
      expect(await text.evaluate(el => el.scrollWidth <= el.clientWidth && el.scrollHeight <= el.clientHeight)).toBe(true);
      const dots = notice(page).locator('[data-wait-dot]');
      await expect(dots).toHaveCount(3);
      expect((await dots.first().boundingBox()).x).toBeGreaterThan((await text.boundingBox()).x + (await text.boundingBox()).width);
      await expect(action(card(page.locator('[data-panel=current]'), "q4"), "stamp")).toBeInViewport({ ratio: 0.99 });
    }
  }
});

test("saved_submission_adds_non_alarm_guidance_at_exactly_three_minutes_without_redrawing_other_regions", async ({ shoryo, page }) => {
  await twoRounds(shoryo);
  await page.goto(shoryo.url);
  await sendAll(page);
  await expect(notice(page)).toBeVisible();
  const sentAt = Date.parse((await storedTopic(shoryo)).rounds.at(-1).sent_at);
  await page.clock.install({ time: sentAt - 1000 });
  await page.clock.pauseAt(sentAt);
  await page.reload();
  await expect(notice(page)).toBeVisible();
  await page.clock.runFor(179999);
  await expect(guidance(page)).toHaveCount(0);
  const theme = action(banner(page), "theme");
  await theme.focus();
  await banner(page).locator('details summary').click();
  await theme.focus();
  await page.evaluate(() => {
    window.waitUnchangedRegions = {
      header: document.querySelector('header.topbar'),
      card: document.querySelector('[data-panel=current] [data-card=q2]'),
      mutations: 0,
    };
    const observer = new MutationObserver(records => window.waitUnchangedRegions.mutations += records.length);
    observer.observe(document.querySelector('main'), { subtree: true, attributes: true, childList: true, characterData: true });
  });
  await page.clock.runFor(1);
  await expect(guidance(page)).toBeVisible();
  await expect(notice(page)).toHaveAttribute('data-wait-target', 'next_round');
  await expect(page.getByRole('alert')).not.toBeVisible();
  await expect(theme).toBeFocused();
  await expect(banner(page).locator('details')).toHaveJSProperty('open', true);
  expect(await page.evaluate(() => {
    const previous = window.waitUnchangedRegions;
    return previous.mutations === 0 && previous.header === document.querySelector('header.topbar') && previous.card === document.querySelector('[data-panel=current] [data-card=q2]');
  })).toBe(true);
});

test("late_reply_wait_and_stopping_review_do_not_restart_the_saved_submission_age", async ({ shoryo, page }) => {
  await twoRounds(shoryo);
  await page.goto(shoryo.url);
  await shoryo.op({ op: 'ask', question: 'q2', text: 'Reply later' });
  const ask = (await shoryo.wait()).find(event => event.kind === 'ask');
  await page.locator('[data-tab=decisions]').click();
  await reviewAndConfirm(page, page.locator('[data-panel=decisions] [data-decision-item=d1]'));
  await page.locator('[data-tab=current]').click();
  await sendAll(page);
  await expect(notice(page)).toBeVisible();
  const savedSentAt = (await storedTopic(shoryo)).rounds.at(-1).sent_at;
  const sentAt = Date.parse(savedSentAt);
  await page.clock.install({ time: sentAt - 1000 });
  await page.clock.pauseAt(sentAt);
  await page.reload();
  await page.clock.runFor(120000);
  await shoryo.reply(ask.ask, { text: 'Reply during output wait' });
  await expect(card(page.locator('[data-panel=current]'), 'q2').getByText('Reply during output wait')).toBeVisible();
  await page.locator('[data-tab=decisions]').click();
  await action(page.locator('[data-panel=decisions] [data-decision-item=d1]'), 'stop-review').click();
  const events = await shoryo.wait();
  const waiting = shoryo.waitAfter(events.map(event => event.id), 1);
  await expect(agentStatus(page)).toHaveAttribute('data-agent-status', 'waiting');
  await waiting;
  await expect(agentStatus(page)).toHaveCount(0);
  await page.clock.runFor(59999);
  await expect(guidance(page)).toHaveCount(0);
  await page.clock.runFor(1);
  await expect(guidance(page)).toBeVisible();
  expect((await storedTopic(shoryo)).rounds.at(-1).sent_at).toBe(savedSentAt);
});

test("reloading_and_restarting_restore_wait_including_closed_time_but_not_an_old_agent_wait", async ({ shoryo, page }) => {
  await twoRounds(shoryo);
  await page.goto(shoryo.url);
  await sendAll(page);
  await expect(notice(page)).toBeVisible();
  const sentAt = Date.parse((await storedTopic(shoryo)).rounds.at(-1).sent_at);
  await page.clock.install({ time: sentAt + 179000 });
  await page.clock.pauseAt(sentAt + 180000);
  await page.reload();
  await expect(guidance(page)).toBeVisible();
  const events = await shoryo.wait();
  const waiting = shoryo.waitAfter(events.map(event => event.id), 20).catch(() => []);
  await expect(agentStatus(page)).toHaveAttribute('data-agent-status', 'waiting');
  await page.goto('about:blank');
  await shoryo.restart();
  await waiting;
  await page.goto(shoryo.url);
  await expect(notice(page)).toHaveAttribute('data-wait-target', 'next_round');
  await expect(guidance(page)).toBeVisible();
  await expect(agentStatus(page)).toHaveCount(0);
  await shoryo.end();
  await expect(notice(page)).not.toBeVisible();
});

test("legacy_sent_round_without_a_timestamp_waits_without_elapsed_guidance_or_record_repair", async ({ shoryo, page }) => {
  await twoRounds(shoryo);
  await shoryo.submit();
  await shoryo.restartWithoutSentTimestamp();
  await page.clock.install();
  await page.goto(shoryo.url);
  await expect(notice(page)).toHaveAttribute('data-wait-target', 'next_round');
  await page.clock.runFor(600000);
  await expect(guidance(page)).toHaveCount(0);
  expect((await storedTopic(shoryo)).rounds.at(-1).sent_at ?? null).toBeNull();
});

test("end_wait_guidance_preserves_the_open_dialog_focus_and_footer_stays_readable_on_a_narrow_screen", async ({ shoryo, page }) => {
  await twoRounds(shoryo);
  await shoryo.submit();
  await shoryo.round({ subject: 'Result', questions: [], finished_picture: 'a = Screen\n| a |' });
  await page.goto(shoryo.url);
  await sendAll(page);
  await expect(notice(page)).toHaveAttribute('data-wait-target', 'end');
  const sentAt = Date.parse((await storedTopic(shoryo)).rounds.at(-1).sent_at);
  await page.clock.install({ time: sentAt - 1000 });
  await page.clock.pauseAt(sentAt);
  await page.reload();
  await page.clock.runFor(179999);
  await action(page, 'finished-picture').click();
  const focused = await page.evaluateHandle(() => document.activeElement);
  await page.clock.runFor(1);
  await expect(guidance(page)).toBeVisible();
  expect(await page.evaluate(el => document.activeElement === el, focused)).toBe(true);
  await expect(notice(page)).toHaveJSProperty('inert', true);
  await page.keyboard.press('Escape');
  await expect(notice(page)).toHaveJSProperty('inert', false);
  await page.setViewportSize({ width: 360, height: 640 });
  for (const lang of ['en', 'ja']) {
    await banner(page).locator(`[data-language=${lang}]`).click();
    await expect(guidance(page)).toBeInViewport({ ratio: 0.99 });
    expect(await notice(page).locator('[data-wait-text]').evaluate(el => el.scrollWidth <= el.clientWidth)).toBe(true);
  }
  await shoryo.end();
  await expect(notice(page)).not.toBeVisible();
});

test("footer_can_draw_accepted_submission_view_data_without_access_to_application_state", async ({ shoryo, page }) => {
  await twoRounds(shoryo);
  await shoryo.submit();
  await page.goto(shoryo.url);
  const view = await (await fetch(`${shoryo.url}api/view`)).json();
  const sentAt = Date.parse(view.topic.rounds.at(-1).sent_at);
  const drawn = await page.evaluate(async ({ view, sentAt }) => {
    const { waitFooterData } = await import('./view-data.js');
    const { WaitFooter } = await import('./components/wait-footer.js');
    const projected = waitFooterData({ ...view, lang: 'ja' }, 'past', sentAt + 180000);
    const { changesIn, ...data } = projected;
    const region = new WaitFooter(() => { throw new Error('Waiting must not emit actions'); });
    region.update(data);
    return {
      sent: region.el.dataset.sentKind,
      target: region.el.dataset.waitTarget,
      text: region.el.textContent,
      round: region.el.querySelector('[data-wait-round]').textContent,
      guidance: region.el.querySelector('[data-wait-guidance]') !== null,
      dots: region.el.querySelectorAll('[data-wait-dot]').length,
    };
  }, { view, sentAt });
  expect(drawn.sent).toBe('answers');
  expect(drawn.target).toBe('next_round');
  expect(drawn.text).toContain('次のラウンド');
  expect(drawn.round).toContain('2');
  expect(drawn.guidance).toBe(true);
  expect(drawn.dots).toBe(3);
});

test("header_does_not_infer_activity_from_silence", async ({ shoryo, page }) => {
  await page.clock.install();
  await twoRounds(shoryo);
  await page.goto(shoryo.url);
  await expect(agentStatus(page)).toHaveCount(0);

  await page.clock.fastForward("09:30");
  await expect(agentStatus(page)).toHaveCount(0);
  await page.clock.fastForward("01:00");

  await expect(agentStatus(page)).toHaveCount(0);
});

test("a_change_of_the_llm_status_leaves_the_opened_request_open_and_the_focus_in_place", async ({ shoryo, page }) => {
  await twoRounds(shoryo);
  await page.goto(shoryo.url);
  await expect(agentStatus(page)).toHaveCount(0);
  const earlier = await shoryo.wait();
  const request = banner(page).locator("details");
  await request.locator("summary").click();
  const theme = action(banner(page), "theme");
  await theme.focus();

  const waiting = shoryo.waitAfter(
    earlier.map((event) => event.id),
    1,
  );
  await expect(agentStatus(page)).toHaveAttribute("data-agent-status", "waiting");

  await expect(request).toHaveJSProperty("open", true);
  await expect(theme).toBeFocused();
  await waiting;
  await expect(agentStatus(page)).toHaveCount(0);
});

test("after_proceeding_the_header_remains_hidden_even_after_ten_minutes", async ({ shoryo, page }) => {
  await page.clock.install();
  await twoRounds(shoryo);
  await shoryo.submit();
  await shoryo.round({ subject: "Result", questions: [], finished_picture: "a = Screen\n| a |" });
  await page.goto(shoryo.url);
  await action(page, "send").click();
  await page.locator("[data-confirm-send] [data-action=confirm-send]").click();
  await expect(notice(page)).toBeVisible();

  await page.clock.fastForward("10:30");

  await expect(agentStatus(page)).toHaveCount(0);
});
