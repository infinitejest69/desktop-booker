# Contributing to DesktopBooker

Thanks for helping. Bug reports, ideas and pull requests are all welcome.

## Reporting a bug or asking for a feature
Open an [issue](../../issues/new/choose). For a bug, say what you did, what you expected and what happened, and which system you're on (Windows, macOS or Linux). Screenshots help. **Never include real guest details**: use the demo hotel (*Load demo hotel* on an empty database) to show a problem.

Security problems: please don't open a public issue; see [SECURITY.md](SECURITY.md).

## Working on the code
How to set up, run and test the app is in [docs/DEVELOPMENT.md](docs/DEVELOPMENT.md). In short:

- Business rules go in `crates/core` (no I/O), with unit tests written first. Tauri commands stay thin and check permissions in Rust.
- Money is whole pence (integer minor units). Stays are half-open date ranges: check-in night included, check-out day not.
- Every database table has a UUID key and `created_at` / `updated_at` / `updated_by`; all access goes through the repository traits in `crates/store`.
- Browser tests use Playwright against a fake backend (`e2e/`).
- Plans live in [docs/ROADMAP.md](docs/ROADMAP.md) and `docs/phases/`; bigger decisions are written up in `docs/decisions/`.

Before opening a pull request, run:

```sh
cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace
pnpm check && pnpm test && pnpm test:e2e
```

Keep pull requests small and focused, and say what they change and why.

## Licence of contributions
DesktopBooker is MIT with the Commons Clause (see [LICENSE](LICENSE)): free to use and change, but not to sell. By sending a contribution you agree it's offered under the same licence.
