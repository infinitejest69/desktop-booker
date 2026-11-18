# ADR 0002 — Multi-device support deferred, kept possible

**Status:** accepted · 2026-11-18

## Context
It is not yet known whether one PC, several LAN PCs or phones/tablets will be used. There is no hosting.

## Decision
- v1 targets a single PC with a local SQLite file.
- All data access goes through repository traits in `crates/store`; every row has a UUID and `updated_at`/`updated_by`.
- A later LAN mode will run an embedded `axum` server on one host PC; other devices act as clients.
- The live database file must **never** be placed in a shared or cloud-synced folder (risk of corruption). Only backups go there.

## Consequences
Small upfront cost (traits, UUIDs) in exchange for a clean path to multi-device use without a rewrite.
