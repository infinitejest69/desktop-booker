// `pnpm screenshots`: every main screen with the demo hotel, in light and
// dark, for the user guide and README (docs/guide/images/).
import { call, expect, isoDay, shoot, test } from "./fixtures.ts";

interface ChartEntry {
  booking_id: string;
  status: string;
  stay: { start: string; end: string };
}

test("front desk", async ({ page }) => {
  await page.goto("/");
  await expect(page.getByRole("region", { name: "Today at a glance" })).toBeVisible();
  await shoot(page, "dashboard");
  await page.goto("/lists");
  await shoot(page, "daily-lists");
});

test("bookings", async ({ page }) => {
  await page.goto("/chart");
  await shoot(page, "calendar");
  await page.goto("/bookings");
  await shoot(page, "bookings-list");
  await page.goto("/bookings/new");
  await shoot(page, "new-booking");

  // A guest staying now, with a few extras on the bill.
  const today = await call<ChartEntry[]>("chart", { from: isoDay(), to: isoDay(1) });
  const inHouse = today.find((e) => e.status === "checked_in" && e.stay.start < isoDay()) ?? today[0];
  await page.goto(`/bookings/${inHouse.booking_id}`);
  await expect(page.getByRole("heading", { level: 1 })).toBeVisible();
  await shoot(page, "booking");
  await page
    .getByRole("heading", { name: "Bill", exact: true })
    .evaluate((h) => h.scrollIntoView({ block: "start" }));
  await shoot(page, "bill");
});

test("an invoice", async ({ page }) => {
  const past = await call<ChartEntry[]>("chart", { from: isoDay(-30), to: isoDay(-1) });
  for (const entry of past.filter((e) => e.status === "checked_out")) {
    const folio = await call<{ invoices: { id: string }[] }>("get_folio", { owner: { kind: "booking", id: entry.booking_id } });
    if (folio.invoices.length) {
      await page.goto(`/invoices/${folio.invoices[0].id}`);
      await shoot(page, "invoice");
      return;
    }
  }
  throw new Error("no invoiced stay in the last month");
});

test("rooms and housekeeping", async ({ page }) => {
  await page.goto("/floor-plan");
  await shoot(page, "rooms");
  await page.goto("/housekeeping");
  await shoot(page, "housekeeping");
});

test("suites and events", async ({ page }) => {
  await page.goto("/suites");
  await shoot(page, "suites");
  await page.goto("/events");
  await shoot(page, "events");
  const events = await call<{ id: string; kind: string }[]>("list_events", { from: isoDay() });
  const wedding = events.find((e) => e.kind === "wedding") ?? events[0];
  await page.goto(`/events/${wedding.id}`);
  await shoot(page, "event");
});

test("guests", async ({ page }) => {
  await page.goto("/guests");
  await shoot(page, "guests");
  // A tour manager who comes back with every coach tour.
  const [manager] = await call<{ id: string }[]>("search_guests", { query: "tours@" });
  await page.goto(`/guests/${manager.id}`);
  await shoot(page, "guest");
});

test("reports", async ({ page }) => {
  await page.goto("/reports");
  await shoot(page, "reports");
  const saved = await call<{ id: string; spec: { name: string } }[]>("list_saved_reports");
  const byType = saved.find((r) => r.spec.name.startsWith("Room revenue by room type")) ?? saved[0];
  await page.goto(`/reports/build?id=${byType.id}`);
  await shoot(page, "report-builder");
  await page.goto("/reports/dashboards");
  await shoot(page, "report-dashboard", { fullPage: true });
});

test("setting up", async ({ page }) => {
  for (const [path, name] of [
    ["/setup/property", "setup-hotel"],
    ["/setup/floors", "setup-floors"],
    ["/setup/room-types", "setup-room-types"],
    ["/setup/rates", "setup-rates"],
    ["/setup/invoicing", "setup-invoicing"],
    ["/setup/users", "setup-users"],
    ["/setup/email", "setup-email"],
    ["/setup/backups", "setup-backups"],
    ["/setup/audit", "setup-audit"],
  ]) {
    await page.goto(path);
    await shoot(page, name);
  }
});
