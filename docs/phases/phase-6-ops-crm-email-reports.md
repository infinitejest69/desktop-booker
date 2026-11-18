# Phase 6 — Housekeeping, guest CRM, email & reports

**Goal:** day-to-day operations, guest knowledge, outbound confirmations and business insight.
**Rust concepts:** async I/O (`lettre`), templating, aggregation queries, the `csv` crate.

## Tasks
- [ ] Housekeeping status per room (clean / dirty / inspected / out of service); auto-dirty on check-out
- [ ] UI: housekeeping board (also on the floor plan)
- [ ] Daily lists: arrivals, departures, in-house (printable)
- [ ] Guest profile: contact, notes, preferences, stay history, total spend
- [ ] Duplicate guest merge
- [ ] GDPR: export a guest's data (JSON/CSV), erase/anonymise, retention setting
- [ ] SMTP settings; password stored in OS keychain (`keyring`)
- [ ] Email templates (confirmation, amendment, cancellation) with placeholders
- [ ] Send confirmation from a booking; `email_log`; test-send button
- [ ] Report: occupancy % by day/month
- [ ] Report: revenue by month and by category (rooms / suites / extras)
- [ ] Report: ADR & RevPAR (nice to have)
- [ ] CSV export for reports and guest lists
- [ ] Tests for report aggregates

## Acceptance
- A confirmation email arrives through a real SMTP account.
- Occupancy for a seeded month matches a manual calculation.

## Notes / decisions
