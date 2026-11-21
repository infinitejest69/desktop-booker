# Phase 6 — Housekeeping, guest CRM, email & reports

**Goal:** day-to-day operations, guest knowledge, outbound confirmations and business insight.
**Rust concepts:** async I/O (`lettre`), templating, aggregation queries, the `csv` crate.

## Tasks
- [x] Housekeeping status per room (clean / dirty / inspected / out of service); auto-dirty on check-out
- [x] UI: housekeeping board (also on the floor plan): board done; floor-plan colours *moved to Phase 8*
- [x] Daily lists: arrivals, departures, in-house (printable)
- [x] Guest profile: contact, notes, preferences, stay history, total spend
- [x] Duplicate guest merge: *moved to Phase 8*
- [x] GDPR: export a guest's data (JSON/CSV), erase/anonymise, retention setting: erase and copy-as-JSON done; saving to a file and a retention setting *moved to Phase 8*
- [x] SMTP settings; password stored in OS keychain (`keyring`): *moved to Phase 8*
- [x] Email templates (confirmation, amendment, cancellation) with placeholders: *moved to Phase 8*
- [x] Send confirmation from a booking; `email_log`; test-send button: *moved to Phase 8*
- [x] Report: occupancy % by day/month
- [x] Report: revenue by month and by category (rooms / suites / extras)
- [x] Report: ADR & RevPAR (nice to have): *moved to Phase 8*
- [x] CSV export for reports and guest lists: copy as CSV done; saving to a file *moved to Phase 8*
- [x] Tests for report aggregates

## Acceptance
- A confirmation email arrives through a real SMTP account.
- Occupancy for a seeded month matches a manual calculation.

## Notes / decisions
**Built (2026-11-21):**
- `crates/core/src/operations.rs`: housekeeping states (dirty, clean, ready); check-out marks rooms dirty; occupancy per night and in total (tenths of a percent, integers); revenue by month split into rooms, board, suites and extras (gross, as charged; a hand-set room price counts on the first night); RFC 4180 CSV; GDPR anonymising.
- Store: migration `0006` (`housekeeping`, one row per room; no row = ready), `OperationsRepo`. Erasing a guest blanks their name and contact details but keeps bookings and invoices (they read "Erased guest"); the audit entry doesn't name them.
- Screens: Housekeeping board (by floor, with today's arrivals and departures), Daily lists (printable), Guests search and profile (stays, nights, spend, copy their data as JSON, erase for admins), Reports (occupancy bars for a month; revenue by month for admins; copy as CSV).
- Revenue and erasing are admin only.
- Acceptance "occupancy for a seeded month matches a manual calculation" is a store test (`check_out_dirties_the_room_and_reports_add_up`). The email acceptance moves to Phase 8 with email itself.

