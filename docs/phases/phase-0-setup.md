# Phase 0 — Setup & repo

**Goal:** a compiling Tauri 2 + Svelte app shell inside a Cargo workspace, with CI, pushed to the private GitHub repo `desktop-booker`.
**Rust concepts:** cargo, workspaces, crates, `rustfmt`/`clippy`.

## Tasks
- [x] `git init`; write `docs/` (requirements, roadmap, phase checklists, ADRs)
- [x] Install `rustup` (stable toolchain) + `rust-analyzer`
- [x] Install Node LTS + pnpm
- [x] Install Tauri Linux deps (`webkit2gtk-4.1`, `base-devel`, `curl`, `wget`, `file`, `openssl`, `appmenu-gtk-module`, `libappindicator-gtk3`, `librsvg`)
- [x] Install `github-cli`; `gh auth login`
- [x] Scaffold Tauri 2 + Svelte-TS (`pnpm create tauri-app`)
- [x] Restructure into workspace: `crates/core`, `crates/store`, `crates/services`, `src-tauri`, frontend in `src/`
- [x] `.gitignore`, `README.md`, licence choice (proprietary/private vs MIT)
- [x] `rustfmt.toml`, clippy lint config, `vitest` set up
- [x] GitHub Actions CI: fmt + clippy + test on ubuntu; build matrix ubuntu / windows / macos
- [x] `gh repo create desktop-booker --private --source . --push`
- [x] `pnpm tauri dev` opens a window on Linux

## Acceptance
- `cargo test --workspace` and `pnpm vitest run` pass (even if trivial).
- CI green on the first push.
- The repo is private on GitHub.

## Verify
```bash
cargo fmt --check && cargo clippy --workspace -- -D warnings && cargo test --workspace
pnpm vitest run
pnpm tauri dev
gh repo view desktop-booker
```

## Notes / decisions
- **Frontend stays in `src/`** (SvelteKit default), not `ui/` as first planned. Moving it would mean fighting SvelteKit's file conventions for no real gain.
- **Crate names are prefixed `booker-`** (`booker-core`, `booker-store`, `booker-services`), because a crate named `core` would clash with Rust's built-in `core` library.
- **Licence:** private / `UNLICENSED` (all rights reserved). Revisit if it is ever open-sourced.
- Release build profile (LTO, strip) lives in the workspace root `Cargo.toml`. Cargo ignores profiles set in member crates.
- Toolchain at setup: rustc 1.100.0, Node 26.13, pnpm 12.21, Tauri CLI 2, Svelte 5, Vite 8, vitest 5.
- Repo: https://github.com/infinitejest69/desktop-booker (private).
- First CI run failed: adding `packageManager` to `package.json` made the lockfile stale for `--frozen-lockfile`. Fixed in `fix(ci): update lockfile…`.
- Dependabot (account default) is on. It opened PRs straight away, including a **major** `@sveltejs/kit` 2 → 3 bump, and GitHub reports 10 dependency alerts (2 high).

- **ui-sdk added (2026-11-21):** `@ui-sdk/svelte` + `@ui-sdk/core` `0.1.0-beta.13` (unstyled, accessible UI behaviours: keyboard, focus, ARIA, popups). Pinned exactly because it is beta. Agent usage notes are in `AGENTS.md`; the `ui-sdk` skill is in `.agents/skills/` (linked from `.claude/skills/`).

- **SvelteKit 3 (2026-11-21):** merged Dependabot PR #1 (Kit 2.70 → 3.x, resolved to 3.4.0); this clears the SvelteKit/cookie security alerts. Migration:
  - `svelte.config.js` removed. The adapter and preprocess config moved into `sveltekit({...})` in `vite.config.js` (Kit 3 no longer reads the config file).
  - `@sveltejs/adapter-static` 3 → 4 (v3 only supports Kit 2).
  - `tsconfig.json` now `extends: "$app/tsconfig"`.
  - The PR's own CI failed only because of pnpm's `minimumReleaseAge` check (Kit 3.1.1 was brand new). By merge time the version was old enough.
- Remaining alert: `glib` 0.18 (Rust, medium) comes in through Tauri's GTK stack on Linux. It can't be upgraded until Tauri moves, and we don't use `VariantStrIter`.
- Linux dev note: `MESA-EGL ... failed to create dri2 screen` warnings on NVIDIA are a harmless WebKitGTK warning. If the window ever renders blank, try `WEBKIT_DISABLE_DMABUF_RENDERER=1 pnpm tauri dev`.

## Completed (2026-11-21)
CI green on main for Lint & test + Build on ubuntu / windows / macOS. Kit 3 migration verified locally (`pnpm check`, `pnpm test`, `pnpm build`, cargo fmt/clippy/test, `pnpm tauri dev` loads the page).
