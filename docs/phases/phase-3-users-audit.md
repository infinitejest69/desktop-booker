# Phase 3 — Users, PIN login & audit log

**Goal:** know who is using the app and who changed what.
**Rust concepts:** app state (`tauri::State`, `Mutex`/`RwLock`), error types across layers, hashing.

## Tasks
- [x] `store`: `users` table (name, role, PIN hash, active)
- [x] `services`: PIN hashing/verification with `argon2`
- [ ] First-run wizard: create the first Admin
- [ ] Lock screen / PIN login; switch user; auto-lock after idle
- [ ] Roles: Admin (settings, rates, users, delete) vs Staff (bookings, housekeeping)
- [ ] Enforce roles in Tauri commands (not just in the UI)
- [x] `audit_log`: entity, action, before/after JSON, user, timestamp, written in the same transaction
- [ ] UI: users management (Admin)
- [ ] UI: audit trail view with filters

## Acceptance
- A Staff user cannot change rates even by calling the command directly.
- Every booking change shows in the audit log with the user's name.

## Notes / decisions
**Decisions from Andy (2026-11-21):**
- PIN when the app starts and after it auto-locks when idle; a **Switch user** button hands over quickly.
- Auto-lock after **15 minutes** idle by default (admins can change it in Hotel details; 0 = never).
- **Staff** may: make and edit bookings, check in/out, cancel and mark no-show.
- **Admin only:** set prices by hand, everything under My hotel (setup, rates, layout), manage users, view the audit log.

**Design:**
- `booker-core`: `User`, `Role` (Admin/Staff), `Permission`, `Role::allows`; PIN rules (4–8 digits).
- PINs stored only as Argon2 hashes (`booker-services`). Five wrong PINs lock that user out for 30 seconds.
- The signed-in user lives in Tauri state. Every command checks the session and the permission it needs before touching the store; the store is given the acting user, so audit entries are written in the same transaction as the change.

## Status at pause (2026-11-21)
**Done (backend):**
- `booker-core`: `User`, `Role`, `Permission` with the full permission table, PIN rules (4–8 digits), the last-admin guard, the `AuditEntry` type and the `idle_lock_minutes` setting.
- `booker-services`: Argon2id `hash_pin` / `verify_pin`.
- `booker-store`: migration `0003_users_audit.sql`; `list_users`, `save_user`, `pin_hash`; `store.as_user(&user)` so every save and delete (settings, floors, room types, spaces, rate rules, board options, bookings, status changes, users) writes an audit entry with before/after JSON **in the same transaction**; `list_audit(user, entity, limit)` lists newest first, tie-broken by the time-ordered id.
- Tests: 96 Rust (incl. 7 new users/audit store tests), 23 Vitest, 20 Playwright — all passing.

## Next session
1. Tauri session state: signed-in user, `session_status`, `create_first_admin`, `login` (5 wrong PINs → 30 s lockout), `logout`; idle lock (default 15 min, `idle_lock_minutes`).
2. Enforce permissions in every command (`require(Permission)`), use `store.as_user(...)` for changes; staff can't set `price_override`; new error kinds `locked` / `forbidden`.
3. UI: first-run wizard, lock screen with PIN pad, signed-in name + Switch user in the top bar, hide admin-only menu items for staff, price override hidden for staff, auto-lock setting on Hotel details.
4. UI: Users page and Audit log page (filters) under My hotel.
5. Fake backend + Playwright tests for login, switch user, staff restrictions and the audit log.
