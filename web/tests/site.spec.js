import { test, expect } from "@playwright/test";
import { createServer } from "node:http";
import { readFile } from "node:fs/promises";

let server;
let siteUrl;

test.beforeAll(async () => {
  const html = await readFile(new URL("../../docs/site/index.html", import.meta.url));
  server = createServer((request, response) => {
    if (request.url === "/shoryo/") {
      response.writeHead(200, { "Content-Type": "text/html; charset=utf-8" });
      response.end(html);
    } else {
      response.writeHead(404);
      response.end();
    }
  });
  await new Promise((resolve) => server.listen(0, "127.0.0.1", resolve));
  siteUrl = `http://127.0.0.1:${server.address().port}/shoryo/`;
});

test.afterAll(async () => {
  if (server) await new Promise((resolve) => server.close(resolve));
});

test("the landing page opens in English under a project path", async ({ page }) => {
  const errors = [];
  page.on("pageerror", (error) => errors.push(error.message));
  await page.goto(siteUrl);
  await expect(page.locator("html")).toHaveAttribute("lang", "en");
  await expect(page.getByRole("heading", { level: 1 })).toContainText("decisions you can retrace");
  await page.getByRole("link", { name: "Explore a decision" }).click();
  await expect(page).toHaveURL(`${siteUrl}#experience`);
  expect(await page.locator("img").evaluateAll((images) => images.every((image) => image.complete && image.naturalWidth > 0))).toBe(true);
  expect(errors).toEqual([]);
});

test("language changes preserve the decision until reset and survive reload", async ({ page }) => {
  await page.goto(siteUrl);
  await page.getByRole("button", { name: "03 Record a decision" }).click();
  await expect(page.getByRole("button", { name: "Stamp as checked" })).toBeDisabled();
  await page.getByRole("button", { name: "Save to local files", exact: true }).click();
  await page.getByRole("button", { name: "日本語", exact: true }).click();
  await expect(page.getByRole("button", { name: "ローカルファイルに保存する ✓" })).toHaveAttribute("aria-pressed", "true");
  await page.getByRole("button", { name: "確認の判子を押す" }).click();
  await page.getByRole("button", { name: "English", exact: true }).click();
  await expect(page.getByRole("button", { name: "Stamped", exact: true })).toBeDisabled();
  await page.getByRole("button", { name: "Send the round →" }).click();
  await page.getByRole("button", { name: "日本語", exact: true }).click();
  await expect(page.getByRole("button", { name: "送信済み ✓" })).toBeDisabled();
  await page.getByRole("button", { name: "04 前提を見直す" }).click();
  await page.getByRole("button", { name: "保存先を変更した場面を見る →" }).click();
  await page.getByRole("button", { name: "English", exact: true }).click();
  await expect(page.getByText("Premise changed", { exact: true })).toHaveCount(2);
  await page.getByRole("button", { name: "Reset", exact: true }).click();
  await page.getByRole("button", { name: "03 Record a decision" }).click();
  await expect(page.getByRole("button", { name: "Stamp as checked" })).toBeDisabled();
  await page.getByRole("button", { name: "日本語", exact: true }).click();
  expect(await page.evaluate(() => localStorage.getItem("ba0918-language"))).toBe("ja");
  await page.reload();
  await expect(page.locator("html")).toHaveAttribute("lang", "ja");
  await expect(page.getByRole("button", { name: "01 問いを確かめる" })).toHaveAttribute("aria-pressed", "true");
});

test("the selected product screen follows the language without losing its selection", async ({ page }) => {
  await page.goto(siteUrl);
  const image = page.locator("#screen-image");
  const englishMap = await image.getAttribute("src");
  await page.getByRole("button", { name: "The agreed finished picture", exact: true }).click();
  const englishResult = await image.getAttribute("src");
  expect(englishResult).not.toBe(englishMap);
  await page.getByRole("button", { name: "日本語", exact: true }).click();
  await expect(page.getByRole("button", { name: "合意後の完成図", exact: true })).toHaveAttribute("aria-pressed", "true");
  expect(await image.getAttribute("src")).not.toBe(englishResult);
  await expect(image).toHaveAttribute("alt", /議題終了画面/);
  await page.getByRole("button", { name: "English", exact: true }).click();
  await expect(image).toHaveAttribute("src", englishResult);
  expect(await image.evaluate((element) => element.complete && element.naturalWidth > 0)).toBe(true);
});

test("both languages fit a phone and keep their README links in that language", async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto(siteUrl);
  for (const [button, language, readme] of [["English", "en", "README.md"], ["日本語", "ja", "README-ja.md"]]) {
    await page.getByRole("button", { name: button, exact: true }).click();
    await expect(page.locator("html")).toHaveAttribute("lang", language);
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
    const links = await page.locator('a[href*="README"]').evaluateAll((elements) => elements.map((element) => element.href));
    expect(links.length).toBeGreaterThan(0);
    expect(links.every((href) => new URL(href).pathname.endsWith(readme))).toBe(true);
  }
});
