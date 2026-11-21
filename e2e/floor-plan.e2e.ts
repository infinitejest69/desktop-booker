import { expect, openSetup, rectOf, test } from "./fixtures";

const CELL = 28;

test("empty hotel offers the demo, which shows on the floor plan", async ({ demoHotel: page }) => {
  await page.getByRole("link", { name: "Floor plan", exact: true }).click();
  await expect(page.getByText("4 bedrooms · 1 function suites · 1 floors")).toBeVisible();
  const rooms = page.getByRole("list", { name: "Ground floor rooms" });
  await expect(rooms.getByRole("listitem")).toHaveCount(5);
  await expect(rooms.getByRole("listitem").filter({ hasText: "G04" })).toContainText("Out of service");
});

test.describe("layout editor", () => {
  test.beforeEach(async ({ demoHotel: page }) => {
    await openSetup(page, "Layout editor");
    await expect(page.getByRole("group", { name: "Ground floor layout" })).toBeVisible();
  });

  const room = (page: import("@playwright/test").Page, name: string) =>
    page.getByRole("button", { name: new RegExp(`^${name}\\b`) });

  async function drag(page: import("@playwright/test").Page, name: string, cells: number) {
    const box = (await room(page, name).boundingBox())!;
    await page.mouse.move(box.x + 20, box.y + 20);
    await page.mouse.down();
    await page.mouse.move(box.x + 20 + CELL * cells, box.y + 20, { steps: 8 });
    await page.mouse.up();
  }

  test("dragging a room into free space moves and saves it", async ({ page }) => {
    await drag(page, "G01", 13);
    await expect.poll(() => rectOf(page, "G01")).toEqual({ x: 13, y: 0, w: 3, h: 4 });
    await expect(page.locator("#details-heading")).toHaveText("G01");
  });

  test("dropping a room onto another is refused", async ({ page }) => {
    await drag(page, "G02", 2);
    await expect(page.getByText("G02 would overlap G03")).toBeVisible();
    expect(await rectOf(page, "G02")).toEqual({ x: 3, y: 0, w: 3, h: 4 });
  });

  test("arrow keys move the focused room one cell", async ({ page }) => {
    await drag(page, "G01", 13);
    await expect.poll(() => rectOf(page, "G01")).toMatchObject({ x: 13, y: 0 });
    await room(page, "G01").focus();
    await page.keyboard.press("ArrowDown");
    await expect.poll(() => rectOf(page, "G01")).toMatchObject({ x: 13, y: 1 });
  });

  test("the corner handle resizes the selected room", async ({ page }) => {
    await drag(page, "G01", 13);
    await expect.poll(() => rectOf(page, "G01")).toMatchObject({ x: 13 });
    const handle = (await page.getByRole("button", { name: "Resize G01" }).boundingBox())!;
    await page.mouse.move(handle.x + 5, handle.y + 5);
    await page.mouse.down();
    await page.mouse.move(handle.x + 5 + CELL * 2, handle.y + 5, { steps: 6 });
    await page.mouse.up();
    await expect.poll(() => rectOf(page, "G01")).toEqual({ x: 13, y: 0, w: 5, h: 4 });
  });

  test("add bedroom places a new room in the first free spot", async ({ page }) => {
    await page.getByRole("button", { name: "Add bedroom" }).click();
    await expect(page.getByText("Added G05")).toBeVisible();
    expect(await rectOf(page, "G05")).toEqual({ x: 12, y: 0, w: 3, h: 4 });
    await expect(page.locator("#details-heading")).toHaveText("G05");
  });
});

test("setup pages load", async ({ demoHotel: page }) => {
  for (const name of ["Floors", "Room types", "Hotel details"]) {
    await openSetup(page, name);
    await expect(page.getByRole("heading", { level: 1, name })).toBeVisible();
  }
  await expect(page.getByRole("textbox", { name: "Hotel name" })).toHaveValue("The Grange");
});
