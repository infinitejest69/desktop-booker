# Phase 9 — Report and dashboard builder

**Goal:** admins can build, save and arrange their own reports and dashboards from a set of measures and splits ([ADR 0005](../decisions/0005-report-builder.md)).
**Rust concepts:** enums with data, `match` to check that a combination is allowed, `HashMap`/`BTreeMap` grouping, iterator folds, `serde` for saved specs.

## Before starting: Andy to check
The lists below are a proposal. Strike out, add or rename anything before any code is written.

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
- [ ] Andy checks the measures, splits and periods above
- [ ] ADR 0005 marked accepted
- [ ] Core: `Measure`, `Split`, `Period`, `ChartKind`, `ReportSpec` + the allowed-combinations table (tests first)
- [ ] Core: fact rows (room-night, booking, payment, suite hire, guest) and `run` that groups and totals them (tests first, including money rounding and empty periods)
- [ ] Core: relative periods → actual date ranges (tests first)
- [ ] Store: migration for `saved_reports`, `dashboards`, `dashboard_items`; `ReportRepo` with fact loading and save/list/delete
- [ ] Commands: `run_report`, saved reports, dashboards, all `ViewReports`; `report_csv` accepts a spec
- [ ] UI: chart components (number, bar, line, table) in `src/lib/reports/`
- [ ] UI: report builder (pick measure → split → period → chart, with a live preview); save under a name
- [ ] UI: dashboards (choose saved reports, order them, size each one: small / wide); Reports page gets Standard / My reports / Dashboards views
- [ ] Fake backend + e2e: build and save a report, put it on a dashboard, staff refused
- [ ] Docs: REQUIREMENTS.md Reports row updated

## Acceptance
- An admin builds "Room revenue by room type, last month, bar chart", saves it, and sees it again after restarting the app.
- A dashboard with three saved reports shows current figures. Figures match the standard reports for the same period.
- A staff member can't open the builder or dashboards, and calling the commands directly is refused.

## Notes / decisions
- Created 2026-11-21: Andy chose option 1 (preset measures and splits) over raw SQL or an external BI tool. See ADR 0005.
