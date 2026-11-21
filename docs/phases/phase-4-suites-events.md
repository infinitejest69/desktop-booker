# Phase 4 — Function suites & events

**Goal:** book function suites by time and group everything for a wedding or dinner under one event.
**Rust concepts:** more complex domain modelling, enums with data, time ranges with `chrono::NaiveDateTime`.

## Tasks
- [x] `core`: time-slot availability with setup/teardown buffers
- [x] `core`: hire types: hourly / half-day / day, with pricing
- [x] `store`: migrations for `suite_bookings`, `events`, event links
- [x] Suite booking: layout, attendees (≤ capacity for that layout), notes, catering requirements
- [x] Event container: name, type (wedding/conference/dinner…), contact, date(s)
- [x] Link suite bookings + room block (held rooms with release date) to an event
- [ ] Packages (e.g. wedding package: suite + per-head catering + room block rate)
- [x] Catering numbers / dietary summary per event
- [x] UI: suite calendar (day/week timeline per suite)
- [x] UI: event page bringing everything together
- [ ] Floor-plan view shows suite occupancy for a chosen time
- [x] Tests: buffer overlaps, capacity per layout, room-block release

## Acceptance
- Two suite bookings that collide only through buffers are rejected.
- An event page shows suites, rooms, numbers and value in one place.

## Notes / decisions
**Decisions from Andy (2026-11-21):**
- Suite pricing: **hourly** (with a minimum number of hours), **half day / full day**, **per head** (packages, e.g. a day-delegate rate) and **set by hand**. All four are supported; the booking chooses one.
- **Setup and clear-down buffers per suite** (defaults on the suite, editable per booking). Bookings may not overlap *including* buffers.
- An event holds **suite bookings**, **catering sittings with numbers and dietary notes**, a **room block** for guests, and the **organiser's contact plus notes / running order**.
- **Staff and admins** can book suites and events; hand-set prices stay admin only.

**Design:**
- `booker-core/src/suites.rs`:
  - suite rate card (`hourly`, `min_hours`, `half_day`, `full_day`, default `setup_minutes` / `cleardown_minutes`), stored on the suite;
  - `SuitePricing` enum: `Hourly`, `HalfDay`, `FullDay`, `PerHead { price }`, `Manual { price }`;
  - `SuiteBooking` (suite, optional event, organiser guest, start/end `NaiveDateTime`, layout, attendees, buffers, pricing, status, notes);
  - `validate_suite_booking`: end after start; layout offered and attendees ≤ its capacity; no overlap with the suite's other live bookings once both sides' buffers are added;
  - `price_suite_booking`: hourly rounds up to whole hours and applies the minimum.
- **Events** (`Event`): name, kind (wedding / conference / dinner / party / other), organiser guest, dates, notes and running order, catering sittings (name, time, adults, children, dietary notes).
- **Room block:** reuses bedroom bookings. Creating a block makes one *provisional* booking per held room, linked to the event, at the group rate (a price override set by the system). On or after the release date, any block rooms still provisional are released (cancelled) automatically when the app starts, and the dashboard lists blocks due for release. Rooms taken up stay as normal bookings; their guest can be changed to the real guest.
- **UI:** suite day/week timeline, suite booking form, event page (suites, sittings, dietary summary, room block, total value), floor plan shows suite use at a chosen time.
- **Progress:** `booker-core` now has `suites.rs` (rate card, layouts, `SuitePricing`, `SuiteBooking`, `validate_suite_booking`, `price_suite_booking`) and `events.rs` (`Event`, `EventKind`, `CateringSitting`, `catering_totals`, `RoomBlock`, `validate_event`, `block_due_for_release`), with 11 tests. **Next:** store migration `0004` (suite rate columns on `spaces`, `suite_bookings`, `events`, `event_sittings`, a link from `bookings` to `event_id`), repositories with audit, commands, then UI.

**More decisions (2026-11-21):**
- A room block is a set of ordinary bedroom bookings linked by `bookings.event_id`. A room is *held* while its booking is provisional; confirming it (usually after changing the guest to the person staying) means it has been taken up. Every live linked booking counts towards the block, so resizing never touches a taken-up room.
- Held rooms are priced at the group rate for the whole stay (stored as the room's price override). They are booked to the organiser, sleep the room type's default occupancy, with no board.
- The group rate counts as a hand-set price, so only an admin can set or change it. Staff can still create events, sittings and suite bookings, and per-head (package) suite pricing is open to staff. A *manual* suite price is admin only.
- Packages are covered by per-head suite pricing (e.g. day-delegate rate) plus the block's group rate; no separate package object for now.

## Status (2026-11-21)
**Done:**
- core: `suites.rs`, `events.rs`, room-block planner (`plan_room_block`, `pick_block_rooms`) and the `EventDetail` / `EventListItem` / `SuiteBookingEntry` views. 81 core tests.
- store: migration `0004`, `SuiteRepo` (rate cards, suite bookings with price snapshot, buffer clash check and audit) and `EventRepo` (events with sittings, room-block sync, `release_due_blocks`), in `crates/store/src/events.rs`, with 6 integration tests.
- commands: `get/save_suite_rates`, `list/get/save_suite_booking`, `set_suite_booking_status`, `list/get/save_event`, with permission checks. Held rooms past their release date are released at app start.

**Screens (2026-11-21):**
- **Function suites** (main nav): day timeline per suite with setup and clear-down hatched, a week grid, and the day's hires in a table. Clicking a hire opens the suite booking form; clicking a suite name or a free day starts a new one.
- **Suite booking form** (`/suites/booking`): suite, optional event (fills in organiser and date), times (an end before the start means past midnight), layout with seats, attendees, buffers, pricing with a live price, organiser, status, notes, cancel.
- **Events** (main nav): list (with past events optional), event page (value, numbers, organiser, running order, suites, sittings with dietary lists, bedrooms with held rooms and release date), new/edit form with sittings and the room block. Staff see the group rate but can't change it.
- **Floors and layout:** a suite's hire rates and default buffers under its details.
- **Dashboard:** events in the next two weeks and a notice when held rooms are about to be released.
- **Demo data:** suite rates, a wedding (two suites, two sittings, a block of doubles), a conference and a club dinner. An existing demo hotel gets them from Events, "Load demo events".

**Left in this phase:**
1. Floor plan: show suite use at a chosen time.
2. Packages: decide whether per-head pricing plus the block's group rate is enough (see decisions above), then tick or build.
