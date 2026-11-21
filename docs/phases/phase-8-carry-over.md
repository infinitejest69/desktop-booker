# Phase 8 — Carried over

**Goal:** finish the features left over from phases 4 to 7: things that need a decision, a real machine or account to check, or a new dependency.

Items move here from their phase (and are marked *moved to Phase 8* there), so each earlier phase can be closed. Supersedes nothing; linked from the roadmap.

## Tasks
### From Phase 4 (suites & events)
- [ ] Floor plan: show suite use at a chosen time
- [ ] Packages: decide whether per-head suite pricing plus the room block's group rate is enough, or build a package item (e.g. "wedding package" at one price)

### From Phase 5 (invoicing)
- [ ] Print an invoice to PDF on Windows and on Linux and check the file opens (Phase 5 acceptance)

### From Phase 6 (operations, guests, email, reports)
- [ ] SMTP settings; password stored in the OS keychain (`keyring`)
- [ ] Email templates (confirmation, amendment, cancellation) with placeholders
- [ ] Send confirmation from a booking; `email_log`; test-send button (acceptance: arrives through a real SMTP account)
- [ ] Duplicate guest merge
- [ ] Save CSV and GDPR exports to a file (needs a save-file dialog, `tauri-plugin-dialog`); Phase 6 offers copy to clipboard
- [ ] Housekeeping status colours on the floor plan
- [ ] Report: ADR & RevPAR

### From Phase 7 (backups & packaging)
- [ ] Clean install from the CI artefacts on Windows and Linux (Phase 7 acceptance)
- [ ] macOS .dmg checked on a Mac

## Notes / decisions
- Created 2026-11-21 at Andy's request: "lets do phase 6 and 7 and push all the undone features in a phase 8".
