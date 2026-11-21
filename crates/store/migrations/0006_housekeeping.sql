-- Phase 6: housekeeping status per bedroom. A room with no row is ready
-- (inspected); check-out marks it dirty.

CREATE TABLE housekeeping (
    id          TEXT PRIMARY KEY NOT NULL,
    space_id    TEXT NOT NULL UNIQUE REFERENCES spaces (id),
    status      TEXT NOT NULL CHECK (status IN ('clean', 'dirty', 'inspected')),
    note        TEXT NOT NULL DEFAULT '',
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_by  TEXT
);
