# DesktopBooker

Offline desktop booking system for a small family-run hotel: rooms, function suites, events, invoicing, housekeeping, guests and reports. No hosting and no running costs. Data stays on the hotel's own computer.

Built with **Rust + Tauri 2** and a **SvelteKit 3 (Svelte 5)** frontend. Runs on Windows, Linux and macOS; Android planned.

- Requirements & architecture: [docs/REQUIREMENTS.md](docs/REQUIREMENTS.md)
- Roadmap & progress: [docs/ROADMAP.md](docs/ROADMAP.md)

> **Status:** Phases 0–1 done · Phase 2 (room bookings & rates) built, awaiting a hands-on check.

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
Opens the DesktopBooker window. On an empty database, the Dashboard offers **Load demo hotel** (3 floors, 40 rooms, 2 suites, prices, board options and a few bookings). Frontend edits (`src/`) appear as soon as you save; Rust edits (`src-tauri/`, `crates/`) recompile and relaunch the window automatically. The first build takes about a minute.

- **NVIDIA on Linux, window blank or flickering:** `WEBKIT_DISABLE_DMABUF_RENDERER=1 pnpm tauri dev`. The `MESA-EGL` warnings in the terminal are harmless.
- **Frontend only, in a browser:** `pnpm dev`, then open http://localhost:1420. Calls into Rust won't work there.

## Build a release

```bash
pnpm tauri build              # optimised app + installers
pnpm tauri build --no-bundle  # just the executable (faster)
```
- Executable: `target/release/desktop-booker` (`.exe` on Windows)
- Installers: `target/release/bundle/` (Linux: AppImage / deb / rpm; Windows: msi / nsis; macOS: dmg / app)

Each OS's installers have to be built on that OS. CI builds all three on every push to `main`.

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

The database is a single SQLite file in the OS app-data folder (e.g. `~/.local/share/com.desktopbooker.app/` on Linux, `%APPDATA%\com.desktopbooker.app\` on Windows). Each time the app closes it saves a backup copy to `backups/` next to the database (newest 10 kept). Never put the live database in a cloud-synced or network folder; scheduled backups to a folder of your choice come in Phase 7.

## Licence

Private. All rights reserved.
