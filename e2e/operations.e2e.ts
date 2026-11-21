import { expect, test } from "./fixtures";

const nav = (page: import("@playwright/test").Page, name: string) =>
  page.getByRole("navigation", { name: "Main" }).getByRole("link", { name, exact: true }).click();

test("checking a guest out leaves the room dirty until housekeeping marks it", async ({ demoHotel: page }) => {
  // Tom Price is in G03; check him in and out from his booking.
  await nav(page, "Bookings");
  await page.getByRole("row", { name: /Tom Price/ }).getByRole("link").first().click();
  await page.getByRole("button", { name: "Check in" }).click();
  await page.getByRole("button", { name: "Check out" }).click();
  await expect(page.getByText(/Checked out/).first()).toBeVisible();

  await nav(page, "Housekeeping");
  const room = page.getByRole("listitem", { name: "Room G03" });
  await expect(room).toContainText("Dirty");
  await room.getByText("Clean", { exact: true }).click();
  await expect(page.getByText("G03: clean")).toBeVisible();
  await expect(room).toContainText("Clean");
});

test("the daily lists show today's arrivals with board", async ({ demoHotel: page }) => {
  await nav(page, "Housekeeping");
  await page.getByRole("link", { name: "Daily lists" }).click();
  const arrivals = page.getByRole("region", { name: "Arrivals" });
  await expect(arrivals.getByRole("row", { name: /Margaret Hughes/ })).toContainText("Bed & breakfast");
  await expect(page.getByRole("button", { name: "Print" })).toBeVisible();
});

test("a guest's profile shows their stays and can be erased", async ({ demoHotel: page }) => {
  await nav(page, "Guests");
  await page.getByRole("searchbox", { name: "Search" }).fill("Hughes");
  await page.getByRole("link", { name: "Margaret Hughes" }).click();
  await expect(page.getByRole("heading", { level: 1, name: "Margaret Hughes" })).toBeVisible();
  await expect(page.getByRole("region", { name: "Guest at a glance" })).toContainText("1 booking, 2 nights");

  await page.getByRole("button", { name: "Erase details" }).click();
  await page.getByRole("alertdialog").getByRole("button", { name: "Erase details" }).click();
  await expect(page.getByRole("heading", { level: 1, name: "Erased guest" })).toBeVisible();
  // The booking itself is kept.
  await expect(page.getByRole("region", { name: "Stays" }).getByRole("row")).toHaveCount(2);
});

test("the occupancy report counts tonight's rooms", async ({ demoHotel: page }) => {
  await nav(page, "Reports");
  await expect(page.getByRole("heading", { level: 2, name: /^Occupancy/ })).toBeVisible();
  // Margaret and Tom fill 2 of the 3 rooms in service tonight.
  const tonight = new Date().getDate();
  await expect(
    page.getByRole("list", { name: "Occupancy by night" }).getByRole("listitem").nth(tonight - 1),
  ).toHaveAccessibleName(/66\.7%, 2 of 3/);
});
