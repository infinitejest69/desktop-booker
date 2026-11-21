# Phase 5 — Invoicing & payments

**Goal:** produce correct invoices and track what has been paid.
**Rust concepts:** precise money arithmetic, traits for document rendering, file I/O.

## Tasks
- [x] `core`: invoice totals from room nights, board, suites, extras; tax per line (configurable rates, inclusive/exclusive)
- [ ] `store`: migrations for `tax_rates`, `invoice_lines`, `payments`, invoice numbering
- [ ] Sequential, gap-free invoice numbers
- [ ] Extras/charges on a booking (bar, restaurant, misc.)
- [ ] Deposits and part payments; payment methods (cash/card/bank transfer) recorded only
- [ ] Balance due, overpayment/refund handling
- [ ] **Spike:** PDF in Rust (`genpdf`/`printpdf`) vs HTML → print-to-PDF; record the choice as an ADR
- [ ] PDF invoice + receipt with hotel logo/details
- [ ] Settings: currency, tax rates, invoice footer/bank details
- [ ] UI: booking folio (charges, payments, balance), issue invoice
- [ ] Tests: tax rounding, mixed tax rates, deposits

## Acceptance
- Invoice totals match a hand-calculated example to the penny.
- A PDF opens correctly on Windows and Linux.

## Notes / decisions
**Core (2026-11-21):** `crates/core/src/invoices.rs`.
- A *folio* is a booking's running account: `Charge` lines (category, quantity, unit price, tax rate copied onto the line) and `Payment`s (method, deposit flag; negative amount = refund).
- Tax rates are stored in basis points (2000 = 20%). Tax is worked out per line and rounded to the penny (halves away from zero), then summed, so printed lines always add up. A hotel-wide setting will say whether prices include tax (UK default) or have it added.
- `folio_totals` gives net, tax, gross, a per-rate tax summary, deposits, paid and balance (negative = refund owed). A refund can't exceed what's been paid.
- Invoice numbers: `format_invoice_number(prefix, n)` (e.g. INV-000042); the store will take `n` from a counter in the same transaction, so there are no gaps.
- The acceptance example (2 nights, breakfast, a zero-rated newspaper, deposit and part payment) is a unit test and matches to the penny.

**Next:** store migration (`tax_rates`, `charges`, `payments`, `invoices`, counter, `prices_include_tax` setting); room nights, board and suites become charges on the folio; commands; folio UI; PDF spike and ADR.

