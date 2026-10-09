import { test, expect } from './fixtures.js';
import { question } from './rounds.js';
import { action, card } from './screen.js';

const codeBody = '  first();  \n\tsecond();\n';
const codePart = { type: 'code', title: 'Sample', language: 'unknown-language', role: 'example', body: codeBody };

async function show(shoryo, page, parts) {
  const q = question('q1', 'Read this explanation?');
  q.background = parts;
  await shoryo.round({ subject: 'Reading', questions: [q] });
  await page.goto(shoryo.url);
  await page.addStyleTag({ content: ':root { font-size: 16px; }' });
  await action(card(page, 'q1'), 'open').click();
  return card(page, 'q1').locator('.context');
}

test('ui_polish_code_header_operation_keeps_metadata_and_focus', async ({ shoryo, page, context }) => {
  const scope = await show(shoryo, page, [codePart]);
  const code = scope.locator('[data-part=code]');
  const copy = code.getByRole('button', { name: 'Copy code', exact: true });
  await expect(code.getByText('unknown-language', { exact: true })).toBeVisible();
  await expect(code.getByText('Example', { exact: true })).toBeVisible();
  await expect(copy).toHaveAttribute('title', 'Copy code');
  await expect(copy.locator('svg[aria-hidden="true"]')).toHaveCount(1);
  const [buttonBox, titleBox, bodyBox] = await Promise.all([copy.boundingBox(), code.getByText('Sample', { exact: true }).boundingBox(), code.locator('pre').boundingBox()]);
  expect(buttonBox.y + buttonBox.height).toBeLessThanOrEqual(bodyBox.y);
  expect(buttonBox.x).toBeGreaterThan(titleBox.x + titleBox.width);
  await context.grantPermissions(['clipboard-read', 'clipboard-write']);
  await copy.focus();
  await expect(copy).toBeFocused();
  expect(await copy.evaluate(n => parseFloat(getComputedStyle(n).outlineWidth))).toBeGreaterThanOrEqual(3);
  await copy.press('Enter');
  await expect.poll(() => page.evaluate(() => navigator.clipboard.readText())).toBe(codeBody);
  await page.evaluate(() => Object.defineProperty(navigator, 'clipboard', { configurable: true, value: undefined }));
  await copy.press('Enter');
  await copy.press('Tab');
  const select = code.getByRole('button', { name: 'Select code' });
  await expect(select).toBeFocused();
  await select.press('Enter');
  expect(await page.evaluate(() => getSelection().getRangeAt(0).cloneContents().textContent)).toBe(codeBody);
});

test('ui_polish_code_bounds_and_selection_remain_readable_in_both_themes', async ({ shoryo, page }) => {
  const scope = await show(shoryo, page, [codePart, { ...codePart, title: 'Long', body: ('wide '.repeat(60) + '\n').repeat(80) }]);
  for (const theme of ['light', 'dark']) {
    await page.evaluate(theme => document.documentElement.dataset.theme = theme, theme);
    const block = scope.locator('[data-part=code]').first();
    const pre = block.locator('pre');
    const style = await block.evaluate(n => {
      const p = n.querySelector('pre'), metadata = n.querySelector('span');
      const b = getComputedStyle(n), s = getComputedStyle(p), m = getComputedStyle(metadata);
      let painted = metadata.parentElement;
      while (getComputedStyle(painted).backgroundColor === 'rgba(0, 0, 0, 0)') painted = painted.parentElement;
      return { border: parseFloat(b.borderTopWidth), background: s.backgroundColor, color: s.color, metadata: m.color, metadataBackground: getComputedStyle(painted).backgroundColor, font: s.fontSize, wrap: s.whiteSpace, height: p.clientHeight };
    });
    expect(style.border).toBeGreaterThan(0);
    expect(style.font).toBe('14px');
    expect(style.wrap).toBe('pre');
    expect(style.height).toBeLessThan(90);
    const contrast = (foreground, background) => {
      const luminance = color => color.match(/[\d.]+/g).slice(0, 3).map(v => Number(v) / 255).map(v => v <= .04045 ? v / 12.92 : ((v + .055) / 1.055) ** 2.4).reduce((sum, v, i) => sum + v * [.2126, .7152, .0722][i], 0);
      const a = luminance(foreground), b = luminance(background);
      return (Math.max(a, b) + .05) / (Math.min(a, b) + .05);
    };
    expect(contrast(style.color, style.background)).toBeGreaterThanOrEqual(4.5);
    expect(contrast(style.metadata, style.metadataBackground)).toBeGreaterThanOrEqual(4.5);
    const long = scope.locator('pre').nth(1);
    expect(await long.evaluate(n => n.clientHeight)).toBeLessThanOrEqual(320);
    expect(await long.evaluate(n => n.scrollWidth > n.clientWidth && n.scrollHeight > n.clientHeight)).toBe(true);
    await long.evaluate(n => { n.scrollTop = 80; n.scrollLeft = 80; });
    expect(await long.evaluate(n => n.scrollTop > 0 && n.scrollLeft > 0)).toBe(true);
    const bounds = await pre.boundingBox();
    await page.mouse.move(bounds.x + 20, bounds.y + 20);
    await page.mouse.down();
    await page.mouse.move(bounds.x + 110, bounds.y + 20);
    await page.mouse.up();
    expect(await page.evaluate(() => getSelection().toString().length)).toBeGreaterThan(0);
  }
  await page.setViewportSize({ width: 320, height: 400 });
  expect(await scope.locator('pre').nth(1).evaluate(n => n.clientHeight)).toBeLessThanOrEqual(200);
  await page.addStyleTag({ content: ':root { font-size: 24px; }' });
  expect(await scope.locator('pre').first().evaluate(n => getComputedStyle(n).fontSize)).toBe('21px');
});

test('ui_polish_code_feedback_waits_for_success_and_stays_with_its_part', async ({ shoryo, page }) => {
  const scope = await show(shoryo, page, [codePart, { ...codePart, body: 'Other' }]);
  await page.evaluate(() => Object.defineProperty(navigator, 'clipboard', { configurable: true, value: { writeText: () => new Promise(resolve => { window.finishCopy = resolve; }) } }));
  const first = scope.locator('[data-part=code]').first();
  const copy = first.getByRole('button', { name: 'Copy code', exact: true });
  await copy.focus();
  await copy.press('Enter');
  await expect(first.getByRole('status')).toBeEmpty();
  await expect(copy.locator('svg')).toHaveCount(1);
  await shoryo.op({ op: 'ask', question: 'q1', text: 'Update' });
  await expect(card(page, 'q1').locator('[data-ask]')).toHaveCount(1);
  await expect(copy).toBeFocused();
  await page.evaluate(() => window.finishCopy());
  await expect(first.getByRole('status')).not.toBeEmpty();
  await expect(copy).toBeFocused();
  await expect(scope.locator('[data-part=code]').nth(1).getByRole('status')).toBeEmpty();
});

const wideFlow = { type: 'flow', title: 'Wide path', role: 'example', nodes: [{ id: 'a', kind: 'process', label: 'First', position: { x: 150, y: 100 } }, { id: 'b', kind: 'process', label: 'Last', position: { x: 1500, y: 100 } }], edges: [], canvas: { width: 1700, height: 200 } };
const shortFlow = { ...wideFlow, title: 'Short path', nodes: [wideFlow.nodes[0]], canvas: { width: 300, height: 160 } };
const matrix = svg => svg.evaluate(n => { const m = n.getScreenCTM(); return { x: m.e, y: m.f, k: m.a }; });
const inline = scope => scope.getByRole('group', { name: 'Diagram viewport', exact: true });

test('ui_polish_inline_natural_scale_and_short_height_keep_all_content', async ({ shoryo, page }) => {
  const scope = await show(shoryo, page, [wideFlow, shortFlow]);
  const surfaces = inline(scope);
  await expect(surfaces).toHaveCount(2);
  for (const size of [{ width: 1280, height: 800 }, { width: 390, height: 400 }, { width: 320, height: 800 }]) {
    await page.setViewportSize(size);
    for (const [i, part] of [wideFlow, shortFlow].entries()) {
      const surface = surfaces.nth(i), svg = surface.locator('svg[role=img]');
      await expect(svg).toBeVisible();
      expect((await matrix(svg)).k).toBe(1);
      const bounds = await surface.boundingBox();
      expect(bounds.height).toBeLessThanOrEqual(Math.min(360, size.height / 2));
      expect(bounds.height).toBeLessThanOrEqual(part.canvas.height);
      expect(await surface.evaluate(n => getComputedStyle(n).overflow)).toBe('hidden');
      expect(await svg.getAttribute('width')).toBe(String(part.canvas.width));
    }
  }
  const enlarge = scope.getByRole('button', { name: 'Enlarge diagram' }).first();
  await expect(enlarge).toHaveAttribute('title', 'Enlarge diagram');
  await expect(enlarge.locator('svg[aria-hidden=true]')).toHaveCount(1);
  await expect(scope.locator('[data-node-label]')).toHaveText(['First', 'Last', 'First']);
  await expect(scope.getByText(/two.*pinch/i).first()).toBeVisible();
});

test('ui_polish_inline_plain_wheel_reads_page_and_modified_wheel_keeps_pointer_anchor', async ({ shoryo, page, context }) => {
  const scope = await show(shoryo, page, [wideFlow, ...Array.from({ length: 8 }, () => ({ ...codePart, body: 'line\n'.repeat(40) }))]);
  const surface = inline(scope), svg = surface.locator('svg[role=img]');
  await expect(svg).toBeVisible();
  await surface.scrollIntoViewIfNeeded();
  const before = await matrix(svg);
  const b = await surface.boundingBox(), x = b.x + 90, y = b.y + 80;
  await page.mouse.move(x, y);
  const scroll = await page.evaluate(() => scrollY);
  await page.mouse.wheel(0, 120);
  await expect.poll(() => page.evaluate(() => scrollY)).toBeGreaterThan(scroll);
  expect((await matrix(svg)).k).toBe(before.k);
  expect(await surface.evaluate(n => n.scrollTop + n.scrollLeft)).toBe(0);
  await surface.scrollIntoViewIfNeeded();
  const bb = await surface.boundingBox(), px = bb.x + 90, py = bb.y + 80;
  const anchor = await svg.evaluate((n, p) => new DOMPoint(p.x, p.y).matrixTransform(n.getScreenCTM().inverse()).toJSON(), { x: px, y: py });
  const cdp = await context.newCDPSession(page);
  await cdp.send('Input.dispatchMouseEvent', { type: 'mouseWheel', x: px, y: py, deltaX: 0, deltaY: -100, modifiers: 2 });
  await expect.poll(async () => (await matrix(svg)).k).toBeGreaterThan(before.k);
  const moved = await svg.evaluate((n, p) => new DOMPoint(p.x, p.y).matrixTransform(n.getScreenCTM()).toJSON(), anchor);
  expect(moved.x).toBeCloseTo(px, 0); expect(moved.y).toBeCloseTo(py, 0);
  await expect(scope.locator('pre').first()).toBeVisible();
  expect(await scope.locator('pre').first().evaluate(n => { const e = new WheelEvent('wheel', { ctrlKey: true, cancelable: true, bubbles: true }); n.dispatchEvent(e); return e.defaultPrevented; })).toBe(false);
});

test('ui_polish_inline_drag_and_keys_reach_content_without_selecting_labels', async ({ shoryo, page }) => {
  const scope = await show(shoryo, page, [wideFlow]);
  const surface = inline(scope), svg = surface.locator('svg[role=img]');
  await expect(svg).toBeVisible();
  const label = await svg.locator('[data-node-label]').first().boundingBox();
  const before = await matrix(svg);
  await page.mouse.move(label.x + 5, label.y + 5); await page.mouse.down();
  await page.mouse.move(label.x + 65, label.y + 25); await page.mouse.up();
  const after = await matrix(svg);
  expect(after.x - before.x).toBeCloseTo(60, 0); expect(after.k).toBe(before.k);
  expect(await page.evaluate(() => getSelection().toString())).toBe('');
  await surface.focus(); await surface.press('ArrowLeft');
  expect((await matrix(svg)).x).toBeCloseTo(after.x - 40, 0);
  await surface.press('+'); expect((await matrix(svg)).k).toBeCloseTo(1.2);
  await surface.press('-'); expect((await matrix(svg)).k).toBeCloseTo(1);
});

test.describe('inline trusted touch', () => {
  test.use({ hasTouch: true });
  test('ui_polish_inline_touch_routes_one_contact_to_page_and_two_to_diagram', async ({ shoryo, page, context }) => {
    const scope = await show(shoryo, page, [wideFlow, ...Array.from({ length: 8 }, () => codePart)]);
    const surface = inline(scope), svg = surface.locator('svg[role=img]');
    await expect(svg).toBeVisible(); await surface.scrollIntoViewIfNeeded();
    const b = await surface.boundingBox();
    const cdp = await context.newCDPSession(page);
    const send = (type, points) => cdp.send('Input.dispatchTouchEvent', { type, touchPoints: points });
    const point = (id, x, y) => ({ id, x, y });
    const before = await matrix(svg), scroll = await page.evaluate(() => scrollY);
    await send('touchStart', [point(1, b.x + 100, b.y + 140)]);
    await send('touchMove', [point(1, b.x + 100, b.y + 110)]);
    await send('touchEnd', []);
    await expect.poll(() => page.evaluate(() => scrollY)).toBeGreaterThan(scroll);
    expect((await matrix(svg)).k).toBe(before.k);
    await surface.scrollIntoViewIfNeeded();
    const bb = await surface.boundingBox(), start = await matrix(svg), nowScroll = await page.evaluate(() => scrollY);
    await send('touchStart', [point(1, bb.x + 80, bb.y + 80), point(2, bb.x + 160, bb.y + 80)]);
    await send('touchMove', [point(1, bb.x + 60, bb.y + 100), point(2, bb.x + 180, bb.y + 100)]);
    await send('touchEnd', []);
    expect((await matrix(svg)).k).toBeGreaterThan(start.k);
    expect(await page.evaluate(() => scrollY)).toBe(nowScroll);
  });
  test('ui_polish_inline_contact_transitions_and_cancel_leave_no_stale_gesture', async ({ shoryo, page, context }) => {
    const scope = await show(shoryo, page, [wideFlow, ...Array.from({ length: 8 }, () => codePart)]);
    const surface = inline(scope), svg = surface.locator('svg[role=img]');
    await expect(svg).toBeVisible(); await surface.scrollIntoViewIfNeeded();
    const b = await surface.boundingBox(), cdp = await context.newCDPSession(page);
    const send = (type, points) => cdp.send('Input.dispatchTouchEvent', { type, touchPoints: points });
    const p = (id, x, y) => ({ id, x: b.x + x, y: b.y + y });
    await send('touchStart', [p(1, 80, 100)]);
    await send('touchStart', [p(1, 80, 100), p(2, 160, 100)]);
    await send('touchMove', [p(1, 90, 100), p(2, 170, 100)]);
    const pan = await matrix(svg), scroll = await page.evaluate(() => scrollY);
    await send('touchEnd', [p(2, 170, 100)]);
    expect(await page.evaluate(() => scrollY)).toBe(scroll);
    await send('touchMove', [p(1, 90, 80)]);
    await expect.poll(() => page.evaluate(() => scrollY)).toBeGreaterThan(scroll);
    expect((await matrix(svg)).k).toBe(pan.k);
    await send('touchCancel', []);
    const cancelled = await matrix(svg);
    await surface.dispatchEvent('pointermove', { pointerId: 1, pointerType: 'touch', clientX: b.x + 200, clientY: b.y + 100 });
    expect(await matrix(svg)).toEqual(cancelled);
  });
});

test('ui_polish_inline_view_and_opener_survive_reply_and_font_revalidation', async ({ shoryo, page }) => {
  const scope = await show(shoryo, page, [shortFlow, shortFlow]);
  const surface = inline(scope).nth(1), svg = surface.locator('svg[role=img]');
  await expect(svg).toBeVisible(); await surface.focus(); await surface.press('ArrowLeft');
  const before = await matrix(svg);
  await shoryo.op({ op: 'ask', question: 'q1', text: 'Update' });
  await shoryo.reply(1, { parts: [{ type: 'text', body: 'Arrived' }] });
  await expect(card(page, 'q1').locator('[data-reply]')).toContainText('Arrived');
  await expect(surface).toBeFocused();
  expect(await matrix(svg)).toEqual(before);
  const opener = scope.getByRole('button', { name: 'Enlarge diagram' }).nth(1);
  await opener.focus();
  await page.addStyleTag({ content: '.explanation-diagram { font-size: 3rem; }' });
  await expect(scope.locator('[data-layout-failure]')).toHaveCount(2);
  await expect(opener).toBeHidden();
  await page.addStyleTag({ content: '.explanation-diagram { font-size: .875rem; }' });
  await expect(opener).toBeVisible();
  expect((await matrix(svg)).k).toBe(before.k);
});

async function expectWhole(surface) {
  const geometry = await surface.locator('svg[role=img]').evaluate(n => {
    const box = n.getBoundingClientRect(), area = n.closest('[tabindex="0"]').getBoundingClientRect();
    return { left: box.left - area.left, top: box.top - area.top, right: box.right - area.right, bottom: box.bottom - area.bottom, dx: (box.left + box.right - area.left - area.right) / 2, dy: (box.top + box.bottom - area.top - area.bottom) / 2, k: n.getScreenCTM().a };
  });
  expect(geometry.left).toBeGreaterThanOrEqual(-2); expect(geometry.top).toBeGreaterThanOrEqual(-2);
  expect(geometry.right).toBeLessThanOrEqual(2); expect(geometry.bottom).toBeLessThanOrEqual(2);
  expect(Math.abs(geometry.dx)).toBeLessThanOrEqual(2); expect(Math.abs(geometry.dy)).toBeLessThanOrEqual(2);
  return geometry;
}

test('ui_polish_dialog_opens_whole_and_centred_after_measurement', async ({ shoryo, page }) => {
  const huge = { ...shortFlow, canvas: { width: 8192, height: 8192 } };
  const scope = await show(shoryo, page, [huge]);
  const opener = scope.getByRole('button', { name: 'Enlarge diagram' });
  await opener.focus(); await opener.press('Enter');
  const surface = page.locator('[data-explanation-viewer]');
  await expect(surface).toBeFocused(); await expect(surface.locator('svg[role=img]')).toBeVisible();
  await expectWhole(surface);
  await page.setViewportSize({ width: 390, height: 220 });
  await expect.poll(async () => (await matrix(surface.locator('svg[role=img]'))).k).toBeLessThan(.01);
  await expectWhole(surface);
  const initial = (await matrix(surface.locator('svg[role=img]'))).k;
  await surface.press('+'); expect((await matrix(surface.locator('svg[role=img]'))).k).toBeCloseTo(initial * 1.2, 5);
  await surface.press('-'); expect((await matrix(surface.locator('svg[role=img]'))).k).toBeCloseTo(initial * 1.2, 5);
});

test('ui_polish_dialog_whole_and_zero_fit_without_moving_focus', async ({ shoryo, page }) => {
  const scope = await show(shoryo, page, [wideFlow]);
  const inlineSurface = inline(scope);
  await inlineSurface.focus(); await inlineSurface.press('ArrowLeft'); await inlineSurface.press('+');
  await inlineSurface.press('0'); await expect(inlineSurface).toBeFocused(); await expectWhole(inlineSurface);
  const inlineState = await matrix(inlineSurface.locator('svg[role=img]'));
  await scope.getByRole('button', { name: 'Enlarge diagram' }).click();
  const surface = page.locator('[data-explanation-viewer]');
  await expect(surface.locator('svg[role=img]')).toBeVisible();
  await surface.press('ArrowRight'); await surface.press('+'); await surface.press('0');
  await expectWhole(surface); await expect(surface).toBeFocused();
  await surface.press('ArrowDown');
  await page.getByRole('dialog').getByRole('button', { name: 'Whole', exact: true }).click();
  await expectWhole(surface);
  await page.keyboard.press('Escape');
  expect(await matrix(inlineSurface.locator('svg[role=img]'))).toEqual(inlineState);
});

test('ui_polish_dialog_menu_and_close_keep_keyboard_scope_and_opener', async ({ shoryo, page }) => {
  const scope = await show(shoryo, page, [wideFlow, wideFlow]);
  const opener = scope.getByRole('button', { name: 'Enlarge diagram' }).nth(1);
  await opener.focus(); await opener.press('Enter');
  const dialog = page.getByRole('dialog'), trigger = dialog.getByRole('button', { name: 'Move diagram', exact: true });
  await trigger.focus(); await trigger.press('Enter'); await expect(trigger).toHaveAttribute('aria-expanded', 'true');
  await trigger.press('Tab');
  const left = dialog.getByRole('button', { name: 'Move left', exact: true });
  await expect(left).toBeFocused();
  const before = await matrix(dialog.locator('svg[role=img]'));
  await left.press('Enter'); expect((await matrix(dialog.locator('svg[role=img]'))).x).toBeCloseTo(before.x - 40, 0);
  await left.press('Escape'); await expect(trigger).toBeFocused(); await expect(trigger).toHaveAttribute('aria-expanded', 'false');
  await expect(dialog).toBeVisible();
  const close = dialog.getByRole('button', { name: 'Close', exact: true });
  await expect(close).toHaveAttribute('title', 'Close');
  await close.focus(); await close.press('Space'); await expect(opener).toBeFocused();
});

test('ui_polish_dialog_remaining_viewport_keeps_controls_visible_in_short_windows', async ({ shoryo, page }) => {
  const scope = await show(shoryo, page, [wideFlow]);
  await scope.getByRole('button', { name: 'Enlarge diagram' }).click();
  for (const size of [{ width: 390, height: 400 }, { width: 320, height: 400 }]) {
    await page.setViewportSize(size);
    const dialog = page.getByRole('dialog');
    for (const name of ['Close', 'Zoom out', 'Whole', 'Zoom in', 'Move diagram']) {
      const button = dialog.getByRole('button', { name, exact: true }); await expect(button).toBeVisible();
      const b = await button.boundingBox(); expect(b.x).toBeGreaterThanOrEqual(0); expect(b.y).toBeGreaterThanOrEqual(0); expect(b.y + b.height).toBeLessThanOrEqual(size.height);
    }
    expect(await dialog.evaluate(n => n.scrollHeight <= n.clientHeight + 2)).toBe(true);
    await expectWhole(page.locator('[data-explanation-viewer]'));
    await expect(dialog.getByRole('button', { name: /Reset|100%|Undo/ })).toHaveCount(0);
  }
});

test('ui_polish_dialog_live_redraw_and_failure_keep_identity_and_source_readable', async ({ shoryo, page }) => {
  const scope = await show(shoryo, page, [shortFlow, shortFlow]);
  const opener = scope.getByRole('button', { name: 'Enlarge diagram' }).nth(1);
  await opener.click(); const surface = page.locator('[data-explanation-viewer]');
  await expect(page.getByRole('dialog').getByText('Example', { exact: true })).toBeVisible();
  await expect(surface.locator('svg[role=img]')).toBeVisible(); await surface.press('ArrowLeft');
  const before = await matrix(surface.locator('svg[role=img]'));
  await shoryo.op({ op: 'ask', question: 'q1', text: 'Update' }); await shoryo.reply(1, { parts: [{ type: 'text', body: 'Arrived' }] });
  await expect(card(page, 'q1').locator('[data-reply]')).toContainText('Arrived'); await expect(surface).toBeFocused();
  expect(await matrix(surface.locator('svg[role=img]'))).toEqual(before);
  await page.addStyleTag({ content: '.explanation-diagram { font-size: 3rem; }' });
  const source = page.getByRole('dialog').locator('pre'); await expect(source).toContainText('First');
  expect(await source.evaluate(n => getComputedStyle(n).userSelect)).not.toBe('none');
  await page.addStyleTag({ content: '.explanation-diagram { font-size: .875rem; }' });
  await expect(surface.locator('svg[role=img]')).toBeVisible();
  expect((await matrix(surface.locator('svg[role=img]'))).k).toBe(before.k);
  await page.keyboard.press('Escape'); await expect(opener).toBeFocused();
});

test('ui_polish_dialog_positive_tiny_and_hidden_surfaces_defer_fit_until_measured', async ({ shoryo, page }) => {
  const huge = { ...shortFlow, canvas: { width: 8192, height: 8192 } };
  await show(shoryo, page, [huge]);
  await page.evaluate(async part => {
    const { ExplanationViewer } = await import('./components/explanation-viewer.js');
    const view = new ExplanationViewer().update({ part, identity: 'component-example', lang: 'en' });
    const holder = document.createElement('div'); holder.id = 'component-example';
    holder.append(view.el); document.body.append(holder);
    const surface = view.el.querySelector('[tabindex="0"]');
    surface.style.cssText = 'flex:none;width:0px;height:0px';
  }, huge);
  const holder = page.locator('#component-example'), surface = holder.getByRole('group', { name: 'Diagram viewer', includeHidden: true, exact: true });
  const svg = surface.locator('svg[role=img]');
  await expect(svg).toHaveCount(1);
  expect(await svg.evaluate(n => new DOMMatrix(getComputedStyle(n).transform).a)).toBe(1);
  await surface.evaluate(n => { n.style.width = '10px'; n.style.height = '10px'; });
  await expect.poll(async () => (await matrix(svg)).k).toBeLessThan(.01);
  expect((await surface.boundingBox()).width).toBe(10);
  expect((await surface.boundingBox()).height).toBe(10);
  await expectWhole(surface);
  const before = await svg.evaluate(n => getComputedStyle(n).transform);
  await surface.evaluate(n => { n.style.width = '0px'; n.style.height = '0px'; });
  await holder.getByRole('button', { name: 'Whole', exact: true, includeHidden: true }).evaluate(n => n.click());
  expect(await svg.evaluate(n => getComputedStyle(n).transform)).toBe(before);
  await holder.evaluate(n => { n.hidden = true; n.querySelector('[tabindex="0"]').style.height = '20px'; n.querySelector('[tabindex="0"]').style.width = '20px'; });
  await holder.evaluate(n => { n.hidden = false; });
  await expect.poll(async () => (await matrix(svg)).k).toBeGreaterThan(Number(before.match(/matrix\(([^,]+)/)[1]));
  await expectWhole(surface);
});

test('ui_polish_dialog_inline_whole_fits_large_canvas_and_move_closes_on_outside_activation', async ({ shoryo, page }) => {
  const huge = { ...shortFlow, canvas: { width: 8192, height: 8192 } };
  const scope = await show(shoryo, page, [huge, codePart]);
  await page.setViewportSize({ width: 390, height: 128 });
  const surface = inline(scope), svg = surface.locator('svg[role=img]');
  await expect(svg).toBeVisible(); expect((await matrix(svg)).k).toBe(1);
  await surface.focus(); await surface.press('0');
  const fitted = await expectWhole(surface); expect(fitted.k).toBeLessThan(.01);
  expect((await surface.boundingBox()).height).toBeLessThanOrEqual(64);
  await page.setViewportSize({ width: 390, height: 800 });
  await expect.poll(async () => (await matrix(svg)).k).toBeGreaterThan(fitted.k);
  await expectWhole(surface);
  const trigger = scope.getByRole('button', { name: 'Move diagram', exact: true });
  await trigger.click(); await expect(trigger).toHaveAttribute('aria-expanded', 'true');
  await action(card(page, 'q1'), 'open').focus();
  await page.locator('[data-language="en"]').click();
  await expect(trigger).toHaveAttribute('aria-expanded', 'false');
});
