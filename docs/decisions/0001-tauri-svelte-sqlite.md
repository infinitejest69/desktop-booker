# ADR 0001 — Tauri 2 + Svelte + SQLite

**Status:** accepted · 2026-11-18

## Context
The app must be offline, free to run and cross-platform (Windows/Linux/macOS, Android later). The owner wants to learn Rust.

## Decision
- **Tauri 2** for the shell; all business logic in Rust.
- **Svelte 5 + TypeScript** for the UI (rich widgets such as the floor plan and tariff chart are easier on the web stack).
- **SQLite** (via `sqlx`) as a single local database file.
- Pure-Rust alternatives (Slint, egui, iced) were considered and rejected: Android support is weaker and there are fewer business-UI widgets.

## Consequences
- Two languages (Rust + TS), but the Rust side holds everything important and testable.
- Tauri mobile makes Android possible later from the same codebase.
