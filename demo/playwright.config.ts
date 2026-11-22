/// <reference types="node" />
// Demo runs (screenshots and videos): the app in Chromium, talking to the
// real backend through the demo bridge, with the demo hotel loaded.
// `pnpm screenshots`. Separate from the e2e tests, which use the fake backend.
import { existsSync } from "node:fs";
import { defineConfig, devices } from "@playwright/test";

const systemChromium = ["/usr/bin/chromium", "/usr/bin/chromium-browser"].find(existsSync);
const executablePath = process.env.CHROMIUM_PATH ?? (process.env.CI ? undefined : systemChromium);

export default defineConfig({
  testDir: ".",

  // One browser at a time: every scene shares the one demo database.
  workers: 1,
  fullyParallel: false,
  reporter: "list",
  timeout: 120_000,
  globalSetup: "./global-setup.ts",
  use: {
    baseURL: "http://127.0.0.1:1420",
    // Fail a step quickly rather than wait out the whole scene.
    actionTimeout: 20_000,
    trace: "retain-on-failure",
    locale: "en-GB",
    timezoneId: "Europe/London",
  },
  projects: [
    {
      name: "screenshots",
      testMatch: "**/*.demo.ts",
      use: { ...devices["Desktop Chrome"], viewport: { width: 1440, height: 900 }, deviceScaleFactor: 2, launchOptions: { executablePath } },
    },
    {
      // Full HD (1080p) at 1×, so whole screens fit: Andy's choice.
      name: "videos",
      testMatch: "**/*.video.ts",
      timeout: 600_000,
      use: { ...devices["Desktop Chrome"], viewport: { width: 1920, height: 1080 }, deviceScaleFactor: 1, launchOptions: { executablePath } },
    },
  ],
  webServer: [
    {
      command: "pnpm dev",
      url: "http://127.0.0.1:1420",
      reuseExistingServer: true,
      timeout: 120_000,
    },
    {
      // A fresh database every run, so every run shows the same hotel.
      command: "cargo run -q -p desktop-booker --bin demo-bridge --features demo-bridge -- --fresh --port 1430",
      port: 1430,
      reuseExistingServer: false,
      timeout: 600_000,
    },
  ],
});
