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

## Status at pause (2026-11-18)
- Locally verified: `pnpm check`, `pnpm test`, `cargo fmt/clippy/test` all pass; `pnpm tauri dev` builds and opens the window.
- CI run for `fix(ci)` commit: **Lint & test (Linux) passed**. The ubuntu / windows / macos build jobs were still running at the pause.

## Next session
1. Check CI: `gh run list -L3`. If a build job failed, `gh run view <id> --log-failed` and fix.
2. Triage Dependabot: review the alerts at /security/dependabot; merge safe patch/minor PRs; **hold the SvelteKit 3 major** (decide via ADR), or configure `.github/dependabot.yml` to group updates and ignore majors.
3. Once CI is green on all 3 OSes → mark Phase 0 `done` in ROADMAP and start Phase 1.
