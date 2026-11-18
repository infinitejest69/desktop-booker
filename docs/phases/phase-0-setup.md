# Phase 0 — Setup & repo

**Goal:** a compiling Tauri 2 + Svelte app shell inside a Cargo workspace, with CI, pushed to the private GitHub repo `desktop-booker`.
**Rust concepts:** cargo, workspaces, crates, `rustfmt`/`clippy`.

## Tasks
- [x] `git init`; write `docs/` (requirements, roadmap, phase checklists, ADRs)
- [ ] Install `rustup` (stable toolchain) + `rust-analyzer`
- [ ] Install Node LTS + pnpm
- [ ] Install Tauri Linux deps (`webkit2gtk-4.1`, `base-devel`, `curl`, `wget`, `file`, `openssl`, `appmenu-gtk-module`, `libappindicator-gtk3`, `librsvg`)
- [ ] Install `github-cli`; `gh auth login`
- [ ] Scaffold Tauri 2 + Svelte-TS (`pnpm create tauri-app`)
- [ ] Restructure into workspace: `crates/core`, `crates/store`, `crates/services`, `src-tauri`, `ui/`
- [ ] `.gitignore`, `README.md`, licence choice (proprietary/private vs MIT)
- [ ] `rustfmt.toml`, clippy lint config, `vitest` set up
- [ ] GitHub Actions CI: fmt + clippy + test on ubuntu; build matrix ubuntu / windows / macos
- [ ] `gh repo create desktop-booker --private --source . --push`
- [ ] `pnpm tauri dev` opens a window on Linux

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
