/// <reference types="node" />
import { existsSync } from "node:fs";
import { defineConfig, devices } from "@playwright/test";

// Locally, use the system Chromium if there is one; CI installs Playwright's own.
const systemChromium = ["/usr/bin/chromium", "/usr/bin/chromium-browser"].find(existsSync);
const executablePath = process.env.CHROMIUM_PATH ?? (process.env.CI ? undefined : systemChromium);

export default defineConfig({
  testDir: "e2e",
  testMatch: "**/*.e2e.ts",
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 1 : 0,
  reporter: process.env.CI ? "github" : "list",
  use: {
    baseURL: "http://127.0.0.1:1420",
    trace: "retain-on-failure",
    viewport: { width: 1400, height: 900 },
  },
  projects: [
    {
      name: "chromium",
      use: { ...devices["Desktop Chrome"], viewport: { width: 1400, height: 900 }, launchOptions: { executablePath } },
    },
  ],
  webServer: {
    command: "pnpm dev",
    url: "http://127.0.0.1:1420",
    reuseExistingServer: !process.env.CI,
    timeout: 120_000,
  },
});
