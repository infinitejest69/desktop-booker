# Phase 1 — Property configuration & floor-plan editor

**Goal:** the hotel can describe itself (settings, floors, room types, rooms, suites) and lay out each floor visually.
**Rust concepts:** structs, enums, `Result`/`?`, `thiserror`, traits (repositories), `sqlx` migrations & queries, async basics.

## Tasks
- [x] `core`: domain types — `Floor`, `Space { kind: Room | Suite }`, `RoomType`, `GridRect`, newtype IDs (UUID)
- [x] `core`: validation (unique room numbers, grid rects inside bounds, no overlapping boxes on a floor)
- [x] `store`: SQLite connection + embedded migrations; DB file in the OS app-data dir
- [x] `store`: migrations for `property_settings`, `floors`, `room_types`, `spaces`
- [x] `store`: repository traits + SQLite implementations
- [x] `src-tauri`: CRUD commands + typed DTOs; generate TS types
- [x] UI: app shell (sidebar nav, settings page)
- [x] UI: property settings form (hotel name, address, currency, timezone)
- [x] UI: floors list + room types CRUD
- [x] UI: **grid editor** — add / drag / resize / snap / delete boxes, edit room number/type/capacity
- [x] UI: suite properties (capacity per layout)
- [x] UI: read-only floor view with status legend (placeholder statuses until Phase 2)
- [x] Unit tests for `core` validation; repository tests against in-memory SQLite
- [x] Seed/demo data command (e.g. 3 floors, 30 rooms, 2 suites)

## Acceptance
- Can configure a 100-room, 4-floor hotel. Layout survives a restart.
- Overlapping boxes and duplicate room numbers are rejected with a clear message.

## Notes / decisions
- **Status (2026-11-21):** all tasks built and tested. 41 Rust tests (core rules + store against in-memory and on-disk SQLite), 11 Vitest, 7 Playwright browser tests. Still needed: a hands-on check by Andy in the real app (`pnpm tauri dev`) against the real database before marking the phase done.
- **sqlx with runtime-checked queries**, not the `query!` macros: no database needed at compile time (simpler CI/offline builds). sqlx 0.9 only accepts SQL known at compile time, so shared SQL fragments are joined with `concat!` and a small macro rather than `format!`.
- **IDs stored as TEXT** (readable in any SQLite browser). Rows are read into private `*Row` structs and converted with `TryFrom`, giving `StoreError::Corrupt` on bad data instead of a panic.
- **`property_settings` is a singleton** (`id = 1`), the one table without a UUID key.
- **Rooms and suites share one `spaces` table** (`kind` = room | suite). Rooms take occupancy from their room type; suites have a capacity per layout (theatre, boardroom, banquet, cabaret, reception).
- **Validation runs in a transaction**: load → `booker-core` rule → write → commit. The database also has CHECK constraints and a unique index on space names as a backstop.
- **Deletes are soft** (`deleted_at`). A floor can't be deleted while it has spaces; a room type can't be deleted while rooms use it.
- **TypeScript types come from Rust** via ts-rs: `pnpm gen:types` writes `src/lib/bindings/`. Regenerate after changing a `booker-core` type.
- **Frontend imports use `#lib/...` with the file extension** (e.g. `#lib/api.ts`). SvelteKit 3 replaced the `$lib` alias with Node subpath imports (`"imports"` in package.json).
- **UI built with ui-sdk** (Field, Input, NumberInput, Select, Switch, Tabs, Button; `toast()` and `confirm()` via `<OverlayHost />`) on its `web.css`, themed with a green primary colour.
- **Floor-plan editor** is a custom ui-sdk primitive (`src/lib/floor-plan/editor.ts`) built on `createResize`. It works in pixels internally (one snap step = one 28 px cell); the page converts to and from grid cells. Overlaps are refused instantly in the browser (`grid.ts` mirrors the Rust rules) and validated again in Rust on save.
- **Browser tests use Playwright** (`pnpm test:e2e`, specs in `e2e/*.e2e.ts`). The Tauri backend doesn't exist in a plain browser, so `e2e/fake-backend.js` stands in for it. Locally this uses the system Chromium; CI installs Playwright's own.
- Demo data has 3 floors, 40 rooms and 2 suites (the plan said 30 rooms).
- **Closed (2026-11-21):** Andy has some changes in mind and will raise them in a later phase.
- **Floors and layout, reworked (2026-11-21, from Andy's feedback):**
  - The Floors page and the Layout editor are now **one page**, "Floors and layout": pick or add a floor; rename, set its level and grid size, or delete it in the side panel; lay out its rooms on the same screen.
  - **Palette:** drag a Bedroom (3 × 4), Family room (4 × 4) or Function suite (6 × 4) onto the grid, where it lands centred on the drop point. Clicking a piece places it in the first free space. "Add N bedrooms" fills free spots in a row.
  - **Grid resize handles** on the right edge, bottom edge and corner work like spreadsheet column handles: drag, or focus them and use the arrow keys. They snap to cells, never cut off a room, and are capped at 200.
  - **Bug fixed:** on a big grid (e.g. 200 × 200) the horizontal scrollbar sat at the very bottom of a canvas thousands of pixels tall. The plan now measures where it starts and sizes itself to end at the bottom of the window, so both scrollbars are always on screen.
