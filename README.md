# DesktopBooker

Offline desktop booking system for a small family-run hotel: rooms, function suites, events, invoicing, housekeeping, guests and reports. No hosting and no running costs. Data stays on the hotel's own computer.

Built with **Rust + Tauri 2** and a **SvelteKit 3 (Svelte 5)** frontend. Runs on Windows, Linux and macOS; Android planned.

- Requirements & architecture: [docs/REQUIREMENTS.md](docs/REQUIREMENTS.md)
- Roadmap & progress: [docs/ROADMAP.md](docs/ROADMAP.md)

> **Status:** Phases 0–7 done (v1 features) · Phase 8 (carried-over checks on real machines) in progress · Phase 9 (report & dashboard builder) and Phase 10 (backlog) planned.

## What's in it

- **Dashboard:** one page: a small plan of each floor, today's arrivals and departures, events coming up, backup warnings.
- **Bookings:** one menu item with two views: the **Calendar** (rooms × nights; click a free night to book, a booking to open it) and the **List** (search by reference, name or email; searching from the calendar opens the list). Each room on a booking can ask for parking (cars × the nightly parking price set under Rates).
- **Suites and events:** one menu item with two views: the suite diary (hire by the hour) and the events list (catering sittings, room blocks).
- **Rooms:** every floor in one list, as plans or cards; availability, dirty / clean / ready, whole floors at once; each room's features (wifi, pets allowed, accessible, smoking) as icons, and a *Room must have* filter when booking.
- **Guests:** profiles, history, merging duplicates, data export.
- **Invoices & payments**, **email** to guests through the hotel's Gmail.
- **Admin only** (below the divider in the menu): **Reports** (occupancy, revenue, ADR & RevPAR, CSV) and **My hotel** (setup, rates, users, backups, audit log).

## Prerequisites

| Tool | Version used | Notes |
|---|---|---|
| Rust (via `rustup`) | stable (1.100+) | `rustup default stable` |
| Node.js | LTS (26.x) | |
| pnpm | 12.x | Version pinned in `package.json` (`packageManager`) |
| Tauri system libraries | — | Linux only, see below |

**Linux (Arch / CachyOS):**
```bash
sudo pacman -S --needed rustup nodejs npm pnpm base-devel webkit2gtk-4.1 curl wget file openssl appmenu-gtk-module libappindicator-gtk3 librsvg
rustup default stable
```
**Linux (Debian / Ubuntu):** `sudo apt install libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf build-essential`
**Windows / macOS:** see the [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/). Windows needs the Visual Studio C++ Build Tools and WebView2; macOS needs the Xcode Command Line Tools.

Then install the JS dependencies:
```bash
pnpm install
```

## Run the app (development)

```bash
pnpm tauri dev
```
Opens the DesktopBooker window. On a new database it first asks you to create the admin (name + 4–8 digit PIN); after that everyone signs in with their own PIN, and the app locks itself after 15 idle minutes (changeable in Hotel details). On an empty database, the Dashboard offers **Load demo hotel** (3 floors, 40 rooms, 2 suites, prices, board options and a few bookings). Frontend edits (`src/`) appear as soon as you save; Rust edits (`src-tauri/`, `crates/`) recompile and relaunch the window automatically. The first build takes about a minute.

- **NVIDIA on Linux, window blank or flickering:** `WEBKIT_DISABLE_DMABUF_RENDERER=1 pnpm tauri dev`. The `MESA-EGL` warnings in the terminal are harmless.
- **Frontend only, in a browser:** `pnpm dev`, then open http://localhost:1420. Calls into Rust won't work there.
- **`Port 1420 is already in use`:** another dev server is still running (an old `pnpm dev` or `pnpm tauri dev` in another terminal, or one left behind after a crash). Stop it with Ctrl+C in that terminal, or find and stop it:
  ```bash
  ss -ltnp | grep 1420     # shows the pid
  kill <pid>
  ```

## Build a release

```bash
pnpm tauri build              # optimised app + installers
pnpm tauri build --no-bundle  # just the executable (faster)
```
- Executable: `target/release/desktop-booker` (`.exe` on Windows)
- Installers: `target/release/bundle/` (Linux: AppImage / deb / rpm; Windows: msi / nsis; macOS: dmg / app)

Each OS's installers have to be built on that OS. CI checks all three build on every push to `main`.

**Testing the Linux build locally:**

1. Stop any running `pnpm tauri dev` window (both use the same database).
2. `pnpm tauri build`. The first time takes a few minutes and downloads the AppImage tools.
3. Run it:
   ```bash
   ./target/release/desktop-booker                                        # the bare executable
   ./target/release/bundle/appimage/DesktopBooker_0.1.0_amd64.AppImage    # the AppImage (needs fuse2 on Arch)
   ```
   No FUSE? `--appimage-extract-and-run` after the AppImage path works without it. The `.deb` (Debian/Ubuntu) and `.rpm` (Fedora) are in `target/release/bundle/` too, for testing on those systems.
4. **To test it like a clean install**, move the app's data aside first and put it back afterwards:
   ```bash
   mv ~/.local/share/com.desktopbooker.app ~/.local/share/com.desktopbooker.app.dev
   # …test: first-run admin, Load demo hotel, a booking, a backup and restore…
   rm -rf ~/.local/share/com.desktopbooker.app
   mv ~/.local/share/com.desktopbooker.app.dev ~/.local/share/com.desktopbooker.app
   ```

**Publishing installers:** push a version tag and GitHub builds them all:

```bash
git tag v1.0.0 && git push origin v1.0.0
```

The Release workflow (`.github/workflows/release.yml`) attaches Windows (MSI, NSIS), Linux (AppImage, deb) and macOS (dmg) installers to a **draft** release on GitHub; check it, then publish. Windows and macOS builds are unsigned, so the system asks for confirmation the first time.

## Checks

The same checks run in CI ([.github/workflows/ci.yml](.github/workflows/ci.yml)):

```bash
pnpm check                                             # svelte-check / TypeScript
pnpm test                                              # vitest (frontend unit tests)
pnpm test:e2e                                          # Playwright browser tests (e2e/)
cargo fmt --all --check                                # Rust formatting
cargo clippy --workspace --all-targets -- -D warnings  # Rust lints
cargo test --workspace                                 # Rust tests
```

Browser tests run the frontend in Chromium with a fake backend (`e2e/fake-backend.js`). Locally they use the system Chromium (`/usr/bin/chromium`, or set `CHROMIUM_PATH`); otherwise run `pnpm exec playwright install chromium` once.

After changing a Rust type in `booker-core`, regenerate the TypeScript types with `pnpm gen:types`.

## Project layout

```
crates/
  core/       booker-core     — pure domain logic (no I/O): types, rules, pricing
  store/      booker-store    — SQLite persistence: migrations, repositories, backups
  services/   booker-services — email, PDF, CSV, hashing, keychain
src-tauri/    desktop-booker  — Tauri shell: thin commands calling the crates
src/          SvelteKit 3 frontend (SPA via adapter-static; config in vite.config.js)
e2e/          Playwright browser tests (+ fake backend)
docs/         requirements, roadmap, phase checklists, decisions (ADRs)
```

## Data

The database is a single SQLite file in the OS app-data folder (e.g. `~/.local/share/com.desktopbooker.app/` on Linux, `%APPDATA%\com.desktopbooker.app\` on Windows). Never put the live database in a cloud-synced or network folder.

**Backups:** the app backs up once a day while it's open and again when it closes, keeping one per day for 14 days and one per month for 12 months. Choose the folder under *My hotel → Backups* (a USB stick, network drive, OneDrive or Dropbox folder is best); by default they go in `backups/` next to the database. The dashboard warns if no backup has been taken in a day or the folder can't be reached. *Restore* takes a backup of the current data first, then replaces it and restarts the app.

## Licence

Private. All rights reserved.
