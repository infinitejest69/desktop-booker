# Phase 4 — Function suites & events

**Goal:** book function suites by time and group everything for a wedding or dinner under one event.
**Rust concepts:** more complex domain modelling, enums with data, time ranges with `chrono::NaiveDateTime`.

## Tasks
- [x] `core`: time-slot availability with setup/teardown buffers
- [x] `core`: hire types: hourly / half-day / day, with pricing
- [x] `store`: migrations for `suite_bookings`, `events`, event links
- [ ] Suite booking: layout, attendees (≤ capacity for that layout), notes, catering requirements
- [ ] Event container: name, type (wedding/conference/dinner…), contact, date(s)
- [ ] Link suite bookings + room block (held rooms with release date) to an event
- [ ] Packages (e.g. wedding package: suite + per-head catering + room block rate)
- [ ] Catering numbers / dietary summary per event
- [ ] UI: suite calendar (day/week timeline per suite)
- [ ] UI: event page bringing everything together
- [ ] Floor-plan view shows suite occupancy for a chosen time
- [ ] Tests: buffer overlaps, capacity per layout, room-block release

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

## Status at pause (2026-11-21)
**Done:** core rules for suites and events (`suites.rs`, `events.rs`, 11 tests) and migration `0004_suites_events.sql` (applies cleanly; all store tests pass).

**Next, in order:**
1. **Store:** suite rate card get/save (columns on `spaces`); `SuiteBookingRepo` (list by suite and time range, save with `validate_suite_booking` + price snapshot + audit, set status); `EventRepo` (list, get with sittings, suite bookings and block bookings, save). Room block: on save, create or adjust provisional bookings linked by `bookings.event_id` at the group rate; `release_due_blocks(today)` cancels unclaimed ones (run at startup).
2. **Commands** with permissions (`MakeBookings` / `CancelBookings`; hand-set `Manual` price needs `OverridePrices`). Then `pnpm gen:types`.
3. **UI:** suite rates on the suite panel in Floors and layout; suite timeline (day/week); suite booking form; Events list + event page (suites, sittings with dietary summary, room block, total value); floor plan suite use at a chosen time; dashboard "blocks due for release".
4. **Fake backend + Playwright tests;** demo data with a wedding.
