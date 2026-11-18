# DesktopBooker — Requirements

Offline booking app for a small family-run hotel: rooms, function suites, events, invoicing, housekeeping, guests and reports.

## Constraints
- **No hosting, no running cost.** All data lives on the hotel's own machine(s).
- Desktop-first: Windows + Linux, macOS if possible. Android later.
- Written in **Rust** (a learning goal) using **Tauri 2**.
- Scale: up to ~100 rooms over multiple floors, plus function suites. Everything is configurable, nothing hard-coded.

## Functional requirements (v1)
| Area | Requirement |
|---|---|
| Property setup | Configurable floors, rooms, room types, function suites (capacity, layouts). |
| Floor plan | Built-in **grid editor** per floor: drag and resize room/suite boxes. A live view colours each one by status for a chosen date (free / booked / arriving / departing / dirty / out of service). |
| Room bookings | Tariff chart (rooms × dates grid), create/edit/cancel, check-in/out, multi-room bookings, **no double-booking**. |
| Rates | Per room type per night; seasonal and weekend date-range rates; board basis (Room only / B&B / DBB per guest); manual price override per booking. |
| Function suites | Hourly / half-day / day hire, layout (theatre / boardroom / banquet), attendee count, setup and teardown buffers. |
| Events | Weddings and dinners: links suite bookings, catering numbers and a block of rooms to one event; packages. |
| Invoicing | Deposits, part payments, balance, payment method (recorded only, no card processing), PDF invoice/receipt, configurable currency and tax rates. |
| Guests (CRM) | Guest records, stay history, notes and preferences, GDPR export/erase. |
| Housekeeping | Clean / dirty / inspected per room; daily arrivals, departures and in-house lists. |
| Email | Confirmation emails through the hotel's own SMTP account. |
| Reports | Occupancy %, revenue by month and by category, CSV export. |
| Users | Staff name + PIN, Admin / Staff roles, audit log of who changed what. |
| Backups | Automatic daily backup to a chosen folder (USB / OneDrive / Dropbox / NAS), rotation, one-click restore. |
| Channels | None. Bookings are entered by hand (OTA / iCal deferred). |
| Multi-device | **Undecided.** v1 runs on one PC; the design must allow LAN multi-PC and Android later. |

## Architecture
- **Tauri 2** app. Rust holds all business logic and data. Frontend: **Svelte 5 + TypeScript + Vite**. The floor plan and tariff chart are custom SVG / CSS-grid components.
- **Cargo workspace:**
  - `crates/core` — pure domain: types, availability/overlap rules, rate calculation, invoice totals. No I/O.
  - `crates/store` — SQLite via `sqlx` (embedded migrations), repositories, audit log, backup/restore.
  - `crates/services` — email (`lettre`), PDF, CSV (`csv`), PIN hashing (`argon2`), OS keychain secrets (`keyring`).
  - `src-tauri` — thin Tauri commands returning typed DTOs (`serde`; TS types via `specta`/`ts-rs`).
  - `ui/` — Svelte frontend.
- **Data rules**
  - UUID primary keys; `created_at` / `updated_at` / `updated_by` on every row; soft-delete.
  - Money stored as **integer minor units** (pence/cents).
  - Stays use **half-open date ranges** `[check_in, check_out)`.
  - The overlap check and insert run in one transaction, backed by a uniqueness guard on `room_nights`.
- **Multi-device path (not in v1):** all data access goes through `store` repository traits. A later LAN mode embeds an `axum` server in a "host" PC; other PCs and Android connect to it as clients. **Never** share the live SQLite file over a network/cloud folder, because that corrupts it. Only backups go there.

## Core data model (first pass)
`property_settings`, `users`, `audit_log`, `floors`, `spaces` (kind: room|suite, grid x/y/w/h, floor_id), `room_types`, `rate_plans`, `rate_rules` (date range, weekdays, price), `board_options`, `guests`, `bookings`, `booking_rooms`, `room_nights`, `suite_bookings` (start/end datetime, layout, attendees), `events`, `invoice_lines`, `payments`, `tax_rates`, `housekeeping_status`, `email_log`.

## Risks
| Risk | Likelihood | Mitigation |
|---|---|---|
| Multi-device need appears after v1 | High | Repository traits + UUIDs + timestamps from day one; LAN mode as its own phase. |
| Double-booking bugs | Medium | Overlap logic in pure `core` with unit + property tests (`proptest`); DB guard on `room_nights`. |
| Learning curve (Rust + Tauri + Svelte) | High | Logic in small pure crates; each phase lists the Rust concepts it teaches. |
| Data loss on a single PC | Medium | Basic auto-backup pulled forward into Phase 2; restore tested. |
| PDF generation in Rust is fiddly | Medium | Spike in Phase 5; fallback is HTML invoice → print-to-PDF. |
| GDPR (guest personal data) | Medium | Data stays local; keychain for secrets; export/erase tools; retention setting. |
| Android via Tauri mobile is less mature | Medium | Deferred; responsive UI from the start. |
