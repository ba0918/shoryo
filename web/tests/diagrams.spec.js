// Diagrams in replies and the finished picture are never clipped: labels stay inside their
// boxes, a reply's diagram fits the reply, and the finished picture fits its view
// (docs/spec/screen.md, "図", "完成図").
import { test, expect } from "./fixtures.js";
import { roundOne, roundTwo } from "./rounds.js";
import { action, card } from "./screen.js";

const longLabel = "利用者が画面で問いを読み、聞き返し、答えをまとめて送るまでの流れを、記録に残しながら進める仕組みの全体";

const widePicture = [
  "a = 画面",
  "b = サーバー",
  "c = 記録のファイル",
  "d = 地図",
  "e = 完成図",
  "f = ? まだ決まっていない置き場所",
  "| a | b | c | d | e | f |",
  "a -> b : 答えを送る",
  "b -> c : 残す",
].join("\n");

test.beforeEach(async ({ shoryo, page }) => {
  await shoryo.round({ ...roundOne, finished_picture: widePicture });
  await shoryo.submit();
  await shoryo.round(roundTwo);
  await page.goto(shoryo.url);
});

async function replyWith(shoryo, page, diagram) {
  const q2 = card(page, "q2");
  await q2.locator("[data-field=ask]").fill("Show me");
  await action(q2, "ask").click();
  let ask;
  await expect
    .poll(async () => {
      ask = (await shoryo.wait()).find((event) => event.kind === "ask");
      return ask !== undefined;
    })
    .toBe(true);
  await shoryo.reply(ask.ask, { text: "Here is the picture.", diagram });
  const svg = q2.locator("[data-reply] [data-diagram]");
  await expect(svg).toBeVisible();
  return svg;
}

const inside = (inner, outer) =>
  inner.x >= outer.x - 0.5 &&
  inner.y >= outer.y - 0.5 &&
  inner.x + inner.width <= outer.x + outer.width + 0.5 &&
  inner.y + inner.height <= outer.y + outer.height + 0.5;

test("long_japanese_diagram_label_stays_inside_its_box", async ({ shoryo, page }) => {
  const svg = await replyWith(shoryo, page, `a = ${longLabel}\nb = 短い\n| a | b |\na -> b : つなぐ`);

  const node = svg.locator('[data-node="a"]');
  const [text, box] = await node.evaluate((group) => {
    const rect = group.querySelector("rect").getBBox();
    const shown = [...group.querySelectorAll("text")].find((t) => getComputedStyle(t).display !== "none").getBBox();
    return [
      { x: shown.x, y: shown.y, width: shown.width, height: shown.height },
      { x: rect.x, y: rect.y, width: rect.width, height: rect.height },
    ];
  });
  expect(inside(text, box)).toBe(true);
  await expect(node.locator("text").first()).toContainText("…");
  await expect(node.locator("title")).toHaveText(longLabel);
});

test("tapping_a_cut_label_shows_it_whole_inside_the_reply_diagram", async ({ shoryo, page }) => {
  const veryLong = `${longLabel}。${longLabel}`;
  const svg = await replyWith(shoryo, page, `a = ${veryLong}\nb = 短い\n| a | b |`);
  const node = svg.locator('[data-node="a"]');
  const before = await node.locator("rect").boundingBox();

  await node.click();

  const frame = await svg.boundingBox();
  const grown = await node.locator("rect").boundingBox();
  const shown = node.locator("text").filter({ visible: true });
  await expect(shown).not.toContainText("…");
  const whole = await shown.boundingBox();
  expect(grown.height).toBeGreaterThan(before.height);
  expect(inside(grown, frame)).toBe(true);
  expect(inside(whole, frame)).toBe(true);
  expect(inside(whole, grown)).toBe(true);
});

async function keyboardTo(page, control) {
  await expect(control).toBeAttached();
  for (let i = 0; i < 120; i++) {
    if (await control.evaluate(element => element === document.activeElement)) return;
    await page.keyboard.press("Tab");
  }
  throw new Error("Keyboard cannot reach the diagram label control");
}

const keyboardLabel = "利用者が画面で問いを読み、質問し、答えをまとめて送るまでの流れを、記録に残しながら進める仕組みの全体。".repeat(2);

for (const surface of ["reply", "result", "dialog"]) {
  test(`keyboard_expands_and_folds_a_long_label_in_the_${surface}_diagram`, async ({ shoryo, page }) => {
    const diagram = `a = ${keyboardLabel}\nb = 記録\n| a | b |`;
    let scope;
    if (surface === "reply") {
      await shoryo.op({ op: "ask", question: "q2", text: "Explain the full workflow" });
      const ask = (await shoryo.wait()).find(event => event.kind === "ask");
      await shoryo.reply(ask.ask, { text: "Workflow diagram", diagram });
      scope = card(page, "q2").locator("[data-reply]");
    } else {
      await shoryo.submit();
      await shoryo.round({ subject: "Result", questions: [], finished_picture: diagram });
      if (surface === "dialog") {
        await keyboardTo(page, action(page, "finished-picture"));
        await page.keyboard.press("Enter");
        scope = page.locator("[data-finished-picture]");
      } else {
        scope = page.locator("[data-panel=current] [data-result]");
      }
    }
    const control = scope.getByRole("button", { name: keyboardLabel, exact: true });
    await keyboardTo(page, control);
    const shown = control.locator("text").filter({ visible: true });
    await expect(shown).not.toHaveText(keyboardLabel);
    for (const key of ["Enter", "Space"]) {
      await page.keyboard.press(key);
      await expect(shown).toHaveText(keyboardLabel);
      await expect(control).toBeFocused();
      await page.keyboard.press(key);
      await expect(shown).not.toHaveText(keyboardLabel);
      await expect(control).toBeFocused();
    }
  });
}

test("reply_redraw_keeps_focus_on_the_same_node_in_the_same_exchange", async ({ shoryo, page }) => {
  const diagram = `a = ${keyboardLabel}\nb = 記録\n| a | b |`;
  for (const text of ["Explain first", "Explain second"]) await shoryo.op({ op: "ask", question: "q2", text });
  const asks = (await shoryo.wait()).filter(event => event.kind === "ask");
  for (const ask of asks) await shoryo.reply(ask.ask, { text: "Diagram reply", diagram });
  const second = card(page, "q2").locator(`[data-ask="${asks[1].ask}"]`);
  const control = second.getByRole("button", { name: keyboardLabel, exact: true });
  await keyboardTo(page, control);
  await shoryo.op({ op: "ask", question: "q2", text: "Explain third" });
  await expect(card(page, "q2").getByText("Explain third", { exact: true })).toBeVisible();
  await expect(control).toBeFocused();
});

test("result_record_update_keeps_focus_on_the_retained_picture_node", async ({ shoryo, page }) => {
  await shoryo.submit();
  await shoryo.round({ subject: "Result", questions: [], finished_picture: `a = ${keyboardLabel}\n| a |` });
  const result = page.locator('[data-panel=current] [data-result]');
  const control = result.getByRole("button", { name: keyboardLabel, exact: true });
  await keyboardTo(page, control);
  await shoryo.op({ op: "request_review", decision: "d1" });
  await expect(action(result.locator('[data-decision-item=d1]'), "stop-review")).toBeVisible();
  await expect(control).toBeFocused();
});

test("reply_diagram_is_sized_to_its_content", async ({ shoryo, page }) => {
  await page.setViewportSize({ width: 390, height: 800 });
  const svg = await replyWith(shoryo, page, "a = File\nb = Disk\nc = Backup\n| a | b | c |\na -> b : lives on\nb -> c : copied to");

  const reply = await card(page, "q2").locator("[data-reply]").boundingBox();
  const drawn = await svg.boundingBox();
  expect(drawn.x).toBeGreaterThanOrEqual(reply.x - 0.5);
  expect(drawn.x + drawn.width).toBeLessThanOrEqual(reply.x + reply.width + 0.5);
  const frame = await svg.evaluate((element) => {
    const content = element.getBBox();
    const view = element.viewBox.baseVal;
    return { spareWidth: view.width - content.width, spareHeight: view.height - content.height };
  });
  expect(frame.spareWidth).toBeLessThan(60);
  expect(frame.spareHeight).toBeLessThan(60);
});

test("finished_picture_fits_the_view_and_can_be_zoomed", async ({ page }) => {
  await action(page, "finished-picture").click();
  const view = page.locator("[data-finished-picture] [data-diagram-view]");
  await expect(view).toBeVisible();

  const area = await view.boundingBox();
  for (const id of ["a", "b", "c", "d", "e", "f"]) {
    const node = await view.locator(`[data-node="${id}"] rect`).boundingBox();
    expect(inside(node, area)).toBe(true);
  }

  const before = await view.locator('[data-node="a"] rect').boundingBox();
  await action(page.locator("[data-finished-picture]"), "diagram-zoom-in").click();
  const after = await view.locator('[data-node="a"] rect').boundingBox();
  expect(after.width).toBeGreaterThan(before.width * 1.1);
});

test("single_row_finished_pictures_have_two_rows_of_space_without_enlarging_the_drawing", async ({ shoryo, page }) => {
  await page.setViewportSize({ width: 1920, height: 1000 });
  const reference = await replyWith(shoryo, page, "a = File\nb = Disk\n| a |\n| b |");
  const top = await reference.locator('[data-node=a] rect').boundingBox();
  const bottom = await reference.locator('[data-node=b] rect').boundingBox();
  const twoRows = 2 * (bottom.y - top.y);
  await shoryo.submit();
  await shoryo.round({ subject: "Result", questions: [], finished_picture: "a = File\n| a |" });
  const result = page.locator('[data-panel=current] [data-result]');
  const checkPicture = async scope => {
    const area = await scope.locator('[data-diagram-view]').boundingBox();
    const node = await scope.locator('[data-node=a] rect').boundingBox();
    expect(area.height).toBeGreaterThanOrEqual(twoRows - 1);
    expect(node.width).toBeLessThanOrEqual(top.width + 1);
    expect(inside(node, area)).toBe(true);
    await expect.poll(async () => scope.locator('[data-diagram-view]').evaluate(element => {
      const frame = element.getBoundingClientRect();
      const box = element.querySelector('[data-node=a] rect').getBoundingClientRect();
      return Math.max(
        Math.abs(box.x + box.width / 2 - (frame.x + frame.width / 2)),
        Math.abs(box.y + box.height / 2 - (frame.y + frame.height / 2)),
      );
    })).toBeLessThan(2);
  };
  await expect(result).toBeVisible();
  await checkPicture(result);
  const parent = await result.boundingBox();
  const viewer = await result.locator('.diagram-viewer').boundingBox();
  expect(viewer.width).toBeLessThanOrEqual(parent.width);
  expect(viewer.width).toBeLessThan(page.viewportSize().width * 0.95);
  expect(Math.abs(viewer.x + viewer.width / 2 - (parent.x + parent.width / 2))).toBeLessThan(2);

  await action(page, "finished-picture").click();
  const dialog = page.locator('[data-finished-picture]');
  await checkPicture(dialog);
  const box = await dialog.boundingBox();
  expect(box.width).toBeLessThan(page.viewportSize().width * 0.95);
  expect(Math.abs(box.x + box.width / 2 - page.viewportSize().width / 2)).toBeLessThan(2);
  await action(dialog, "close").click();

  await shoryo.op({ op: "request_review", decision: "d1" });
  await shoryo.submit();
  await shoryo.round({ subject: "Review", questions: [], finished_picture: "a = File\n| a |" });
  await page.locator('[data-tab=past]').click();
  await action(page.locator('[data-panel=past] [data-round-choice]').last(), "choose-round").click();
  const past = page.locator('[data-panel=past] [data-result]');
  await expect(past).toBeVisible();
  await checkPicture(past);
});

test("finished_picture_dialog_shrinks_in_short_and_narrow_windows_without_hiding_controls", async ({ shoryo, page }) => {
  await page.setViewportSize({ width: 1440, height: 1000 });
  await shoryo.submit();
  await shoryo.round({ subject: "Result", questions: [], finished_picture: "a = File\n| a |" });
  await action(page, "finished-picture").click();
  const dialog = page.locator('[data-finished-picture]');
  const picture = dialog.locator('[data-diagram-view]');
  const desktop = await picture.boundingBox();
  for (const viewport of [{ width: 1280, height: 300 }, { width: 390, height: 300 }]) {
    await page.setViewportSize(viewport);
    await expect.poll(async () => (await picture.boundingBox()).height).toBeLessThan(desktop.height);
    const window = { x: 0, y: 0, ...viewport };
    expect(inside(await dialog.boundingBox(), window)).toBe(true);
    expect(inside(await picture.boundingBox(), window)).toBe(true);
    for (const name of ["diagram-zoom-out", "diagram-fit", "diagram-zoom-in", "close"]) {
      expect(inside(await action(dialog, name).boundingBox(), window)).toBe(true);
    }
    await action(dialog, "diagram-zoom-in").click();
    await action(dialog, "diagram-fit").click();
  }
  await action(dialog, "close").click();
  await expect(dialog).toHaveCount(0);
});

test("wheel_and_ctrl_wheel_over_a_reply_diagram_leave_it_unchanged_and_not_prevented", async ({ shoryo, page }) => {
  const svg = await replyWith(shoryo, page, "a = File\nb = Disk\n| a | b |");
  const before = await svg.boundingBox();
  const node = svg.locator('[data-node="a"] rect');
  const nodeBefore = await node.boundingBox();
  for (const ctrlKey of [false, true]) {
    expect(await svg.evaluate((el, ctrlKey) => {
      const event = new WheelEvent("wheel", { deltaY: -100, ctrlKey, bubbles: true, cancelable: true });
      el.dispatchEvent(event);
      return event.defaultPrevented;
    }, ctrlKey)).toBe(false);
    expect(await svg.boundingBox()).toEqual(before);
    expect(await node.boundingBox()).toEqual(nodeBefore);
  }
});

test("plain_wheel_over_the_result_picture_leaves_its_zoom_unchanged", async ({ shoryo, page }) => {
  await shoryo.submit();
  await shoryo.round({ subject: "Result", questions: [], finished_picture: widePicture });
  const picture = page.locator("[data-panel=current] [data-diagram-view]");
  await expect(picture).toBeVisible();
  const before = await picture.locator('[data-node="a"] rect').boundingBox();
  expect(await picture.evaluate(el => {
    const event = new WheelEvent("wheel", { deltaY: 100, bubbles: true, cancelable: true });
    el.dispatchEvent(event);
    return event.defaultPrevented;
  })).toBe(false);
  expect((await picture.locator('[data-node="a"] rect').boundingBox()).width).toBe(before.width);
  await expect(picture.locator("[data-wheel-hint]")).toBeVisible();
});

test("ctrl_or_meta_wheel_zooms_the_result_picture_and_prevents_the_default", async ({ shoryo, page }) => {
  await shoryo.submit();
  await shoryo.round({ subject: "Result", questions: [], finished_picture: widePicture });
  const picture = page.locator("[data-panel=current] [data-diagram-view]");
  await expect(picture).toBeVisible();
  for (const modifier of ["ctrlKey", "metaKey"]) {
    const before = (await picture.locator('[data-node="a"] rect').boundingBox()).width;
    expect(await picture.evaluate((el, modifier) => {
      const event = new WheelEvent("wheel", { deltaY: -100, bubbles: true, cancelable: true, [modifier]: true });
      el.dispatchEvent(event);
      return event.defaultPrevented;
    }, modifier)).toBe(true);
    expect((await picture.locator('[data-node="a"] rect').boundingBox()).width).toBeGreaterThan(before);
  }
});
