# Phase 11 — Demo hotel, user guide, videos and going public

**Goal:** a rich, made-up seaside hotel to demo with; a user guide with screenshots; short demo videos for YouTube; and the repository ready to make public under MIT + Commons Clause.
**Rust concepts:** a seeded random number generator (`rand` with a fixed seed: same data every run), builders, generating data relative to "today".

## Decisions (Andy, 2026-11-22)
- **Demo hotel:** a seaside town hotel. Working name **The Lanterns Hotel, Kelmouth**, an invented hotel and town. It replaces the current "The Grange" demo.
- **Videos:** scripted with Playwright (repeatable; recorded again with one command after UI changes).
- **Licence:** MIT + Commons Clause. Free to use and change, including to run a hotel, but nobody may sell the software (or paid hosting/support whose value comes mainly from it). This is *source-available*, not OSI open source.
- **Git history:** rewrite the author email on every commit to Andy's GitHub noreply address before going public (force push, after Andy confirms).

## Tasks
### A. The demo hotel (data)
- [x] Core `demo.rs`, pure and tested first: generates the hotel from a fixed seed and today's date. Same seed and date give the same data; dates move with today, so it always looks current.
  - **Building:** 3 floors, about 40 rooms (sea view and town view; singles, doubles, twins, family rooms, two suites); a ballroom and a small meeting room; room features (wifi everywhere, a few dog-friendly rooms, accessible ground-floor rooms, no smoking rooms); prices with seasonal rate rules and weekend supplements; RO / B&B / DBB board.
  - **Guests:** about 600 invented guests (common first names and surnames, made-up email addresses and phone numbers on reserved test ranges), with some returning guests.
  - **Bookings:** 18 months of history and 6 months ahead. Busy summers and school holidays, quiet Januaries. Every source and status; cancellations and no-shows; multi-room family bookings; parking on about a third.
  - **Events:** weddings in the ballroom (with room blocks), a few conferences, a coach-tour group, Christmas and New Year dinners.
  - **Money:** bills with bar and restaurant extras, deposits and payments by every method, issued invoices for past stays.
  - **Housekeeping:** today's state (some dirty, some clean, some ready).
- [x] Store: `seed_demo` uses the generator (one transaction; "Load demo hotel" stays on the empty dashboard). Tests: loads, no double bookings, reports show seasonal occupancy.
- [x] ~~Export the same data as JSON for the fake backend~~ Replaced (Andy, 2026-11-22) by the **demo bridge**: `src-tauri/src/bin/demo_bridge.rs` runs the app's real commands behind `http://127.0.0.1:1430`, and `demo/bridge-client.js` points the app in Chromium at it, so screenshots and videos show the real hotel and real figures.

### B. Screenshots
- [x] `pnpm screenshots`: Playwright scripts that load the demo hotel and capture each screen (dashboard, calendar, booking, rooms, suites and events, bill, reports and dashboards, setup), in light and dark, at a fixed window size and a fixed "today".
- [x] Images saved under `docs/guide/images/`, committed, and redone whenever screens change.

### C. User guide
- [ ] `docs/guide/` in Markdown: getting started (install, first admin, demo hotel), a front-desk day, bookings and the calendar, rooms and housekeeping, suites and events, bills and payments, guests, reports and dashboards, hotel setup, users and PINs, email, backups and restore, FAQ.
- [ ] Published as a website with VitePress (fits the pnpm/Vite setup) on GitHub Pages, built by a workflow on every push to `main`; linked from the README.

### D. Demo videos
- [x] `pnpm videos`: Playwright scenes (about 1 to 3 minutes each) with a slowed, visible pointer and on-screen captions. Candidates: a tour in 2 minutes; taking a booking; check-in and check-out; a wedding with a room block; the bill and payments; building a report and a dashboard; backups.
- [x] ffmpeg joins each scene with a title card and end card into `videos/out/*.mp4` (1920 × 1080). Andy uploads to YouTube and adds voice-over or music if wanted; links go in the README and guide.

### E. Going public
- [x] `LICENSE`: MIT + Commons Clause; `package.json` and the Cargo `license` fields updated; README licence section.
- [x] Scan the whole history for secrets (gitleaks); check the docs for personal details (for example notes mentioning Andy's Gmail) and reword them.
- [x] ~~Rewrite commit author emails to the GitHub noreply address~~ Not needed: every commit already uses the noreply address.
- [x] `CONTRIBUTING.md`, `SECURITY.md`, issue and pull request templates; README with screenshots, the guide link and videos.
- [x] Andy switches the repository to public on GitHub (done 2026-11-22, before the clean-up).

## Acceptance
- "Load demo hotel" gives The Lanterns Hotel with a year and a half of believable history, and the reports, calendar and dashboard look like a real, busy seaside hotel.
- `pnpm screenshots` and `pnpm videos` run unattended and give the same pictures on every run.
- The guide is online, every page has up-to-date screenshots, and the README links to it and to the videos.
- The repository is public with the agreed licence, no secrets in its history and no personal email on its commits.

## Notes / decisions
- Created 2026-11-22 at Andy's request: a richer demo hotel ("a made up hotel"), screenshots, a user guide and demo videos for YouTube, and a public repository "free to anyone to use", which Andy then narrowed to "free use but stop people selling the product". *Richer demo data* moved here from Phase 10.
- Licence options considered: PolyForm Shield (no competing products), MIT + Commons Clause (no selling), PolyForm Noncommercial (rejected: running a hotel is commercial use), GPL-3.0 (open source; resale allowed with source). Not legal advice.
- All demo people, emails and phone numbers are invented; phone numbers use the UK's reserved drama ranges and emails use `example.com`-style domains, so nothing reaches a real person.
- **README and guide started (2026-11-22):** at Andy's request the README is now a short pitch for hoteliers ("a basic sales pitch") with five everyday workflows (a phone booking, check-in, dinner charged to the room, check-out and the bill, housekeeping) and links to a page per feature. The feature pages are the start of the user guide (`docs/guide/`: front desk, bookings, rooms, suites and events, bills, guests, reports, setup, backups), written from the app's real screen and button names; screenshots come with task B. The developer instructions moved unchanged to `docs/DEVELOPMENT.md`. Still to write for task C: installing, first run and the FAQ.
- **Paused 2026-11-22 (parked on branch `wip/phase-11-demo-hotel`):**
  - Done: `crates/core/src/demo.rs` (`demo_hotel(seed, today)`, our own SplitMix64 generator rather than `rand`, so the output never changes between crate versions; IDs come from the generator too). It gives about 5,600 bookings, 4,500 guests (more than the planned 600, to suit 40 rooms over two years), 85 events, coach tours twice a year, wedding room blocks, bills, deposits, payments by every method, about 2,000 invoices, and this morning's housekeeping. 11 core tests.
  - Store `seed_demo` writes it: building through the repos, stays and bills in one transaction (`BookingContext` loaded once), events through the repos, then invoices in date order. Bookings get the date they were made and a reference from that year. About 3.4 s in release, about 9 s in dev (dependencies now built with `opt-level = 2` in the dev profile).
  - Removed `seed_demo_events` / `seed_demo_invoicing` and the Events page's "Load demo events" button; the dashboard button shows "Loading the demo hotel…".
  - All checks passed at this point (cargo fmt, clippy, cargo test, pnpm check, pnpm test, 72 e2e).
  - **Open question for task A3:** the fake backend's reports are stand-ins (its `revenue_report` returns nothing), so screenshots taken against it would show blank or wrong figures. Proposed instead: a dev-only bridge that runs the real Tauri commands (Tauri's mock runtime) behind a local HTTP port, so Playwright in Chromium drives the real backend with the demo hotel. Needs Andy's OK.
- **Resumed 2026-11-22:** Andy agreed to the bridge, and asked for more coach tours.
  - *Coach tours:* four invented operators, about 14 tours a year from March to November (fewer in high summer), 2 to 4 nights, Sunday to Tuesday arrivals, 8 to 14 doubles and twins plus singles, dinner included, at an agreed group rate per room (£118 a night, singles £75), the driver free, an operator's tour manager returning every time, settled by bank transfer with an invoice; now and then one ahead is cancelled. Stays with an agreed price are settled from the full folio.
  - *Bridge:* the command list moved into `invoke_handler::<R>()` in `src-tauri/src/lib.rs`, and the eight commands taking an `AppHandle` became generic over the runtime, so the app and the bridge share exactly the same commands. The bridge uses Tauri's mock runtime (`tauri/test`, only with `--features demo-bridge`) with the app's real config and permissions; plain standard-library HTTP, one request at a time.
  - *Demo runs:* `demo/playwright.config.ts` starts Vite and a fresh bridge, `demo/global-setup.ts` makes the admin (Andy, 1234) and loads the demo hotel, and `pnpm screenshots` runs `demo/*.demo.ts`. First picture: `docs/guide/images/dashboard.png` (1440 × 900 at 2×). Dates follow today, so pictures change from day to day; that's accepted, since the demo always looks current.
- **Screenshots done (2026-11-22):** 27 screens in light and dark (`demo/screenshots.demo.ts`), saved as lossless WebP through ffmpeg (4 MB in all, instead of 15 MB as PNG), shown in the guide and README with `<picture>` so readers see the dark pictures when their theme is dark. The demo hotel now also has eight saved reports and a "Monday meeting" dashboard, so Reports isn't empty. Fixed `pnpm tauri dev` after the bridge became a second binary (`default-run = "desktop-booker"`). Not a fixed "today" after all: dates follow the day the pictures are taken.
- **Videos started (2026-11-22):** `pnpm videos` records `demo/scenes.video.ts`: six scenes (tour, phone booking, check-in/dinner/check-out, housekeeping, wedding, reports) to `videos/out/*.mp4` (ignored by git; Andy uploads them). Recorded from Chromium's screencast and encoded by ffmpeg (H.264, CRF 18, 30 fps), sharper than Playwright's own video. `demo/stage.js` draws the pointer, click ripples, captions and title/end cards inside the page, so no editing is needed. At Andy's request: true 1080p (1920 × 1080 at 1×) so whole screens fit; the tour also shows the bookings List with a search, and searches for and opens a guest. Demo data gained office Christmas parties (Fridays and Saturdays, 20 Nov to 20 Dec) and a few more conferences, so the suites aren't empty in late autumn.
- **Nine videos (2026-11-22):** added setting up a hotel, staff/PINs/audit, and guests and data protection (Andy's choice). Demo data gained a duplicate guest to merge (Margaret and "Maggie" Trevithick). YouTube links are in the README workflows and at the top of each guide page; Andy uploads, then sends the links. Making the videos found a real bug: a new event's room block kept the default dates when the event's first day changed, so it held the wrong nights. Fixed in `EventForm.svelte` (an unsaved block moves with the first day), with an e2e test.
- **All nine videos on YouTube (2026-11-22):** title and end cards are drawn in the page by `demo/stage.js`, so ffmpeg only encodes (no joining needed). Links in the README (workflows, and a Video column in the Features table) and at the top of each guide page. Backups has no video yet.
- **Going public (2026-11-22):** Andy made the repository public, then asked for the clean-up. History scan (every version of every file, for keys, tokens, private keys, password assignments, `.env`/key/database files and personal emails): nothing secret found; every commit is already authored with the GitHub noreply address, so no rewrite or force push. The only find was a made-up `grange@gmail.com` in tests, now `grange.hotel@example.com` (the old string stays in history; it isn't anyone's secret). Added `LICENSE` (Commons Clause v1.0 over MIT; `license-file` in Cargo, "SEE LICENSE IN LICENSE" in package.json), `CONTRIBUTING.md`, `SECURITY.md` (private reporting through GitHub), issue forms and a pull request template. Andy's name stays as author and copyright holder.
