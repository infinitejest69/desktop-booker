# Phase 2 — Room bookings & rates

**Goal:** take, change and cancel room bookings on a tariff chart with correct prices and no double-booking.
**Rust concepts:** ownership & borrowing in algorithms, iterators, `chrono` dates, generics, property testing (`proptest`), transactions.

## Tasks
- [x] `core`: `DateRange` (half-open `[check_in, check_out)`), `nights()` iterator, overlap rules
- [x] `core`: availability engine (TDD first: edge cases at same-day check-out/check-in)
- [x] `core`: rate engine: base per room type/night → seasonal rule → weekday rule → board basis per guest → manual override
- [x] `core`: money type (integer minor units), rounding rules
- [x] `proptest`: no two bookings ever share a room-night
- [x] `store`: migrations for `guests`, `bookings`, `booking_rooms`, `room_nights`, `rate_plans`, `rate_rules`, `board_options`
- [x] `store`: create booking in one transaction with a uniqueness guard on `(space_id, night)`
- [x] Booking statuses: provisional / confirmed / checked-in / checked-out / cancelled / no-show
- [x] UI: rates setup (room-type prices, seasons, weekend rules, board options)
- [x] UI: **tariff chart** (rooms × dates, scrollable, colour by status, click to create)
- [x] UI: booking form (guest lookup/create, rooms, dates, adults/children, board, price breakdown, override)
- [x] UI: booking detail, edit, move room, cancel
- [x] UI: check-in / check-out actions
- [x] Floor-plan view shows real statuses for a chosen date
- [x] Basic auto-backup on app close (pulled forward from Phase 7)
- [x] Search bookings by guest name / reference / date

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

**Built (2026-11-21):**
- `booker-core`: `Money`, `DateRange`, rate engine (`quote_stay`), bookings and statuses, `validate_booking`, `free_rooms`; property test that no accepted bookings share a room-night.
- `booker-store`: migration `0002_bookings.sql`; `RateRepo`, `GuestRepo`, `BookingRepo`; per-year reference counter; `room_nights` unique index as the database's own double-booking guard.
- **Price snapshots:** when a booking is edited or its status changes, rooms whose room, dates, guests and board are unchanged keep their saved nightly prices. Changed rooms are re-priced at today's rates.
- **Backup on exit:** the app copies the database to `<app data>/backups/booker-<timestamp>.db` with `VACUUM INTO`, keeping the newest 10. (`VACUUM INTO` from an in-memory database writes to memory, which is why the backup test uses a file database.)
- **UI:** Rates page (rules + board options), prices on Room types, check-in/out times on Hotel details, tariff chart (custom ui-sdk grid primitive on `createGridFocus`: one tab stop, arrow keys, Enter/click to open or book), booking form (guest search, multiple rooms, live price breakdown, price override), booking page with status actions, Bookings search, floor plan coloured by the chosen day's arrivals / in-house / departures.
- **Tests:** 81 Rust, 21 Vitest, 14 Playwright.
- **Not checked yet:** the tariff chart's speed at 100 rooms × 60 days (the demo has 40 rooms; the chart shows 21 days at a time).
- **Existing databases:** upgrading a Phase 1 database leaves room types at £0 and adds no board options. Set prices on Room types and Rates, or start from a fresh demo hotel.
- **Fuller demo hotel (at Andy's request):** about 60 generated bookings from two weeks ago to six weeks ahead, with realistic statuses (checked out, in house, arriving, confirmed, some provisional, cancelled and no-show), families, two-room groups, mixed board and guest notes, plus four rate rules (weekend, summer, Christmas, winter midweek offer). A fixed-seed generator (`DemoRng` in `seed.rs`) makes it identical every time.
- **Navigation (from Andy's feedback, 2026-11-21):** Back/Forward in a top bar (also Alt+←/→ and mouse side buttons), breadcrumbs on booking pages, and the chart's week and open booking kept in the URL so Back returns you to exactly where you were. Clicking a booking on the chart opens a **side panel** (non-modal ui-sdk Drawer) with status actions, Edit and "Open full page". Saving a new booking from the chart returns to the chart with the booking highlighted and a toast with "Open booking". Focus stays in the panel's actions after a status change.
- SvelteKit 3 renamed `goto` options: `replaceState` → `replace`, and `keepFocus`/`noScroll` → `reset: false`.
- **Dashboard and menus (from Andy's feedback, 2026-11-21):** the home page is now a front-desk **Dashboard** for today: counts (arriving, departing, in house, free tonight, occupancy), Arrivals and Departures with one-click Check in / Check out, In house, a two-week tariff chart and the next upcoming bookings. The floor plan moved to `/floor-plan`. The sidebar has day-to-day pages at the top and a collapsible **My Hotel** group for setup (Hotel details, Floors, Room types, Rates, Layout editor), which opens automatically on those pages. The chart grid is now a shared `TariffChart` component used by both the chart page and the dashboard.
- **Chart sizing (from Andy's feedback):** the tariff chart now measures its width and shows as many days as fit (minimum 7, maximum 62), with columns stretching to fill. Bars are sized relative to their cell with `box-sizing: border-box`, so stays that run past either edge stop exactly at it (before, padding was added on top of the width and bars overflowed).
- **Design pass (2026-11-21):** see [ADR 0003](../decisions/0003-visual-design.md). Slate/heather palette with shared status colours, Atkinson Hyperlegible Next + Young Serif (bundled), room numbers as brass key fobs, the dashboard headline as a sentence, the hotel name as the sidebar sign, and no " · " separators or all-caps labels.
