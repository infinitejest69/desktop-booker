# Phase 1 — Property configuration & floor-plan editor

**Goal:** the hotel can describe itself (settings, floors, room types, rooms, suites) and lay out each floor visually.
**Rust concepts:** structs, enums, `Result`/`?`, `thiserror`, traits (repositories), `sqlx` migrations & queries, async basics.

## Tasks
- [ ] `core`: domain types — `Floor`, `Space { kind: Room | Suite }`, `RoomType`, `GridRect`, newtype IDs (UUID)
- [ ] `core`: validation (unique room numbers, grid rects inside bounds, no overlapping boxes on a floor)
- [ ] `store`: SQLite connection + embedded migrations; DB file in the OS app-data dir
- [ ] `store`: migrations for `property_settings`, `floors`, `room_types`, `spaces`
- [ ] `store`: repository traits + SQLite implementations
- [ ] `src-tauri`: CRUD commands + typed DTOs; generate TS types
- [ ] UI: app shell (sidebar nav, settings page)
- [ ] UI: property settings form (hotel name, address, currency, timezone)
- [ ] UI: floors list + room types CRUD
- [ ] UI: **grid editor** — add / drag / resize / snap / delete boxes, edit room number/type/capacity
- [ ] UI: suite properties (capacity per layout)
- [ ] UI: read-only floor view with status legend (placeholder statuses until Phase 2)
- [ ] Unit tests for `core` validation; repository tests against in-memory SQLite
- [ ] Seed/demo data command (e.g. 3 floors, 30 rooms, 2 suites)

## Acceptance
- Can configure a 100-room, 4-floor hotel. Layout survives a restart.
- Overlapping boxes and duplicate room numbers are rejected with a clear message.

## Notes / decisions
