# Phase 7 — Backups & packaging (v1.0)

**Goal:** the data is safe and the app installs easily on Windows, Linux and macOS.
**Rust concepts:** background tasks (`tokio`), filesystem handling, cross-platform builds.

## Tasks
- [x] Scheduled daily backup using the SQLite online backup API (safe while running)
- [x] Backup folder setting (USB / OneDrive / Dropbox / NAS), rotation (e.g. keep 14 daily + 12 monthly)
- [x] Backup status indicator + warning if the last backup is too old or the folder is missing
- [x] Restore from backup with confirmation + automatic pre-restore snapshot
- [x] Tested restore round-trip
- [x] App icon, name, version metadata
- [x] Windows installer (MSI/NSIS) via CI
- [x] Linux AppImage + .deb via CI
- [x] macOS .dmg via CI (unsigned is fine initially): built by the release workflow; checking it on a Mac *moved to Phase 8*
- [x] Release workflow: tag → GitHub Release with artefacts

## Acceptance
- Delete the live DB → restore from backup → all data is back.
- Clean install from the CI artefacts works on Windows and Linux.

## Notes / decisions
**Built (2026-11-21):**
- `crates/core/src/backups.rs`: backup names (`booker-YYYYMMDD-HHMMSS.db`), overdue after 26 hours, rotation keeping the newest backup of each of the last 14 days and of each of the last 12 months (the newest backup always survives).
- Store: migration `0007` (`backup_settings`), `BackupRepo` (`backup_now` = `VACUUM INTO`, safe while the app runs, then rotation), `list_backups`, `restore_file` (checks the SQLite header, removes the old `-wal`/`-shm` files, copies the backup in), `SqliteStore::close`.
- App: a background thread checks every 30 minutes and backs up when overdue; a backup is also taken on exit. Restore takes a fresh backup first, closes the database, swaps the file and restarts the app.
- Screens: My hotel → Backups (status, folder and how many to keep, Back up now, list with Restore after a confirmation); a dashboard warning when no backup in a day or the folder can't be reached.
- Acceptance "delete the live DB → restore → all data back" is a store test (`delete_the_live_data_restore_a_backup_and_it_is_all_back`).
- Packaging: a new app icon (brass key fob on heather, `src-tauri/app-icon.png`; regenerate sizes with `pnpm tauri icon src-tauri/app-icon.png`), bundle metadata (publisher, category, descriptions), and `.github/workflows/release.yml`: pushing a `v*` tag builds Windows (MSI, NSIS), Linux (AppImage, .deb) and macOS (.dmg, unsigned) installers into a **draft** GitHub Release. A local `.deb` build was checked. Clean installs from CI artefacts on Windows and Linux (the second acceptance) need real machines: *moved to Phase 8*.

