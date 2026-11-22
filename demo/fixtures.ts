import { execFileSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import { test as base, expect, type Page } from "@playwright/test";

const client = fileURLToPath(new URL("./bridge-client.js", import.meta.url));

/** Pages talk to the real backend through the demo bridge. */
export const test = base.extend({
  page: async ({ page }, use) => {
    await page.addInitScript({ path: client });
    await use(page);
  },
});

/** Where guide screenshots go. */
export const imageDir = fileURLToPath(new URL("../docs/guide/images/", import.meta.url));

const bridge = "http://127.0.0.1:1430/invoke/";

/** Calls a command on the bridge directly, e.g. to find a booking to show. */
export async function call<T>(cmd: string, args: Record<string, unknown> = {}): Promise<T> {
  const response = await fetch(bridge + cmd, { method: "POST", body: JSON.stringify(args) });
  const value = await response.json();
  if (!response.ok) throw new Error(`${cmd}: ${JSON.stringify(value)}`);
  return value as T;
}

/** Today as YYYY-MM-DD, `days` from now. */
export function isoDay(days = 0): string {
  const d = new Date();
  d.setDate(d.getDate() + days);
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}`;
}

/**
 * Saves `<name>.webp` and `<name>-dark.webp` once the page has settled: no
 * requests in flight and nothing still "working it out".
 */
export async function shoot(page: Page, name: string, options: { fullPage?: boolean } = {}): Promise<void> {
  await page.waitForLoadState("networkidle");
  await expect(page.getByText("Working it out…")).toHaveCount(0);
  await page.mouse.move(0, 0);
  for (const scheme of ["light", "dark"] as const) {
    await page.emulateMedia({ colorScheme: scheme });
    await page.waitForTimeout(150);
    const png = await page.screenshot({ fullPage: options.fullPage ?? false });
    // Lossless WebP: the same picture at about a third of the size of a PNG.
    execFileSync(
      "ffmpeg",
      ["-loglevel", "error", "-y", "-f", "image2pipe", "-i", "-", "-c:v", "libwebp", "-lossless", "1", "-compression_level", "6",
        `${imageDir}${name}${scheme === "dark" ? "-dark" : ""}.webp`],
      { input: png },
    );
  }
  await page.emulateMedia({ colorScheme: "light" });
}

export { expect };
