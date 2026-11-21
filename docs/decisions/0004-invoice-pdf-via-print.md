# 0004: Invoices are HTML pages printed to PDF by the system

**Status:** accepted (2026-11-21). Supersedes nothing; settles the Phase 5 PDF spike.

## Context
Phase 5 needs invoices and receipts that can be printed or emailed as a PDF on Windows and Linux (and later Mac). The plan listed a spike: generate PDFs in Rust (`genpdf` / `printpdf`) or render an HTML page and use the system's print-to-PDF.

## Options
1. **Rust PDF crate** (`printpdf`, `genpdf`). Full control and a file without a dialog, but text layout, fonts (we use Atkinson Hyperlegible and Young Serif), wrapping, tables and currency formatting all have to be written by hand, and every layout change is a Rust change: a second copy of the invoice layout to keep in step with the screen.
2. **HTML invoice page + system print dialog.** The invoice page (`/invoices/[id]`) is already laid out with the app's fonts and formatting. `window.print()` opens the operating system's print dialog, which on Windows ("Microsoft Print to PDF"), Linux (GTK "Print to File" as PDF) and macOS ("Save as PDF") can save a PDF. Print CSS hides the app's menus.
3. **Headless browser to PDF** (Chromium `printToPDF`). Not available inside Tauri's system webview without shipping a browser.

## Decision
Option 2. An issued invoice is stored as a frozen copy (JSON) and shown on a printable page; staff press **Print or save as PDF** and choose a printer or PDF.

## Consequences
- No extra Rust dependencies; the printed invoice always matches what's on screen.
- Saving a PDF is one extra step in the print dialog. Emailing invoices (Phase 6) will need a PDF *file*; if the dialog step is a problem then, revisit option 1 for the email attachment only, reusing the stored invoice data.
- Print output varies a little between operating systems' print engines; the page uses plain black-on-white styling, simple tables and no background images, so differences are cosmetic.
