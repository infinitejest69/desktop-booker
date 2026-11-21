# DesktopBooker — guidance for Claude

Offline hotel booking app (Rust + Tauri 2 + Svelte 5 + SQLite). Andy is **learning Rust**: explain non-obvious Rust (ownership, lifetimes, traits, error handling) briefly when introducing it, and prefer clear idiomatic code over clever code.

## Working on this repo
1. Read [docs/ROADMAP.md](docs/ROADMAP.md) to find the current `in-progress` phase.
2. Work only on tasks in that phase file (`docs/phases/`). Don't start the next phase without asking.
3. Tick `- [x]` tasks as they are finished, and update the progress column in `ROADMAP.md`.
4. Record notable choices under "Notes / decisions" in the phase file, or as a new ADR in `docs/decisions/`.
5. Scope and architecture live in [docs/REQUIREMENTS.md](docs/REQUIREMENTS.md). Follow it or propose an ADR to change it.

## Rules
- Also follow [AGENTS.md](AGENTS.md) (e.g. ui-sdk usage notes).
- Business logic goes in `crates/core` (pure, no I/O) with unit tests written first. Tauri commands stay thin.
- Money is integer minor units. Stays are half-open `[check_in, check_out)`.
- Every DB table has a UUID PK and `created_at`/`updated_at`/`updated_by`. All access goes through repository traits.
- Enforce permissions in Rust commands, not just in the UI.
- Before finishing: `cargo fmt --check && cargo clippy --workspace -- -D warnings && cargo test --workspace` and `pnpm vitest run`.
