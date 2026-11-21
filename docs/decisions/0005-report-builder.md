# 0005: Custom reports and dashboards are built from a fixed set of measures and splits

**Status:** proposed (2026-11-21). Extends the Reports scope in REQUIREMENTS.md; settles how Phase 9 works.

## Context
Reports today are fixed: occupancy by night, revenue by month, ADR and RevPAR. Andy wants admins to build their own reports and dashboards. The app is offline and single-machine (ADR 0002), money is integer minor units, permissions are checked in Rust commands, and all data access goes through repository traits.

## Options
1. **Preset building blocks.** Rust offers a fixed list of *measures* (occupancy, revenue, ADR, arrivals…) that can each be *split* by a time period or a category (room type, board, source…). An admin picks measure + split + period + chart type and saves it under a name. A dashboard is a saved arrangement of saved reports.
2. **Raw SQL.** Users type a read-only query and the result is charted. Very flexible, but bypasses repository traits and permission checks, can expose any guest data, needs SQL knowledge, and breaks when the schema changes.
3. **External BI tool** (Metabase, Grafana) pointed at the SQLite file. Needs a separate server, which doesn't fit an offline desktop app, and bypasses permissions.

## Decision
Option 1, chosen by Andy on 2026-11-21.

- **Core (`crates/core/src/report_builder.rs`)**, pure and tested first: the `Measure`, `Split`, `Period` and `ChartKind` enums; a `ReportSpec` (name, measure, split, period, chart); which measure/split pairs are allowed; and `run(spec, facts) -> ReportResult`, which groups and totals plain *fact rows* (one per room-night, payment, suite hire and so on) that the store hands it. Money stays `Money`; percentages are per-mille like `DayOccupancy`.
- **Store:** a `ReportRepo` loads the fact rows for a date range and saves reports and dashboards (new migration: `saved_reports`, `dashboards`, `dashboard_items`, each with the usual UUID / `created_at` / `updated_at` / `updated_by`).
- **Commands:** `run_report`, `list/save/delete_saved_report`, `list/save/delete_dashboard`, all requiring `Permission::ViewReports` (admins).
- **Charts:** drawn by our own Svelte/SVG components (number tile, bar, line, table), like the existing occupancy bars. No chart library.
- The existing standard reports stay as they are. They can later be rebuilt as built-in saved reports.

## Consequences
- Adding a measure or split is a Rust change with a test, not something users can do themselves. That's the price of keeping it safe and correct.
- Fact rows are loaded per run and grouped in memory. That's fine for one hotel (a year is roughly 365 × rooms room-nights). Revisit if it gets slow.
- Dashboards are admin-only to start with. Showing a chosen dashboard to staff (for example on the home page) could come later behind its own permission.
- Saved reports store the spec, not the results, so they always show current data. A renamed room type or board shows its new name.
