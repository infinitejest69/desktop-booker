import { expect, test } from "./fixtures";

const nav = (page: import("@playwright/test").Page, name: string) =>
  page.getByRole("navigation", { name: "Main" }).getByRole("link", { name, exact: true }).click();

test("checking a guest out leaves the room dirty until housekeeping marks it", async ({ demoHotel: page }) => {
  // Tom Price is in G03; check him in and out from his booking.
  await nav(page, "Bookings");
  await page.getByRole("button", { name: "List", exact: true }).click();
  await page.getByRole("row", { name: /Tom Price/ }).getByRole("link").first().click();
  await page.getByRole("button", { name: "Check in" }).click();
  await page.getByRole("button", { name: "Check out" }).click();
  await expect(page.getByText(/Checked out/).first()).toBeVisible();

  await nav(page, "Floor plan");
  await page.getByRole("group", { name: "View" }).getByText("Cards").click();
  const room = page.getByRole("listitem", { name: "Room G03" });
  await expect(room).toContainText("Dirty");
  await room.getByText("Clean", { exact: true }).click();
  await expect(page.getByText("G03: clean")).toBeVisible();
  await expect(room).toContainText("Clean");
});

test("rooms slept in last night start the day dirty, and a floor can be marked clean", async ({ demoHotel: page }) => {
  await nav(page, "Floor plan");
  await expect(page.getByRole("heading", { level: 1, name: "Floor plan and housekeeping" })).toBeVisible();
  // Tom has been in G03 since yesterday.
  const g03 = page.getByRole("button", { name: /^Room G03: In house, dirty/ });
  await expect(g03).toBeVisible();
  await expect(page.getByText("To clean: 1")).toBeVisible();

  // Picking the room shows who's there and lets you mark it.
  await g03.click();
  const panel = page.getByRole("complementary", { name: "Room G03" });
  await expect(panel.getByRole("link", { name: "Tom Price" })).toBeVisible();
  await expect(panel).toContainText("Room only");

  await page.getByRole("button", { name: "Mark floor clean" }).click();
  await expect(page.getByText("Ground floor: 3 rooms clean")).toBeVisible();
  await expect(page.getByText("To clean: 0")).toBeVisible();
  await expect(page.getByRole("button", { name: /^Room G03: In house, clean/ })).toBeVisible();
});

test("the old housekeeping link opens the cards", async ({ demoHotel: page }) => {
  await page.evaluate(() => history.pushState({}, "", "/housekeeping"));
  await page.goto("/housekeeping");
  await expect(page).toHaveURL(/\/floor-plan\?view=cards/);
});

test("the dashboard shows each floor with rooms to clean", async ({ demoHotel: page }) => {
  const hotel = page.getByRole("region", { name: "The hotel today" });
  await expect(hotel.getByRole("link", { name: "Ground floor: open the floor plan" })).toBeVisible();
  await expect(hotel.getByRole("listitem", { name: /G03: In house, dirty/ }).or(hotel.getByTitle(/G03: In house, dirty/))).toBeVisible();
});

test("the daily lists show today's arrivals with board", async ({ demoHotel: page }) => {
  await nav(page, "Floor plan");
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
  await expect(page.getByRole("paragraph", { name: "Room rates" }).or(page.getByLabel("Room rates"))).toContainText(
    "Average rate per night sold (ADR) £95.00",
  );
});

test("the dashboard warns until a backup is taken", async ({ demoHotel: page }) => {
  const warning = page.getByRole("alert").filter({ hasText: "No backup in the last day." });
  await expect(warning).toBeVisible();
  await warning.getByRole("link", { name: "Check backups" }).click();

  await expect(page.getByRole("heading", { level: 1, name: "Backups" })).toBeVisible();
  await page.getByRole("button", { name: "Back up now" }).click();
  await expect(page.getByText("Backed up")).toBeVisible();
  await expect(page.getByRole("region", { name: "Backup status" })).toContainText("Last backup");
  await expect(page.getByRole("region", { name: "Backup status" }).getByRole("alert")).toHaveCount(0);
  await expect(page.getByRole("button", { name: "Restore" })).toHaveCount(1);

  await page.getByRole("link", { name: "Dashboard" }).click();
  await expect(page.getByText("No backup in the last day.")).toHaveCount(0);
});

test("a missing backup folder is refused and restoring asks first", async ({ demoHotel: page }) => {
  await page.goto("/setup/backups");
  const settings = page.getByRole("form", { name: "Backup settings" });
  await settings.getByRole("textbox", { name: "Backup folder" }).fill("Z:\\NoSuchDrive");
  await settings.getByRole("button", { name: "Save settings" }).click();
  await expect(page.getByText("That folder can't be found.", { exact: false })).toBeVisible();

  await page.getByRole("button", { name: "Back up now" }).click();
  await page.getByRole("button", { name: "Restore" }).click();
  const dialog = page.getByRole("alertdialog");
  await expect(dialog).toContainText("A backup of the data as it is now is taken first");
  await dialog.getByRole("button", { name: "Cancel" }).click();
  await expect(dialog).toHaveCount(0);
});

test("a duplicate guest can be merged in", async ({ demoHotel: page }) => {
  // A second record for Margaret, with a phone number her main one lacks.
  await page.evaluate(() => {
    const db = (window as unknown as { __fakeDb: { guests: object[] } }).__fakeDb;
    db.guests.push({
      id: crypto.randomUUID(),
      first_name: "Maggie",
      last_name: "Hughes",
      email: "",
      phone: "07700 900999",
      address: "",
      notes: "",
    });
  });

  await nav(page, "Guests");
  await page.getByRole("searchbox", { name: "Search" }).fill("Margaret");
  await page.getByRole("link", { name: "Margaret Hughes" }).click();
  const merge = page.getByRole("region", { name: "Merge a duplicate" });
  await merge.getByRole("searchbox", { name: "Find the duplicate" }).fill("Maggie");
  await merge.getByRole("button", { name: "Merge into Margaret Hughes" }).click();
  await page.getByRole("alertdialog").getByRole("button", { name: "Merge" }).click();
  await expect(page.getByText("Merged Maggie Hughes into Margaret Hughes")).toBeVisible();
  await expect(page.getByRole("region", { name: "Contact" })).toContainText("07700 900999");

  await nav(page, "Guests");
  await page.getByRole("searchbox", { name: "Search" }).fill("Maggie");
  await expect(page.getByText("No guests match.")).toBeVisible();
});
