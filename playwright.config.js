// Browser tests of the screen, against the real binary (scripts/test-web.sh builds it first).
import { defineConfig, devices } from "@playwright/test";

export default defineConfig({
  testDir: "web/tests",
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  reporter: process.env.CI ? "list" : "line",
  timeout: 30_000,
  projects: [{ name: "chromium", use: { ...devices["Desktop Chrome"] } }],
});
