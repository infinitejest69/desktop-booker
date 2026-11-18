# Phase 3 — Users, PIN login & audit log

**Goal:** know who is using the app and who changed what.
**Rust concepts:** app state (`tauri::State`, `Mutex`/`RwLock`), error types across layers, hashing.

## Tasks
- [ ] `store`: `users` table (name, role, PIN hash, active)
- [ ] `services`: PIN hashing/verification with `argon2`
- [ ] First-run wizard: create the first Admin
- [ ] Lock screen / PIN login; switch user; auto-lock after idle
- [ ] Roles: Admin (settings, rates, users, delete) vs Staff (bookings, housekeeping)
- [ ] Enforce roles in Tauri commands (not just in the UI)
- [ ] `audit_log`: entity, action, before/after JSON, user, timestamp, written in the same transaction
- [ ] UI: users management (Admin)
- [ ] UI: audit trail view with filters

## Acceptance
- A Staff user cannot change rates even by calling the command directly.
- Every booking change shows in the audit log with the user's name.

## Notes / decisions
