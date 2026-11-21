import { fileURLToPath } from "node:url";
import { test as base, expect, type Page } from "@playwright/test";

const fakeBackend = fileURLToPath(new URL("./fake-backend.js", import.meta.url));

interface Rect {
  x: number;
  y: number;
  w: number;
  h: number;
}

export const test = base.extend<{ demoHotel: Page }>({
  page: async ({ page }, use) => {
    await page.addInitScript({ path: fakeBackend });
    const errors: string[] = [];
    page.on("pageerror", (e) => errors.push(String(e)));
    page.on("console", (m) => {
      if (m.type() === "error") errors.push(m.text());
    });
    await use(page);
    expect(errors, "console errors").toEqual([]);
  },
  demoHotel: async ({ page }, use) => {
    await page.goto("/");
    await page.getByRole("button", { name: "Load demo hotel" }).click();
    await expect(page.getByRole("region", { name: "Today at a glance" })).toBeVisible();
    await use(page);
  },
});

export async function rectOf(page: Page, name: string): Promise<Rect> {
  return page.evaluate(
    (n) => (window as unknown as { __fakeDb: { spaces: { name: string; rect: Rect }[] } }).__fakeDb.spaces.find((s) => s.name === n)!.rect,
    name,
  );
}

/** Opens a page under the collapsible "My Hotel" menu. */
export async function openSetup(page: Page, name: string): Promise<void> {
  const link = page.getByRole("link", { name, exact: true });
  if (!(await link.isVisible())) await page.getByRole("button", { name: "My hotel" }).click();
  await link.click();
}

export { expect };
