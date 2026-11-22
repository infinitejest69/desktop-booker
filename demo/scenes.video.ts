// `pnpm videos`: short demo films of the app with the demo hotel, for
// YouTube. Each test records one video to videos/out/<name>.mp4.
// Run one with: pnpm videos -g "taking a booking"
import { test as base, expect, type Page } from "@playwright/test";
import { fileURLToPath } from "node:url";
import { call, isoDay } from "./fixtures.ts";
import { card, choose, click, glide, quiet, record, say, scrollTo, stageScript, type, uncard } from "./director.ts";

const client = fileURLToPath(new URL("./bridge-client.js", import.meta.url));

const test = base.extend({
  page: async ({ page }, use) => {
    await page.addInitScript({ path: client });
    await page.addInitScript({ path: stageScript });
    await use(page);
  },
});

const END = "DesktopBooker · free hotel software for your own computer";

interface Space {
  id: string;
  name: string;
  kind: string;
  room_type_id: string | null;
}
interface ChartEntry {
  booking_id: string;
  reference: string;
  guest_name: string;
  status: string;
  stay: { start: string; end: string };
}

function addDays(iso: string, n: number): string {
  const d = new Date(`${iso}T12:00:00`);
  d.setDate(d.getDate() + n);
  return d.toISOString().slice(0, 10);
}

/** "Friday 4 December", as the booking calendar names its cells. */
function longDay(iso: string): string {
  return new Date(`${iso}T12:00:00`).toLocaleDateString("en-GB", { weekday: "long", day: "numeric", month: "long" });
}

const nav = (page: Page, name: string) =>
  page.getByRole("navigation", { name: "Main" }).getByRole("link", { name, exact: true });

async function openDashboard(page: Page) {
  await page.goto("/");
  await expect(page.getByRole("region", { name: "Today at a glance" })).toBeVisible();
  await page.mouse.move(960, 600);
}

/** People on today's lists who aren't part of a group (one line each). */
async function todaysPeople(status: string, which: "start" | "end"): Promise<ChartEntry[]> {
  const entries = await call<ChartEntry[]>("chart", { from: isoDay(-1), to: isoDay(1) });
  const count = new Map<string, number>();
  for (const e of entries) count.set(e.booking_id, (count.get(e.booking_id) ?? 0) + 1);
  return entries.filter((e) => e.status === status && e.stay[which] === isoDay() && count.get(e.booking_id) === 1);
}

// ---------------------------------------------------------------------------

test("a tour in two minutes", async ({ page }) => {
  // A tour manager: a guest who comes back with every coach tour.
  const [guest] = await call<{ first_name: string; last_name: string }[]>("search_guests", { query: "tours@" });
  await openDashboard(page);
  await record(page, "01-tour", async () => {
    await card(page, "DesktopBooker", "Hotel booking software for small, independent hotels", 3500);
    await uncard(page);
    await say(page, "This is the dashboard of The Lanterns Hotel, a made-up seaside hotel with 40 rooms.");
    await glide(page, page.getByRole("region", { name: "The hotel today" }));
    await say(page, "Every room, floor by floor: who's arriving, who's in, who's leaving, and what needs cleaning.");
    await glide(page, page.getByRole("heading", { name: /^Arrivals/ }));
    await say(page, "Today's arrivals and departures, each one click to check in or out.");
    await quiet(page);

    await click(page, nav(page, "Bookings"), 1200);
    await say(page, "The booking calendar: a row for each room, a column for each night.");
    await click(page, page.getByRole("button", { name: "Week →" }), 900);
    await click(page, page.getByRole("button", { name: "Week →" }), 900);
    await say(page, "Click a free night to book it, or a booking to open it.");
    await click(page, page.getByRole("button", { name: "Today" }), 800);
    await click(page, page.getByRole("button", { name: "List", exact: true }), 1500);
    await say(page, "Or as a list: arrivals, nights, rooms and board.");
    await type(page, page.getByRole("searchbox").first(), "Hughes");
    await page.keyboard.press("Enter");
    await page.waitForTimeout(1500);
    await say(page, "Search by booking number, name or email.");
    await quiet(page);

    await click(page, nav(page, "Rooms"), 1200);
    await say(page, "Rooms shows each floor as it really is, with housekeeping at a glance.");
    await quiet(page);

    await click(page, nav(page, "Suites and events"), 1200);
    await click(page, page.getByRole("button", { name: "Week", exact: true }), 1200);
    await say(page, "Function suites and events: weddings, conferences and dinners, with catering and room blocks.");
    await click(page, page.getByRole("button", { name: "Events", exact: true }), 1200);
    await quiet(page);

    await click(page, nav(page, "Guests"), 1200);
    await say(page, "Find any guest by name, email or phone.");
    await type(page, page.getByRole("searchbox", { name: "Search" }), guest.last_name);
    await page.waitForTimeout(1200);
    await click(page, page.getByRole("link", { name: `${guest.first_name} ${guest.last_name}` }).first(), 1500);
    await say(page, "Their record: every stay, what they've spent, and notes for next time.");
    await page.mouse.wheel(0, 400);
    await page.waitForTimeout(1500);
    await quiet(page);

    await click(page, nav(page, "Reports"), 1200);
    await say(page, "Reports for the manager: occupancy, revenue, average room rate and more.");
    await click(page, page.getByRole("button", { name: "Dashboards" }), 1800);
    await say(page, "Build your own reports and put them on a dashboard.");
    await page.mouse.wheel(0, 600);
    await page.waitForTimeout(1500);
    await quiet(page);

    await card(page, "DesktopBooker", "No fees. Your data stays in the hotel. Works offline.", 4000, END);
  });
});

test("taking a booking over the phone", async ({ page }) => {
  // A dog-friendly room free for three nights, one to two weeks ahead.
  const spaces = await call<Space[]>("list_spaces", { floorId: null });
  const dogRooms = spaces.filter((s) => ["G03", "G04", "G05"].includes(s.name));
  let room: Space | undefined;
  let start = "";
  for (let ahead = 8; ahead <= 16 && !room; ahead++) {
    start = isoDay(ahead);
    const free = await call<string[]>("free_rooms", { stay: { start, end: addDays(start, 3) }, ignoreBooking: null });
    room = dogRooms.find((r) => free.includes(r.id));
  }
  if (!room) throw new Error("no dog-friendly room free");

  await openDashboard(page);
  await record(page, "02-taking-a-booking", async () => {
    await card(page, "Taking a booking", "Mrs Hughes rings for three nights, with her dog and her car");
    await uncard(page);
    await click(page, nav(page, "Bookings"), 1200);
    await say(page, `She'd like a dog-friendly room. ${room.name} is free from ${longDay(start)}.`);
    await click(page, page.getByRole("gridcell", { name: `Room ${room.name}, ${longDay(start)}: free` }), 1200);
    await quiet(page);

    await say(page, "The room and arrival are filled in. Three nights, so set the departure.", 2600);
    await glide(page, page.getByLabel("Departure"));
    await page.getByLabel("Departure").fill(addDays(start, 3));
    await page.waitForTimeout(600);
    await type(page, page.getByRole("textbox", { name: "First name" }), "Ffion");
    await type(page, page.getByRole("textbox", { name: "Last name" }), "Hughes");
    await type(page, page.getByRole("textbox", { name: "Email" }), "ffion.hughes@example.com");
    await type(page, page.getByRole("textbox", { name: "Phone" }), "07700 900123");
    await quiet(page);

    await say(page, "Two adults, one car to park, and dinner, bed and breakfast.");
    await click(page, page.getByRole("button", { name: "Increase Cars to park" }));
    await choose(page, page.getByRole("combobox", { name: "Board" }), "Dinner, bed & breakfast");
    await scrollTo(page, page.getByRole("heading", { name: "Booking", exact: true }));
    await choose(page, page.getByRole("combobox", { name: "Booked by" }), "Phone");
    await type(page, page.getByRole("textbox", { name: "Notes" }), "Bringing her dog, Bramble");
    await say(page, "The price is worked out from the room, the season, board and parking.");
    await click(page, page.getByRole("button", { name: "Make booking" }), 1500);
    await quiet(page);

    await say(page, "Booked. It's on the calendar straight away.");
    await click(page, page.getByRole("button", { name: "Open booking" }).first(), 1200);
    await click(page, page.getByRole("link", { name: "Open full page" }), 1500);
    await say(page, "She pays a £50 deposit by card over the phone.");
    const bill = page.getByRole("region", { name: "Bill" });
    await scrollTo(page, bill.getByRole("textbox", { name: "Amount" }));
    await type(page, bill.getByRole("textbox", { name: "Amount" }), "50");
    await click(page, bill.getByRole("switch", { name: "Deposit" }));
    await click(page, bill.getByRole("button", { name: "Record payment" }), 1500);
    await scrollTo(page, bill.getByRole("heading", { name: "Bill", exact: true }), "start");
    await say(page, "With the hotel's email set up, the confirmation goes from the same page.");
    await quiet(page);
    await card(page, "Booked in under a minute", "Mrs Hughes, Bramble and the car, three nights by the sea", 4000, END);
  });
});

test("check in, dinner and check out", async ({ page }) => {
  const [arriving] = await todaysPeople("confirmed", "start");
  const [leaving] = await todaysPeople("checked_in", "end");
  if (!arriving || !leaving) throw new Error("nobody arriving or leaving on their own today");

  await openDashboard(page);
  await record(page, "03-check-in-and-out", async () => {
    await card(page, "A day at the front desk", "Checking in, dinner on the bill, and checking out");
    await uncard(page);
    const arrivals = page.getByRole("region", { name: /Arrivals/ });
    const row = arrivals.getByRole("listitem").filter({ hasText: arriving.reference });
    await say(page, `${arriving.guest_name} has arrived. Their booking is on today's list.`);
    await glide(page, row.getByRole("button", { name: "Check in" }));
    await say(page, "Check in from here in one click, or open the booking to see everything first.");
    await click(page, row.getByRole("link", { name: arriving.guest_name }), 1500);
    await click(page, page.getByRole("button", { name: "Check in" }).first(), 1500);
    await quiet(page);

    await say(page, "In the evening they eat in the restaurant, and it goes on their bill.");
    const bill = page.getByRole("region", { name: "Bill" });
    await scrollTo(page, bill.getByRole("textbox", { name: "Item" }));
    await type(page, bill.getByRole("textbox", { name: "Item" }), "Dinner, Lanterns Restaurant");
    await choose(page, bill.getByRole("combobox", { name: "Kind" }), "Food");
    await type(page, bill.getByRole("textbox", { name: "Price each" }), "58.50");
    await click(page, bill.getByRole("button", { name: "Add to bill" }), 1500);
    await scrollTo(page, bill.getByRole("heading", { name: "Bill", exact: true }), "start");
    await say(page, "Room, meals and the dinner, with VAT worked out.");
    await quiet(page);

    await click(page, nav(page, "Dashboard"), 1500);
    await say(page, `Next morning: ${leaving.guest_name} is leaving. Open their bill.`);
    const departures = page.getByRole("region", { name: /Departures/ });
    await click(page, departures.getByRole("link", { name: leaving.guest_name }).first(), 1500);
    const theirBill = page.getByRole("region", { name: "Bill" });
    await scrollTo(page, theirBill.getByRole("button", { name: "Record payment" }));
    await say(page, "The amount owed is already filled in. They pay by card.");
    await click(page, theirBill.getByRole("button", { name: "Record payment" }), 1500);
    await click(page, theirBill.getByRole("button", { name: "Issue invoice" }), 2000);
    await say(page, "An invoice, ready to print or save as a PDF to email.");
    await glide(page, page.getByRole("button", { name: "Print or save as PDF" }));
    await page.mouse.wheel(0, 500);
    await page.waitForTimeout(1200);
    await quiet(page);
    await click(page, page.getByRole("link", { name: /^Back to Booking/ }), 1500);
    await say(page, "And check them out. Their room is marked for cleaning.");
    await click(page, page.getByRole("button", { name: "Check out" }).first(), 1800);
    await quiet(page);
    await card(page, "That's the front desk", "Check-in, the bill, payment and invoice, check-out", 4000, END);
  });
});

test("housekeeping", async ({ page }) => {
  await openDashboard(page);
  await record(page, "04-housekeeping", async () => {
    await card(page, "Housekeeping", "Which rooms need cleaning, and marking them done");
    await uncard(page);
    await say(page, "Each morning, rooms slept in last night are marked for cleaning. A brass dot shows them.");
    await glide(page, page.getByRole("region", { name: "The hotel today" }));
    await click(page, nav(page, "Rooms"), 1500);
    await say(page, "Rooms shows the whole hotel. Pick a room to see who's in it and change its state.");
    const dirty = page.getByRole("button", { name: /: .*dirty/ }).first();
    await click(page, dirty, 1500);
    await quiet(page);
    await page.keyboard.press("Escape");
    await page.waitForTimeout(600);
    await click(page, page.getByRole("group", { name: "View" }).getByText("Cards"), 1500);
    await say(page, "Or as cards, a floor at a time: handy on a tablet on the trolley.");
    await click(page, page.getByRole("button", { name: "Mark floor clean" }).first(), 1500);
    await say(page, "A whole floor done in one go.");
    await quiet(page);
    await card(page, "Clean rooms, happy guests", "Housekeeping in DesktopBooker", 3500, END);
  });
});

test("a wedding with a room block", async ({ page }) => {
  // A Saturday three to five months away with the ballroom free.
  let day = "";
  for (let ahead = 90; ahead < 150 && !day; ahead++) {
    const d = isoDay(ahead);
    if (new Date(`${d}T12:00:00`).getDay() !== 6) continue;
    const busy = await call<unknown[]>("list_suite_bookings", { from: `${d}T00:00:00`, to: `${addDays(d, 1)}T00:00:00` });
    if (busy.length === 0) day = d;
  }
  if (!day) throw new Error("no free Saturday");

  await openDashboard(page);
  await record(page, "05-wedding", async () => {
    await card(page, "A wedding", "The function suite, the catering and rooms for the guests");
    await uncard(page);
    await click(page, nav(page, "Suites and events"), 1200);
    await click(page, page.getByRole("button", { name: "Events", exact: true }), 1200);
    await click(page, page.getByRole("button", { name: "New event" }), 1200);
    await type(page, page.getByRole("textbox", { name: "Event name" }), "Evans–Kowalski wedding");
    await glide(page, page.getByLabel("First day"));
    await page.getByLabel("First day").fill(day);
    await page.getByLabel("Last day").fill(day);
    await page.waitForTimeout(500);
    await scrollTo(page, page.getByRole("heading", { name: "Organiser" }), "start");
    await type(page, page.getByRole("textbox", { name: "First name" }), "Seren");
    await type(page, page.getByRole("textbox", { name: "Last name" }), "Evans");
    await type(page, page.getByRole("textbox", { name: "Email" }), "seren.evans@example.org");
    await say(page, "The wedding breakfast for 86 adults and 8 children.");
    await click(page, page.getByRole("button", { name: "Add a sitting" }));
    const sitting = page.getByRole("group", { name: "Sitting 1" });
    await type(page, sitting.getByRole("textbox", { name: "Name" }), "Wedding breakfast");
    await type(page, sitting.getByRole("textbox", { name: "Adults" }), "86");
    await type(page, sitting.getByRole("textbox", { name: "Children" }), "8");
    await quiet(page);
    await say(page, "Hold six double rooms for guests, at a group rate, until four weeks before.");
    await scrollTo(page, page.getByRole("heading", { name: "Rooms for guests" }), "start");
    await click(page, page.getByRole("switch", { name: "Hold a block of rooms" }));
    await choose(page, page.getByRole("combobox", { name: "Room type" }), "Double");
    await type(page, page.getByRole("textbox", { name: "Rooms to hold" }), "6");
    await page.waitForTimeout(800);
    await click(page, page.getByRole("button", { name: "Create event" }), 2000);
    await quiet(page);

    await say(page, "The event page brings it all together: catering numbers, held rooms and suites.");
    await page.mouse.wheel(0, 500);
    await page.waitForTimeout(1500);
    await page.mouse.wheel(0, -500);
    await page.waitForTimeout(800);
    await say(page, "Now book the Harbour Ballroom for the day.");
    await click(page, page.getByRole("button", { name: "Book a suite" }), 1500);
    await type(page, page.getByRole("textbox", { name: "What it's for" }), "Wedding breakfast and evening party");
    await page.waitForTimeout(800);
    await scrollTo(page, page.getByRole("button", { name: /Book|Save/ }).last());
    await click(page, page.getByRole("button", { name: /Book|Save/ }).last(), 2000);
    await quiet(page);
    await card(page, "Weddings made simple", "Suites, catering and room blocks in one place", 4000, END);
  });
});

test("reports and dashboards", async ({ page }) => {
  await openDashboard(page);
  await record(page, "06-reports", async () => {
    await card(page, "Reports and dashboards", "Build your own, no spreadsheets needed");
    await uncard(page);
    await click(page, nav(page, "Reports"), 1500);
    await say(page, "The standard reports: occupancy, revenue, average room rate and RevPAR.");
    await page.mouse.wheel(0, 500);
    await page.waitForTimeout(1500);
    await quiet(page);
    await click(page, page.getByRole("group", { name: "Reports" }).getByRole("button", { name: "Build a report" }), 1500);
    await say(page, "Or build your own: pick a measure, how to split it, and the dates.");
    await choose(page, page.getByRole("combobox", { name: "Measure" }), "Room revenue");
    await choose(page, page.getByRole("combobox", { name: "Split by" }), "Room type");
    await choose(page, page.getByRole("combobox", { name: "Period" }), "Last 12 months");
    await say(page, "The chart updates as you go. Save it under a name.");
    await click(page, page.getByRole("button", { name: "Save report" }), 1500);
    await quiet(page);
    await click(page, page.getByRole("group", { name: "Reports" }).getByRole("button", { name: "Dashboards" }), 2000);
    await say(page, "Put saved reports together on a dashboard, like this one for the Monday meeting.");
    await page.mouse.wheel(0, 700);
    await page.waitForTimeout(2000);
    await page.mouse.wheel(0, 700);
    await page.waitForTimeout(2000);
    await quiet(page);
    await card(page, "Know how the hotel is doing", "Reports and dashboards in DesktopBooker", 4000, END);
  });
});

/** Opens a page under the collapsible My hotel menu. */
async function openSetup(page: Page, name: string) {
  const link = page.getByRole("link", { name, exact: true });
  if (!(await link.isVisible())) await click(page, page.getByRole("button", { name: "My hotel" }), 500);
  await click(page, link, 1500);
}

async function signIn(page: Page, name: string, pin: string) {
  await click(page, page.getByRole("button", { name: "Switch user" }), 1200);
  await click(page, page.getByRole("radio", { name }), 500);
  await type(page, page.getByLabel("PIN"), pin);
  await click(page, page.getByRole("button", { name: "Sign in" }), 1500);
}

test("setting up a hotel", async ({ page }) => {
  await openDashboard(page);
  await record(page, "07-setting-up", async () => {
    await card(page, "Setting up your hotel", "Your building, rooms and prices, done once");
    await uncard(page);
    await openSetup(page, "Hotel details");
    await say(page, "Everything about the hotel is under My hotel. First, its name, address and check-in times.");
    await glide(page, page.getByRole("textbox", { name: "Hotel name" }));
    await quiet(page);

    await openSetup(page, "Floors and layout");
    await say(page, "Draw each floor as it really is. Let's add the new garden annexe.");
    await click(page, page.getByRole("button", { name: "Add floor" }), 1200);
    await type(page, page.getByRole("textbox", { name: "Floor name" }), "Garden annexe");
    await click(page, page.getByRole("button", { name: "Save floor" }), 1200);
    await say(page, "Add four bedrooms in one go, then drag them where they belong.");
    await type(page, page.getByRole("textbox", { name: "Bedrooms to add" }), "4");
    await click(page, page.getByRole("button", { name: "Add 4 bedrooms" }), 1500);
    const plan = page.getByRole("group", { name: "Garden annexe layout" });
    const room = plan.getByRole("button").last();
    await glide(page, room);
    const box = (await room.boundingBox())!;
    await page.mouse.down();
    await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2 + 200, { steps: 40 });
    await page.mouse.up();
    await page.waitForTimeout(1200);
    await say(page, "Pick a room to set its number, type and features: wifi, dogs welcome, accessible.");
    await click(page, room, 1200);
    await choose(page, page.getByRole("combobox", { name: "Room type" }), "Sea View Double");
    await click(page, page.getByRole("button", { name: "Save", exact: true }), 1200);
    await quiet(page);

    await openSetup(page, "Room types");
    await say(page, "Room types set who can stay and the normal price per night.");
    await quiet(page);
    await openSetup(page, "Rates");
    await say(page, "Rate rules change prices for seasons, weekends and offers.");
    await scrollTo(page, page.getByRole("heading", { name: "Board options" }), "start");
    await say(page, "Board, such as bed and breakfast, is priced per adult and child. Parking per car, per night.");
    await quiet(page);
    await card(page, "Ready for guests", "Set up once, and change it whenever you like", 4000, END);
  });
});

test("staff, PINs and the audit log", async ({ page }) => {
  await openDashboard(page);
  await record(page, "08-staff-and-audit", async () => {
    await card(page, "Staff, PINs and the audit log", "Everyone signs in as themselves");
    await uncard(page);
    await openSetup(page, "Users");
    await say(page, "Add each person who works the desk, with their own PIN.");
    await type(page, page.getByRole("textbox", { name: "Name" }), "Megan");
    await choose(page, page.getByRole("combobox", { name: "Role" }), "Staff");
    await type(page, page.getByLabel("PIN", { exact: true }), "2468");
    await click(page, page.getByRole("button", { name: "Add user" }), 1500);
    await quiet(page);

    await click(page, nav(page, "Dashboard"), 1200);
    await say(page, "When the shift changes, Megan signs in with her PIN.");
    await signIn(page, "Megan", "2468");
    await say(page, "Staff take bookings, check guests in and out and run the bills. Setup and reports are for admins.");
    await glide(page, page.getByRole("navigation", { name: "Main" }));
    const first = page.getByRole("region", { name: /Arrivals/ }).getByRole("listitem").first();
    await click(page, first.getByRole("button", { name: "Check in" }), 1500);
    await quiet(page);

    await say(page, "Back to Andy, the owner.");
    await signIn(page, "Andy", "1234");
    await openSetup(page, "Audit log");
    await say(page, "The audit log shows who changed what, and when: including Megan's check-in.");
    await page.mouse.wheel(0, 300);
    await page.waitForTimeout(1500);
    await quiet(page);
    await card(page, "Everyone accountable", "Own PINs, the right access, every change recorded", 4000, END);
  });
});

test("guests and data protection", async ({ page }) => {
  const [someone] = await call<{ first_name: string; last_name: string; email: string }[]>("search_guests", {
    query: "@example.net",
  });
  await openDashboard(page);
  await record(page, "09-guests", async () => {
    await card(page, "Guests", "History, duplicates and data protection");
    await uncard(page);
    await click(page, nav(page, "Guests"), 1200);
    await say(page, "Find a guest by name, email or phone.");
    await type(page, page.getByRole("searchbox", { name: "Search" }), "Trevithick");
    await page.waitForTimeout(1200);
    await say(page, "Margaret Trevithick is a regular, but she was once booked as Maggie: two records for one person.");
    await click(page, page.getByRole("link", { name: "Margaret Trevithick" }).first(), 1500);
    await say(page, "Her record: every stay, what she's spent, and notes for next time.");
    await scrollTo(page, page.getByRole("heading", { name: "Stays" }), "start");
    const merge = page.getByRole("region", { name: "Merge a duplicate" });
    await scrollTo(page, merge);
    await say(page, "Merge the duplicate in. Its booking moves across, and its phone number fills the gap.");
    await type(page, merge.getByRole("searchbox", { name: "Find the duplicate" }), "Maggie");
    await page.waitForTimeout(800);
    await click(page, merge.getByRole("button", { name: "Merge into Margaret Trevithick" }), 800);
    await click(page, page.getByRole("alertdialog").getByRole("button", { name: "Merge" }), 1800);
    await quiet(page);

    await scrollTo(page, page.getByRole("heading", { level: 1 }).first(), "start");
    await say(page, "If a guest asks what you hold about them, save it all as a file in one click.");
    await glide(page, page.getByRole("button", { name: "Save their data…" }));
    await quiet(page);
    await click(page, nav(page, "Guests"), 1000);
    await type(page, page.getByRole("searchbox", { name: "Search" }), someone.email);
    await page.waitForTimeout(1000);
    await click(page, page.getByRole("link", { name: `${someone.first_name} ${someone.last_name}` }).first(), 1500);
    await say(page, "And if they ask to be forgotten, erase their details. Their bookings stay for the accounts.");
    await click(page, page.getByRole("button", { name: "Erase details" }), 800);
    await click(page, page.getByRole("alertdialog").getByRole("button", { name: "Erase details" }), 1800);
    await quiet(page);
    await card(page, "Your guests, looked after", "Their history, kept tidy and kept private", 4000, END);
  });
});
