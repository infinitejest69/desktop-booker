import { expect, test } from "./fixtures";

const nav = (page: import("@playwright/test").Page, name: string) =>
  page.getByRole("navigation", { name: "Main" }).getByRole("link", { name, exact: true }).click();

test.describe("report builder", () => {
  test.beforeEach(async ({ demoHotel: page }) => {
    await nav(page, "Reports");
    await page.getByRole("group", { name: "Reports" }).getByRole("button", { name: "Build a report" }).click();
    await expect(page.getByRole("heading", { level: 1, name: "Build a report" })).toBeVisible();
  });

  test("build a report, save it and put it on a dashboard", async ({ page }) => {
    await page.getByRole("combobox", { name: "Split by" }).click();
    await page.getByRole("option", { name: "Room type" }).click();
    const preview = page.getByRole("region", { name: "Room revenue by room type" });
    await expect(preview).toContainText("Double");
    await expect(preview).toContainText("Twin");

    await page.getByRole("button", { name: "Save report" }).click();
    await expect(page.getByText('Saved "Room revenue by room type"')).toBeVisible();
    await expect(page.getByRole("region", { name: "Saved reports" }).getByRole("row", { name: /Room revenue by room type/ })).toBeVisible();

    await page.getByRole("group", { name: "Reports" }).getByRole("button", { name: "Dashboards" }).click();
    await page.getByRole("button", { name: "New dashboard" }).click();
    await page.getByRole("textbox", { name: "Dashboard name" }).fill("Monday meeting");
    await page.getByRole("combobox", { name: "Add a saved report" }).click();
    await page.getByRole("option", { name: "Room revenue by room type" }).click();
    await page.getByRole("button", { name: "Add", exact: true }).click();
    await page.getByRole("button", { name: "Save dashboard" }).click();

    const card = page.getByRole("region", { name: "Room revenue by room type" });
    await expect(card).toContainText("Double");
    await expect(page.getByRole("combobox", { name: "Dashboard" })).toContainText("Monday meeting");
  });

  test("a saved report opens again with its settings", async ({ page }) => {
    await page.getByRole("combobox", { name: "Measure" }).click();
    await page.getByRole("option", { name: "Arrivals" }).click();
    await page.getByRole("button", { name: "Save report" }).click();
    await page.getByRole("button", { name: "New report" }).click();
    await expect(page.getByRole("combobox", { name: "Measure" })).toContainText("Arrivals");

    await page.getByRole("combobox", { name: "Measure" }).click();
    await page.getByRole("option", { name: "Departures" }).click();
    await page.getByRole("region", { name: "Saved reports" }).getByRole("button", { name: "Open" }).click();
    await expect(page.getByRole("combobox", { name: "Measure" })).toContainText("Arrivals");
    await expect(page.getByRole("textbox", { name: "Report name" })).toHaveValue("Arrivals by month");
  });

  test("a measure only offers the splits it can use", async ({ page }) => {
    await page.getByRole("combobox", { name: "Measure" }).click();
    await page.getByRole("option", { name: "Occupancy" }).click();
    await page.getByRole("combobox", { name: "Split by" }).click();
    await expect(page.getByRole("option", { name: "Room type" })).toBeVisible();
    await expect(page.getByRole("option", { name: "Booking source" })).toHaveCount(0);
  });
});

test("staff can't build reports, and the backend refuses them", async ({ page }) => {
  await page.goto("/?fake=locked");
  await page.getByRole("radio", { name: "Sam" }).check();
  await page.getByLabel("PIN").fill("5678");
  await page.getByRole("button", { name: "Sign in" }).click();
  await expect(page.getByText("Signed in as Sam")).toBeVisible();

  const refused = await page.evaluate(async () => {
    const tauri = (window as unknown as { __TAURI_INTERNALS__: { invoke: (c: string, a: object) => Promise<unknown> } })
      .__TAURI_INTERNALS__;
    const tries = ["report_catalogue", "list_saved_reports", "list_dashboards"].map((cmd) =>
      tauri.invoke(cmd, {}).then(
        () => "allowed",
        (e: { kind: string }) => e.kind,
      ),
    );
    return Promise.all(tries);
  });
  expect(refused).toEqual(["forbidden", "forbidden", "forbidden"]);
});
