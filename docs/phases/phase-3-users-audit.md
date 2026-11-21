# Phase 3 — Users, PIN login & audit log

**Goal:** know who is using the app and who changed what.
**Rust concepts:** app state (`tauri::State`, `Mutex`/`RwLock`), error types across layers, hashing.

## Tasks
- [x] `store`: `users` table (name, role, PIN hash, active)
- [x] `services`: PIN hashing/verification with `argon2`
- [x] First-run wizard: create the first Admin
- [x] Lock screen / PIN login; switch user; auto-lock after idle
- [x] Roles: Admin (settings, rates, users, delete) vs Staff (bookings, housekeeping)
- [x] Enforce roles in Tauri commands (not just in the UI)
- [x] `audit_log`: entity, action, before/after JSON, user, timestamp, written in the same transaction
- [x] UI: users management (Admin)
- [x] UI: audit trail view with filters

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

## Built (2026-11-21)
- **Session (`src-tauri/src/session.rs`):** signed-in user behind a `Mutex`, idle lock (setting, default 15 min, 0 = never), five wrong PINs → 30-second lockout for that user. Restarting the app signs everyone out.
- **Every command checks permission** via `session.require(&store, Permission::…)`, which returns the store acting as that user, so changes are audited under their name. Staff can't set or change a hand-set price (checked against the saved booking); cancel and no-show need `CancelBookings`. New error kinds `locked` and `forbidden`.
- **New commands:** `session_status`, `create_first_admin` (only while no users exist), `login`, `logout`, `list_users`, `save_user`, `list_audit`. Demo data is still logged as "System".
- **UI:** first-run screen, lock screen (choose person + PIN), "Signed in as …" with **Switch user** in the top bar, auto-lock after idle (client timer + server check), My hotel shown to admins only, **Users** and **Audit log** pages (filter by person and kind, before/after view), auto-lock setting on Hotel details, "Set the price by hand" hidden for staff, staff see "ask an admin" instead of "Load demo hotel".
- **Bug found and fixed:** the layout's refresh-after-navigation effect read `session.user`, which every refresh replaces, so it looped forever; it now reads it with `untrack`.
- **Tests:** 101 Rust (incl. session/permission tests), 25 Vitest, 25 Playwright (first run, wrong/right PIN, staff limits incl. a direct command refused, switch user, users + audit log).
- **Lockout fix (Andy's report):** during a wrong-PIN lockout the PIN box stayed usable and the "try again in N seconds" text was a frozen snapshot. Now the fifth wrong PIN starts the lockout immediately (earlier tries say how many are left). The error carries `retry_after` seconds, and the lock screen disables the PIN box and Sign in **for that person only**, shows a live countdown and puts the cursor back in the PIN box when it ends. Trying during a lockout never extends it.
