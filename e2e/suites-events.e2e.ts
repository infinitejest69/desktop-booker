import { expect, openSetup, test } from "./fixtures";

const nav = (page: import("@playwright/test").Page, name: string) =>
  page.getByRole("navigation", { name: "Main" }).getByRole("link", { name, exact: true }).click();

test.describe("function suites", () => {
  test("the day timeline shows today's hire and opens it", async ({ demoHotel: page }) => {
    await nav(page, "Function suites");
    await expect(page.getByRole("heading", { level: 1, name: "Function suites" })).toBeVisible();
    const lane = page.getByRole("region", { name: "Garden Suite" });
    const meeting = lane.getByRole("link", { name: /Board meeting/ });
    await expect(meeting).toHaveAccessibleName(/10:00 to 12:00.*40 theatre, confirmed/);
    await meeting.click();
    await expect(page.getByRole("heading", { level: 1, name: "Garden Suite: Board meeting" })).toBeVisible();
    // Two hours, charged at the three-hour minimum of £100 an hour.
    await expect(page.getByText("Price £300.00")).toBeVisible();
  });

  test("each suite's hour lines line up with the time header", async ({ demoHotel: page }) => {
    await nav(page, "Function suites");
    const header = page.locator(".ticks span");
    const lines = page.getByRole("region", { name: "Garden Suite" }).locator(".hour-lines span");
    await expect(header).toHaveCount(17);
    await expect(lines).toHaveCount(17);
    for (const i of [1, 8, 16]) {
      const [h, l] = [await header.nth(i).boundingBox(), await lines.nth(i).boundingBox()];
      expect(Math.abs((h?.x ?? 0) - (l?.x ?? 99))).toBeLessThan(1);
    }
  });

  test("a booking that only clashes through setup time is refused", async ({ demoHotel: page }) => {
    await nav(page, "Function suites");
    await page.getByRole("button", { name: "Book a suite" }).click();
    await expect(page.getByRole("heading", { level: 1, name: "Book a suite" })).toBeVisible();
    await page.getByRole("textbox", { name: "What it's for" }).fill("Lunch talk");
    // The meeting's clear-down runs to 12:30; this booking's setup would start at 11:45.
    await page.getByLabel("Starts").fill("12:45");
    await page.getByLabel("Ends").fill("14:00");
    await page.getByRole("textbox", { name: "Last name" }).fill("Rees");
    await page.getByRole("button", { name: "Book the suite" }).click();
    await expect(page.getByText(/Garden Suite is in use then \("Board meeting"/)).toBeVisible();

    await page.getByLabel("Starts").fill("13:30");
    await page.getByLabel("Ends").fill("15:30");
    await page.getByRole("button", { name: "Book the suite" }).click();
    await expect(page.getByRole("heading", { level: 1, name: "Function suites" })).toBeVisible();
    await expect(page.getByRole("region", { name: "Garden Suite" }).getByRole("link", { name: /Lunch talk/ })).toBeVisible();
  });

  test("the week view lists each day's hires", async ({ demoHotel: page }) => {
    await nav(page, "Function suites");
    await page.getByRole("group", { name: "View" }).getByText("Week").click();
    await expect(page.getByRole("button", { name: "Next week" })).toBeVisible();
    await expect(page.getByRole("link", { name: /Board meeting/ })).toBeVisible();
    await expect(page.getByRole("link", { name: /^Book Garden Suite on/ }).first()).toBeVisible();
  });

  test("an admin sets a suite's rates in Floors and layout", async ({ demoHotel: page }) => {
    await openSetup(page, "Floors and layout");
    await page.getByRole("button", { name: /Garden Suite/ }).first().click();
    const rates = page.getByRole("form", { name: "Hire rates" });
    await expect(rates).toBeVisible();
    await rates.getByRole("textbox", { name: "Per hour" }).fill("120");
    await rates.getByRole("button", { name: "Save rates" }).click();
    await expect(page.getByText("Saved the rates for Garden Suite")).toBeVisible();
  });
});

test.describe("events", () => {
  test("the event page brings suites, catering and held rooms together", async ({ demoHotel: page }) => {
    await nav(page, "Events");
    await page.getByRole("link", { name: "Hughes–Price wedding" }).click();
    await expect(page.getByRole("heading", { level: 1, name: "Hughes–Price wedding" })).toBeVisible();

    const glance = page.getByRole("region", { name: "Event at a glance" });
    // 64 × £55 for the breakfast, plus one room held at £85.
    await expect(glance).toContainText("£3,605.00 in all");
    await expect(glance).toContainText("60 adults and 4 children");

    await expect(page.getByRole("region", { name: "Suites" })).toContainText("Wedding breakfast");
    await expect(page.getByRole("list", { name: "Dietary needs for Wedding breakfast" })).toContainText("1 nut allergy");
    const rooms = page.getByRole("region", { name: "Bedrooms" });
    await expect(rooms).toContainText("1 still held until");
    await expect(rooms.getByRole("row", { name: /G01/ })).toContainText("Held");
  });

  test("a new event holds a block of rooms", async ({ demoHotel: page }) => {
    await nav(page, "Events");
    await page.getByRole("button", { name: "New event" }).click();
    await page.getByRole("textbox", { name: "Event name" }).fill("Davies retirement dinner");
    await page.getByRole("textbox", { name: "Last name" }).fill("Davies");
    await page.getByRole("button", { name: "Add a sitting" }).click();
    await page.getByRole("group", { name: "Sitting 1" }).getByRole("textbox", { name: "Name" }).fill("Dinner");
    await page.getByRole("switch", { name: "Hold a block of rooms" }).click();
    await page.getByRole("textbox", { name: "Rooms to hold" }).fill("2");
    await page.getByRole("button", { name: "Create event" }).click();

    await expect(page.getByRole("heading", { level: 1, name: "Davies retirement dinner" })).toBeVisible();
    await expect(page.getByRole("region", { name: "Bedrooms" })).toContainText("2 still held until");
    await expect(page.getByRole("region", { name: "Event at a glance" })).toContainText("50 adults");
  });

  test("the dashboard warns before held rooms are released", async ({ demoHotel: page }) => {
    const panel = page.getByRole("region", { name: "Events in the next two weeks" });
    await expect(panel.getByRole("note")).toContainText("1 held room for Hughes–Price wedding go back on sale");
  });
});

test("staff can build events but not set the group rate", async ({ page }) => {
  await page.goto("/");
  await page.getByRole("button", { name: "Load demo hotel" }).click();
  await page.getByRole("button", { name: "Switch user" }).click();
  await page.getByRole("radio", { name: "Sam" }).check();
  await page.getByLabel("PIN").fill("5678");
  await page.getByRole("button", { name: "Sign in" }).click();

  await nav(page, "Events");
  await page.getByRole("link", { name: "Hughes–Price wedding" }).click();
  await page.getByRole("button", { name: "Edit event" }).click();
  await expect(page.getByText("only an admin can change it")).toBeVisible();
  await expect(page.getByRole("textbox", { name: "Group rate per room per night" })).toHaveCount(0);
  await page.getByRole("button", { name: "Save changes" }).click();
  await expect(page.getByRole("heading", { level: 1, name: "Hughes–Price wedding" })).toBeVisible();
});
