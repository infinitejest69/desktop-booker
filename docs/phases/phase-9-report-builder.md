# Phase 9 — Report and dashboard builder

**Goal:** admins can build, save and arrange their own reports and dashboards from a set of measures and splits ([ADR 0005](../decisions/0005-report-builder.md)).
**Rust concepts:** enums with data, `match` to check that a combination is allowed, `HashMap`/`BTreeMap` grouping, iterator folds, `serde` for saved specs.

## Measures, splits and periods
Agreed by Andy on 2026-11-22: all 24 measures as listed.

### Measures
| Area | Measure | Shown as |
|---|---|---|
| Rooms | Room-nights sold | count |
| Rooms | Occupancy | % |
| Rooms | Room revenue | £ |
| Rooms | ADR (room revenue ÷ room-nights sold) | £ |
| Rooms | RevPAR (room revenue ÷ room-nights available) | £ |
| Rooms | Guests staying (adults + children per night) | count |
| Bookings | Arrivals | count |
| Bookings | Departures | count |
| Bookings | Bookings made (by the day they were made) | count |
| Bookings | Cancellations | count |
| Bookings | No-shows | count |
| Bookings | Average length of stay | nights |
| Bookings | Average lead time (days booked ahead) | days |
| Money | Total revenue (rooms, board, suites, extras) | £ |
| Money | Board revenue | £ |
| Money | Extras revenue | £ |
| Money | Payments taken | £ |
| Money | Deposits taken | £ |
| Suites & events | Suite hours booked | hours |
| Suites & events | Suite revenue | £ |
| Suites & events | Events | count |
| Suites & events | Event attendees | count |
| Guests | New guests | count |
| Guests | Returning guests (stayed before) | count |

### Splits (group by)
- **Time:** day, week, month, year
- **Rooms:** room type, floor, room, board
- **Bookings:** source (phone, email, walk-in, website, agent, other), status
- **Money:** revenue type (rooms / board / suites / extras), payment method
- **Suites & events:** suite, event kind
- **None:** a single total (for number tiles)

Core holds a table of which splits make sense for each measure. For example, payments can be split by method but not by board, and occupancy can't be split by source.

### Periods
Fixed dates, or relative: today, this week, this month, last month, this year, last year, last 12 months, next 7 / 30 / 90 days (for arrivals and other forward-looking measures).

### Chart types
Number tile, bar chart, line chart (time splits only), table. Every report can also be saved as CSV.

## Tasks
- [x] Andy checks the measures, splits and periods above (2026-11-22: all 24 as listed)
- [x] ADR 0005 marked accepted
- [x] Core: `Measure`, `Split`, `Period`, `ChartKind`, `ReportSpec` + the allowed-combinations table (tests first)
- [x] Core: fact rows (room-night, booking, payment, suite hire, guest) and `run` that groups and totals them (tests first, including money rounding and empty periods)
- [x] Core: relative periods → actual date ranges (tests first)
- [x] Store: migration for `saved_reports`, `dashboards`, `dashboard_items`; `ReportRepo` with fact loading and save/list/delete
- [x] Commands: `run_report`, saved reports, dashboards, all `ViewReports`; `report_csv` accepts a spec
- [x] UI: chart components (number, bar, line, table) in `src/lib/reports/`
- [x] UI: report builder (pick measure → split → period → chart, with a live preview); save under a name
- [x] UI: dashboards (choose saved reports, order them, size each one: small / wide); Reports page gets Standard / My reports / Dashboards views
- [x] Fake backend + e2e: build and save a report, put it on a dashboard, staff refused
- [x] Docs: REQUIREMENTS.md Reports row updated

## Acceptance
- An admin builds "Room revenue by room type, last month, bar chart", saves it, and sees it again after restarting the app.
- A dashboard with three saved reports shows current figures. Figures match the standard reports for the same period.
- A staff member can't open the builder or dashboards, and calling the commands directly is refused.

## Notes / decisions
- Created 2026-11-21: Andy chose option 1 (preset measures and splits) over raw SQL or an external BI tool. See ADR 0005.

**Built (2026-11-22):**
- Core `report_builder.rs`: `Measure` (24), `Split` (15), `Period` (fixed or relative: today, this/last week-month-year, last 12 months, next 7/30/90 days), `ChartKind`, `ReportSpec`, `SavedReport`, `Dashboard` (up to 12 reports, none twice). `allows` decides which splits suit which measure and `catalogue` lists them for the screen, so the rule lives only in Rust. `run` turns *fact rows* from the store into groups. Rates and averages keep a top and a bottom per group, so totals are worked out over everything, never by adding rows up. Time splits show empty days/weeks/months as zero. `report_csv` writes any report for spreadsheets.
- How each measure is worked out:
  - *Room measures* (room-nights, occupancy, room/board revenue, ADR, RevPAR, guests staying) come from each booked night. A hand-set price counts in full on the first night, as in the revenue report. Rooms available = in-service rooms × days.
  - *Arrivals / departures* count booked rooms by date; *bookings made* counts bookings by the (UTC) day they were saved; *cancellations / no-shows* count bookings by arrival date; *average stay / lead time* are per booked room arriving, in tenths.
  - *New guests* are arriving for the first time ever; *returning guests* had an earlier stay (cancelled and no-show bookings don't count as stays).
  - *Money:* total revenue splits into rooms, board, suites and extras like the revenue report; extras = bill charges + parking; payments and deposits by the day taken (refunds negative).
  - *Suites:* suite hours and revenue cover hires that weren't cancelled, on the day they start. *Events* are counted by start date; attendees = the largest catering sitting, or the largest suite hire if it has no sittings.
- Store: migration `0011_reports.sql` (`saved_reports` with the spec as JSON, `dashboards`, `dashboard_items`; deleting a report takes it off dashboards), `ReportRepo` with `report_facts` and save/list/delete; every change in the audit log. A store test checks the figures match the standard occupancy and revenue reports on the demo hotel.
- Commands (all `ViewReports`, admins): `report_catalogue`, `run_report`, `custom_report_csv`, `save_custom_report_csv`, saved reports and dashboards.
- Screens: Reports has three views (*Standard*, *Build a report*, *Dashboards*).
  - *Builder:* name, measure (grouped), split (only those that measure allows), period, chart, a live preview, CSV, and the saved list. The name follows the measure and split until one is typed.
  - *Dashboards:* choose or make one; add saved reports, order them, make any full width.
  - *Charts:* `src/lib/reports/ReportChart.svelte`, drawn in-house (no chart library) in the app's heather: columns over time with a hover tooltip, bars with values at the tip for other splits, a 2px line with a crosshair, one large figure, or a table; every chart also has its figures in a table.
