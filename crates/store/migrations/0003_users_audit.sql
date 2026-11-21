-- Phase 3: staff users and the audit log.

ALTER TABLE property_settings ADD COLUMN idle_lock_minutes INTEGER NOT NULL DEFAULT 15
    CHECK (idle_lock_minutes BETWEEN 0 AND 240);

CREATE TABLE users (
    id          TEXT PRIMARY KEY NOT NULL,
    name        TEXT NOT NULL,
    role        TEXT NOT NULL CHECK (role IN ('admin', 'staff')),
    active      INTEGER NOT NULL DEFAULT 1,
    -- Argon2id PHC string; the PIN itself is never stored.
    pin_hash    TEXT NOT NULL,
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_by  TEXT
);
CREATE UNIQUE INDEX users_name_unique ON users (lower(trim(name)));

-- Append-only record of every change. user_name is copied in so the log
-- still reads correctly if a user is renamed later.
CREATE TABLE audit_log (
    id          TEXT PRIMARY KEY NOT NULL,
    at          TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    user_id     TEXT REFERENCES users (id),
    user_name   TEXT NOT NULL,
    entity      TEXT NOT NULL,
    entity_id   TEXT NOT NULL,
    action      TEXT NOT NULL,
    summary     TEXT NOT NULL,
    before      TEXT,
    after       TEXT
);
CREATE INDEX audit_log_at ON audit_log (at DESC);
CREATE INDEX audit_log_entity ON audit_log (entity, entity_id);
CREATE INDEX audit_log_user ON audit_log (user_id);
