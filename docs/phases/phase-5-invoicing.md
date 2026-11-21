# Phase 5 — Invoicing & payments

**Goal:** produce correct invoices and track what has been paid.
**Rust concepts:** precise money arithmetic, traits for document rendering, file I/O.

## Tasks
- [x] `core`: invoice totals from room nights, board, suites, extras; tax per line (configurable rates, inclusive/exclusive)
- [x] `store`: migrations for `tax_rates`, `invoice_lines`, `payments`, invoice numbering
- [x] Sequential, gap-free invoice numbers
- [x] Extras/charges on a booking (bar, restaurant, misc.)
- [x] Deposits and part payments; payment methods (cash/card/bank transfer) recorded only
- [x] Balance due, overpayment/refund handling
- [x] **Spike:** PDF in Rust (`genpdf`/`printpdf`) vs HTML → print-to-PDF; record the choice as an ADR
- [ ] PDF invoice + receipt with hotel logo/details
- [x] Settings: currency, tax rates, invoice footer/bank details
- [x] UI: booking folio (charges, payments, balance), issue invoice
- [x] Tests: tax rounding, mixed tax rates, deposits

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

**Store and commands (2026-11-21):**
- Migration `0005`: `tax_rates`, `invoice_settings` (one row: prices include tax, prefix, footer, a tax rate per kind of charge), `charges` (extras only), `payments`, `invoices` (frozen JSON copy) and `invoice_counter`.
- A folio belongs to a bedroom booking, a stand-alone suite booking or an event (`FolioOwner`). Stay charges (rooms grouped by nightly price, board on its own line, a hand-set price as one "agreed price" line) and suite charges are worked out from the saved prices each time (`stay_charges`, `suite_charges` in `crates/core/src/folio.rs`), so they always match the booking.
- Suite hire that's part of an event is billed on the event's folio, not its own. Bedrooms are billed on each booking's folio.
- Issuing an invoice takes the next number inside the same transaction and stores the whole invoice as JSON, so it never changes afterwards; a later extra goes on the next invoice. An empty folio can't be invoiced (and uses no number).
- Commands: `list/save_tax_rate`, `get/save_invoice_settings` (admin), `get_folio`, `add_charge`, `add_payment`, `issue_invoice` (staff), `remove_charge` (cancel permission), `get_invoice`.

**UI (2026-11-21):**
- **Bill** panel on the booking and event pages: worked-out lines plus extras (removable), VAT per rate, paid, balance or credit; add an extra; record a payment, deposit or refund; issue an invoice; links to issued invoices.
- **Invoice page** (`/invoices/[id]`): hotel details, bill-to, lines, VAT summary, payments, balance and footer, black on white; **Print or save as PDF** uses the system print dialog ([ADR 0004](../decisions/0004-invoice-pdf-via-print.md)). Print CSS hides the app's menus.
- **My hotel → Invoices and tax:** add tax rates, choose the rate for each kind of charge, prices include tax or not, number prefix, footer.
- **Demo data:** Standard (20%), Reduced (5%) and Zero rated; 20% on everything; footer with VAT number and bank details; bar and dinner charges on a few in-house guests' bills.

## Status at pause (2026-11-21)
**Left in this phase:**
1. Check printing to PDF on Windows and Linux (the acceptance test), then tick "PDF invoice + receipt".
2. A quick look at the bill panel and invoice page in the real app.

**Done since the pause:** Playwright tests (`e2e/invoicing.e2e.ts`) for the bill (stay with VAT, extras, deposit, refund limit), issuing a numbered invoice and its printable page, and the Invoices and tax settings; the fake backend now mirrors the store's folio and invoices.

