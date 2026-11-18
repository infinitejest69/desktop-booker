# Phase 5 — Invoicing & payments

**Goal:** produce correct invoices and track what has been paid.
**Rust concepts:** precise money arithmetic, traits for document rendering, file I/O.

## Tasks
- [ ] `core`: invoice totals from room nights, board, suites, extras; tax per line (configurable rates, inclusive/exclusive)
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
