# Phase 8 — Carried over

**Goal:** finish the features left over from phases 4 to 7: things that need a decision, a real machine or account to check, or a new dependency.

Items move here from their phase (and are marked *moved to Phase 8* there), so each earlier phase can be closed. Supersedes nothing; linked from the roadmap.

## Tasks
### Rooms at a glance (added 2026-11-21)
- [x] One rooms page: the floor plan and the housekeeping cards are two views (Plan / Cards) of the same rooms; every room shows its availability and housekeeping state; selecting a room on the plan shows who's in it and lets you mark it dirty, clean or ready. The Housekeeping menu item goes (its link opens the Cards view).
- [x] Mark a whole floor clean (or ready) in one go
- [x] Rooms slept in last night (still in house, or checked out) are dirty each morning until marked clean that day
- [x] Dashboard: compact floor plans showing availability and dirty rooms at a glance, linking to the floor plan

### Bookings in one place (added 2026-11-21)
- [x] Rename the tariff chart to *Booking calendar*; one *Bookings* menu item with two views, Calendar and List
- [x] Reports admin only, in the side menu just above *My hotel*

### From Phase 4 (suites & events)
- [x] Floor plan: show suite use at a chosen time
- [x] Packages: decided (Andy, 2026-11-21): leave as it is; per-head suite pricing plus the room block's group rate cover packages for now

### From Phase 5 (invoicing)
- [ ] Print an invoice to PDF on Windows and on Linux and check the file opens (Phase 5 acceptance)

### From Phase 6 (operations, guests, email, reports)
- [x] SMTP settings; password stored in the OS keychain (`keyring`)
- [x] Email templates (confirmation, amendment, cancellation) with placeholders
- [x] Send confirmation from a booking; `email_log`; test-send button (acceptance: arrives through a real SMTP account; **still to check with Andy's Gmail app password**)
- [x] Duplicate guest merge
- [x] Save CSV and GDPR exports to a file (needs a save-file dialog, `tauri-plugin-dialog`); Phase 6 offers copy to clipboard
- [x] Report: ADR & RevPAR

### From Phase 7 (backups & packaging)
- [x] Clean install from the CI artefacts on Windows and Linux (Phase 7 acceptance)
- [ ] macOS .dmg checked on a Mac

## Notes / decisions
**Reports admin only (2026-11-21):** Andy asked for Reports to be admin only, next to My hotel. New `Permission::ViewReports` (admins only, like the other admin permissions); `occupancy_report` and the occupancy CSV now require it (revenue and ADR/RevPAR already needed `ViewAudit`). In the menu Reports sits on its own above My hotel, under one divider; staff opening `/reports` directly see a note instead. A report and dashboard *builder* followed: option 1 chosen, planned as Phase 9 ([ADR 0005](../decisions/0005-report-builder.md)).

**Booking calendar (2026-11-21):** Andy asked to rename the tariff chart to "Booking calendar", combined with or tabbed beside Bookings. Done like the floor plan's Plan / Cards: the menu has one *Bookings* item (opening the calendar), and both pages carry a Calendar / List switch (`BookingViews.svelte`). The routes stay `/chart` and `/bookings` so existing links keep working; the menu item is highlighted on either, and on a booking's own pages. Code names (`TariffChart.svelte`, `api.chart`) are unchanged.

**Email and saving files (2026-11-21):**
- Andy: "for now just use Gmail" for testing; MxToolbox can be the *recipient* of a test (its deliverability address replies with a report) but it isn't a server to send through.
- Core `email.rs`: settings (server, port, user name, from address and name), three templates (confirmation, changes, cancellation) with built-in defaults and `{placeholders}`, `fill`, `booking_values`, the log entry. Store: migration `0008` (`email_settings`, `email_templates`, `email_log`). Services: `booker_services::email` sends with `lettre` over rustls (STARTTLS on 587, TLS on 465) and keeps the password in the OS keychain with `keyring` 4 (service "DesktopBooker SMTP", the user name as account); the database never holds it. Gmail needs an app password; spaces in it are removed.
- Screens: My hotel → Email (Use Gmail, account, password saved or not, send a test, edit the three emails, recent emails); "Email the guest" on each booking (confirmation, changes, or cancellation for cancelled bookings) with that booking's sent and failed emails. Every attempt is logged, failures with the reason.
- Saving files: `tauri-plugin-dialog` 2 (Rust crate only, no npm package) opens the system save dialog from Rust; Reports has *Save as CSV…* and a guest's page *Save their data…* (JSON). Copy to clipboard stays too.

**More built (2026-11-21):**
- *Suite use on the floor plan:* a "Suites at" time on the floor plan (now, by default); a suite booked at that time turns heather and says what for; clear the time to see every suite booked that day. Clicking a suite lists its hires that day with links.
- *ADR and RevPAR:* `room_kpis` in core (room revenue ÷ room-nights sold, and ÷ room-nights available, rounded to the penny), shown on Reports for admins.
- *Merging guests:* `merged_guest` in core keeps the chosen record's details, fills its blanks from the duplicate and keeps both notes; the store moves bookings, suite hire and events across, hides the duplicate and logs it. Admin only, on the guest's page.

**Rooms at a glance, built (2026-11-21):** the Floor plan page is now *Floor plan and housekeeping*, with Plan and Cards views of the same rooms. On the plan, tiles keep their availability colour and get a *Dirty* or *Clean* marker; clicking a room opens a panel with its guests (booking, dates, board), Dirty / Clean / Ready and *Book this room*. Each floor has *Mark floor clean* and *Mark floor ready*. The counts include *To clean*. `/housekeeping` redirects to the Cards view and the Housekeeping menu item is gone; *Daily lists* moved to the floor plan toolbar. The dashboard has *The hotel today*: a small plan of each floor with a brass dot on rooms to clean. The morning rule is `housekeeping_today` in core: a room in `room_nights` for last night is dirty unless marked since the local day began.

- Created 2026-11-21 at Andy's request: "lets do phase 6 and 7 and push all the undone features in a phase 8".
- Andy (2026-11-21): the floor plan should give an overview of the hotel and room availability at a glance; he likes the housekeeping cards and wants them combined; whole floors can be marked clean; occupied or checked-out rooms become dirty automatically; the floor layout might suit the dashboard.
- Design: availability colours stay the tile's fill; housekeeping is a small marker on each tile (and the card's edge colour). "Dirty" is worked out each day: a room someone slept in last night is dirty unless it was marked since this morning, so stayovers need servicing daily without anyone setting it.
