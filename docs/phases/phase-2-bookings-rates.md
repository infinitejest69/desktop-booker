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
