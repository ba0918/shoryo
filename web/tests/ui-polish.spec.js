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
