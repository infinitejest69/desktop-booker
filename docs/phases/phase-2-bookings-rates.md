# Phase 2 — Room bookings & rates

**Goal:** take, change and cancel room bookings on a tariff chart with correct prices and no double-booking.
**Rust concepts:** ownership & borrowing in algorithms, iterators, `chrono` dates, generics, property testing (`proptest`), transactions.

## Tasks
- [ ] `core`: `DateRange` (half-open `[check_in, check_out)`), `nights()` iterator, overlap rules
- [ ] `core`: availability engine (TDD first: edge cases at same-day check-out/check-in)
- [ ] `core`: rate engine: base per room type/night → seasonal rule → weekday rule → board basis per guest → manual override
- [ ] `core`: money type (integer minor units), rounding rules
- [ ] `proptest`: no two bookings ever share a room-night
- [ ] `store`: migrations for `guests`, `bookings`, `booking_rooms`, `room_nights`, `rate_plans`, `rate_rules`, `board_options`
- [ ] `store`: create booking in one transaction with a uniqueness guard on `(space_id, night)`
- [ ] Booking statuses: provisional / confirmed / checked-in / checked-out / cancelled / no-show
- [ ] UI: rates setup (room-type prices, seasons, weekend rules, board options)
- [ ] UI: **tariff chart** (rooms × dates, scrollable, colour by status, click to create)
- [ ] UI: booking form (guest lookup/create, rooms, dates, adults/children, board, price breakdown, override)
- [ ] UI: booking detail, edit, move room, cancel
- [ ] UI: check-in / check-out actions
- [ ] Floor-plan view shows real statuses for a chosen date
- [ ] Basic auto-backup on app close (pulled forward from Phase 7)
- [ ] Search bookings by guest name / reference / date

## Acceptance
- An overlapping booking is rejected; same-day turnover is allowed.
- A stay across a season boundary prices each night correctly.
- The tariff chart stays smooth with 100 rooms × 60 days.

## Notes / decisions
**Decisions from Andy (2026-11-21):**
- Pricing is **per room per night** (not per person).
- Bookings record **adults and children** separately. Both count towards max occupancy. Optional per-night supplements: extra adult (above the room type's usual guests) and per child; board options have adult and child prices.
- Check-in **15:00**, check-out **11:00**, configurable in Hotel details. Stays are counted in nights, half-open `[check_in, check_out)`.
- Booking references are **year + number**, e.g. `2026-0001`, restarting each year.

**Design:**
- Rates: base nightly rate per room type → **rate rules** (date range, optional weekdays, optional room type, priority) applied in priority order. Each rule either *sets* the price, *adjusts* it by an amount, or applies a *percentage*. A weekend rate is just a rule for Fri+Sat.
- Board options (RO / BB / DBB…) with adult and child prices per night; chosen per booked room.
- Nightly prices are **snapshotted** into `room_nights` when booked, so rate changes don't alter existing bookings. A manual override is a total for that room's stay.
- Double-booking guard: availability is checked in `booker-core`, then the insert runs in one transaction with a unique index on `(space_id, night)`. Cancelling deletes the booking's nights, freeing them; the booking row stays for history.
- Status lives on the booking: provisional → confirmed → checked-in → checked-out, or cancelled / no-show.
- Money is `i64` minor units everywhere (ts-rs emits it as `number`).
