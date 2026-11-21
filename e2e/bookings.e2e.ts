import { expect, openSetup, test } from "./fixtures";

test.describe("bookings", () => {
  test.beforeEach(async ({ demoHotel: page }) => {
    await page.getByRole("link", { name: "Tariff chart", exact: true }).click();
    await expect(page.getByRole("grid", { name: "Tariff chart" })).toBeVisible();
  });

  test("the chart shows existing bookings", async ({ page }) => {
    await expect(page.getByRole("gridcell", { name: /^Room G02, .*Margaret Hughes/ }).first()).toBeVisible();
    await expect(page.getByRole("gridcell", { name: /^Room G03, .*Tom Price/ }).first()).toBeVisible();
  });

  test("book a free night from the chart, then check in and out from the panel", async ({ page }) => {
    await page.getByRole("gridcell", { name: /^Room G01, .*: free$/ }).first().click();
    await expect(page.getByRole("heading", { name: "New booking" })).toBeVisible();
    await expect(page.getByRole("combobox", { name: "Room" })).toContainText("G01");
    await expect(page.getByText(/^Price £\d/)).toBeVisible();

    await page.getByRole("textbox", { name: "First name" }).fill("David");
    await page.getByRole("textbox", { name: "Last name" }).fill("Lewis");
    await page.getByRole("button", { name: "Make booking" }).click();

    // Back on the chart, with the new booking on it and a toast to open it.
    await expect(page.getByRole("grid", { name: "Tariff chart" })).toBeVisible();
    await expect(page.getByRole("gridcell", { name: /^Room G01, .*David Lewis/ }).first()).toBeVisible();
    await page.getByRole("button", { name: "Open booking" }).click();

    const panel = page.getByRole("dialog");
    await expect(panel).toContainText("David Lewis");
    await panel.getByRole("button", { name: "Check in" }).click();
    await expect(panel.locator(".status")).toHaveText("Checked in");
    await panel.getByRole("button", { name: "Check out" }).click();
    await expect(panel.locator(".status")).toHaveText("Checked out");
  });

  test("a room that's already booked can't be chosen", async ({ page }) => {
    const today = new Date().toLocaleDateString("en-GB", { weekday: "long", day: "numeric", month: "long" });
    await page.getByRole("gridcell", { name: `Room G01, ${today}: free` }).click();
    await page.getByRole("combobox", { name: "Room" }).click();
    await expect(page.getByRole("option", { name: /^G02 .*\(taken\)/ })).toHaveAttribute("aria-disabled", "true");
    await expect(page.getByRole("option", { name: /^G01 / })).not.toHaveAttribute("aria-disabled", "true");
  });

  test("cancelling asks first and frees the room", async ({ page }) => {
    await page.getByRole("gridcell", { name: /^Room G02, .*Margaret Hughes/ }).first().click();
    const panel = page.getByRole("dialog");
    await panel.getByRole("button", { name: "Cancel booking" }).click();
    const ask = page.getByRole("alertdialog");
    await expect(ask).toContainText("The room becomes free");
    await ask.getByRole("button", { name: "Cancel booking" }).click();
    await expect(panel.locator(".status")).toHaveText("Cancelled");
    await expect(panel.getByRole("button", { name: "Reinstate" })).toBeVisible();

    await page.keyboard.press("Escape");
    await expect(panel).toBeHidden();
    await expect(page.getByRole("gridcell", { name: /^Room G02, .*Margaret Hughes/ })).toHaveCount(0);
  });

  test("the chart works from the keyboard", async ({ page }) => {
    const first = page.getByRole("gridcell").first();
    await first.focus();
    await expect(first).toHaveAttribute("aria-label", /^Room G01, /);
    await page.keyboard.press("ArrowDown");
    await expect(page.locator(":focus")).toHaveAttribute("aria-label", /^Room G02, /);
    await page.keyboard.press("Home");
    await page.keyboard.press("ArrowUp");
    await page.keyboard.press("Enter");
    await expect(page.getByRole("heading", { name: "New booking" })).toBeVisible();
  });
});

test.describe("navigation", () => {
  test("the booking panel closes with Escape and Back, and opens as a full page", async ({ demoHotel: page }) => {
    await page.getByRole("link", { name: "Tariff chart", exact: true }).click();
    await page.getByRole("gridcell", { name: /^Room G02, .*Margaret Hughes/ }).first().click();
    const panel = page.getByRole("dialog");
    await expect(panel).toContainText("Margaret Hughes");

    await page.keyboard.press("Escape");
    await expect(panel).toBeHidden();

    await page.getByRole("gridcell", { name: /^Room G02, .*Margaret Hughes/ }).first().click();
    await page.getByRole("button", { name: "Back" }).click();
    await expect(panel).toBeHidden();
    await expect(page.getByRole("grid", { name: "Tariff chart" })).toBeVisible();

    await page.getByRole("gridcell", { name: /^Room G02, .*Margaret Hughes/ }).first().click();
    await panel.getByRole("link", { name: "Open full page" }).click();
    await expect(page.getByRole("navigation", { name: "Breadcrumb" })).toContainText("Bookings");
    await expect(page.getByRole("heading", { level: 1 })).toContainText("Margaret Hughes");

    await page.keyboard.press("Alt+ArrowLeft");
    await expect(page.getByRole("dialog")).toContainText("Margaret Hughes");
  });

  test("the chart remembers its week when you come back", async ({ demoHotel: page }) => {
    await page.getByRole("link", { name: "Tariff chart", exact: true }).click();
    const header = page.getByRole("columnheader").first();
    const before = await header.innerText();
    await page.getByRole("button", { name: "Week →" }).click();
    await expect(header).not.toHaveText(before);
    const firstDay = await header.innerText();
    await page.getByRole("gridcell").first().click();
    await expect(page.getByRole("heading", { name: "New booking" })).toBeVisible();
    await page.getByRole("button", { name: "Cancel" }).click();
    await expect(page.getByRole("columnheader").first()).toHaveText(firstDay);
  });

  test("Back is disabled on the first page", async ({ page }) => {
    await page.goto("/");
    await expect(page.getByRole("button", { name: "Back" })).toBeDisabled();
  });
});

test("rates: add a weekend rule", async ({ demoHotel: page }) => {
  await openSetup(page, "Rates");
  await page.getByRole("textbox", { name: "Name" }).first().fill("Weekend");
  await page.getByRole("button", { name: "Fri" }).click();
  await page.getByRole("button", { name: "Sat" }).click();
  await page.getByRole("button", { name: "Add rule" }).click();
  await expect(page.getByText("Added Weekend")).toBeVisible();
  const row = page.getByRole("row", { name: /Weekend/ });
  await expect(row).toContainText("Fri, Sat");
});

test("floor plan shows today's arrivals and in-house rooms", async ({ demoHotel: page }) => {
  await page.getByRole("link", { name: "Floor plan", exact: true }).click();
  const plan = page.getByRole("list", { name: "Ground floor rooms" });
  await expect(plan.getByRole("listitem").filter({ hasText: "G02" })).toContainText("Arriving");
  await expect(plan.getByRole("listitem").filter({ hasText: "G03" })).toContainText("In house");
  await expect(page.getByText("Arriving: 1")).toBeVisible();
});

test.describe("dashboard", () => {
  test("shows today's arrivals and checks a guest in", async ({ demoHotel: page }) => {
    const glance = page.getByRole("region", { name: "Today at a glance" });
    await expect(glance).toContainText("1Arriving");
    const arrivals = page.getByRole("region", { name: /Arrivals/ });
    await expect(arrivals).toContainText("Margaret Hughes");
    await arrivals.getByRole("button", { name: "Check in" }).click();
    await expect(page.getByText(/Margaret Hughes: checked in/)).toBeVisible();
    await expect(arrivals).toContainText("No more arrivals today");
    await expect(page.getByRole("region", { name: /In house/ })).toContainText("Margaret Hughes");
  });

  test("the two-week chart opens bookings in the chart's side panel", async ({ demoHotel: page }) => {
    await page.getByRole("grid", { name: "Coming weeks" }).getByRole("gridcell", { name: /Margaret Hughes/ }).first().click();
    await expect(page.getByRole("dialog")).toContainText("Margaret Hughes");
    await expect(page.getByRole("heading", { level: 1, name: "Tariff chart" })).toBeVisible();
  });

  test("My Hotel groups the setup pages", async ({ demoHotel: page }) => {
    await expect(page.getByRole("link", { name: "Floors", exact: true })).toBeHidden();
    await page.getByRole("button", { name: "My Hotel" }).click();
    await page.getByRole("link", { name: "Floors", exact: true }).click();
    await expect(page.getByRole("heading", { level: 1, name: "Floors" })).toBeVisible();
    await expect(page.getByRole("link", { name: "Floors", exact: true })).toHaveAttribute("aria-current", "page");
  });
});
