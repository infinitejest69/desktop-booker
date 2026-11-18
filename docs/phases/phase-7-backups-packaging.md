# Phase 7 — Backups & packaging (v1.0)

**Goal:** the data is safe and the app installs easily on Windows, Linux and macOS.
**Rust concepts:** background tasks (`tokio`), filesystem handling, cross-platform builds.

## Tasks
- [ ] Scheduled daily backup using the SQLite online backup API (safe while running)
- [ ] Backup folder setting (USB / OneDrive / Dropbox / NAS), rotation (e.g. keep 14 daily + 12 monthly)
- [ ] Backup status indicator + warning if the last backup is too old or the folder is missing
- [ ] Restore from backup with confirmation + automatic pre-restore snapshot
- [ ] Tested restore round-trip
- [ ] App icon, name, version metadata
- [ ] Windows installer (MSI/NSIS) via CI
- [ ] Linux AppImage + .deb via CI
- [ ] macOS .dmg via CI (unsigned is fine initially)
- [ ] Release workflow: tag → GitHub Release with artefacts

## Acceptance
- Delete the live DB → restore from backup → all data is back.
- Clean install from the CI artefacts works on Windows and Linux.

## Notes / decisions
