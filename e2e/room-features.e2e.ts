import { expect, openSetup, test } from "./fixtures";

// The fake demo hotel: G01 wifi + accessible, G02 wifi + pets allowed,
// G03 wifi, G04 smoking (out of service), and the Garden Suite.

const mainNav = (page: import("@playwright/test").Page, name: string) =>
  page.getByRole("navigation", { name: "Main" }).getByRole("link", { name, exact: true }).click();

test("an admin sets a room's features in Floors and layout", async ({ demoHotel: page }) => {
  await openSetup(page, "Floors and layout");
  await page.getByRole("button", { name: /^G03\b/ }).click();
  const features = page.getByRole("group", { name: /^Features/ });
  await expect(features.getByRole("checkbox", { name: "Wifi" })).toBeChecked();
  await features.getByRole("checkbox", { name: "Pets allowed" }).click();
  await page.getByRole("button", { name: "Save", exact: true }).click();
  await expect(page.getByText("Saved G03")).toBeVisible();

  const saved = await page.evaluate(
    () =>
      (window as unknown as { __fakeDb: { spaces: { name: string; features: string[] }[] } }).__fakeDb.spaces.find(
        (s) => s.name === "G03",
      )!.features,
  );
  expect(saved.sort()).toEqual(["pets_allowed", "wifi"]);
});

test("function suites have no room features", async ({ demoHotel: page }) => {
  await openSetup(page, "Floors and layout");
  await page.getByRole("button", { name: /^Garden Suite\b/ }).click();
  await expect(page.locator("#details-heading")).toHaveText("Garden Suite");
  await expect(page.getByRole("checkbox", { name: "Wifi" })).toHaveCount(0);
});

test("Rooms and the calendar show each room's features", async ({ demoHotel: page }) => {
  await mainNav(page, "Rooms");
  const g01 = page.getByRole("button", { name: /^Room G01:/ });
  await expect(g01).toHaveAccessibleName(/Wifi, accessible, non-smoking$/);
  await g01.click();
  await expect(page.getByRole("complementary", { name: "Room G01" })).toContainText("Wifi, accessible, non-smoking");

  await mainNav(page, "Bookings");
  const g02Row = page.getByRole("rowheader", { name: /G02/ });
  await expect(g02Row.getByRole("listitem")).toHaveText(["Wifi", "Pets allowed"]);
});

test("a booking can ask for a pet-friendly room", async ({ demoHotel: page }) => {
  await page.getByRole("button", { name: "New booking" }).click();
  await expect(page.getByRole("heading", { name: "New booking" })).toBeVisible();
  // A month ahead, when every room is free (G02 has a guest tonight).
  const later = new Date(Date.now() + 30 * 86_400_000).toISOString().slice(0, 10);
  await page.getByLabel("Arrival").fill(later);
  await page.getByRole("group", { name: "Room must have" }).getByRole("button", { name: "Pets allowed" }).click();

  await page.getByRole("combobox", { name: "Room" }).click();
  await expect(page.getByRole("option", { name: /^G01\b/ })).toContainText("doesn't match");
  await expect(page.getByRole("option", { name: /^G01\b/ })).toBeDisabled();
  await expect(page.getByRole("option", { name: /^G02\b/ })).toContainText("pets allowed");
  await expect(page.getByRole("option", { name: /^G02\b/ })).toBeEnabled();
});

test("parking is charged per car per night and shows on the booking", async ({ demoHotel: page }) => {
  await openSetup(page, "Rates");
  const parking = page.getByRole("form", { name: "Parking" });
  await parking.getByLabel("Per car per night").fill("8");
  await parking.getByRole("button", { name: "Save parking price" }).click();
  await expect(page.getByText("Parking: £8.00 per car per night")).toBeVisible();

  await mainNav(page, "Bookings");
  await page.getByRole("button", { name: "New booking" }).click();
  const later = new Date(Date.now() + 30 * 86_400_000).toISOString().slice(0, 10);
  await page.getByLabel("Arrival").fill(later);
  await page.getByRole("combobox", { name: "Room" }).click();
  await page.getByRole("option", { name: /^G03\b/ }).click();
  await page.getByLabel("Cars to park", { exact: true }).fill("2");
  await page.getByLabel("Cars to park", { exact: true }).blur();
  await page.getByText(/^Price £/).click();
  await expect(page.getByRole("row").filter({ hasText: "£16.00" }).first()).toBeVisible();

  await page.getByRole("textbox", { name: "First name" }).fill("Priya");
  await page.getByRole("textbox", { name: "Last name" }).fill("Shah");
  await page.getByRole("button", { name: "Make booking" }).click();
  await page.getByRole("button", { name: "Open booking" }).click();
  await expect(page.getByRole("dialog")).toContainText("parking for 2 cars");
});
