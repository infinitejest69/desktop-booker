-- Phase 8: email to guests. The SMTP password is NOT stored here: it's in
-- the operating system's keychain.

-- One row (id = 1).
CREATE TABLE email_settings (
    id            INTEGER PRIMARY KEY CHECK (id = 1),
    host          TEXT NOT NULL DEFAULT '',
    port          INTEGER NOT NULL DEFAULT 587,
    username      TEXT NOT NULL DEFAULT '',
    from_address  TEXT NOT NULL DEFAULT '',
    from_name     TEXT NOT NULL DEFAULT '',
    created_at    TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at    TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_by    TEXT
);

-- The hotel's own wording; a kind with no row uses the built-in default.
CREATE TABLE email_templates (
    id          TEXT PRIMARY KEY NOT NULL,
    kind        TEXT NOT NULL UNIQUE CHECK (kind IN ('confirmation', 'amendment', 'cancellation')),
    subject     TEXT NOT NULL,
    body        TEXT NOT NULL,
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_by  TEXT
);

-- Every email sent or tried.
CREATE TABLE email_log (
    id          TEXT PRIMARY KEY NOT NULL,
    at          TEXT NOT NULL,
    recipient   TEXT NOT NULL,
    subject     TEXT NOT NULL,
    kind        TEXT,
    booking_id  TEXT REFERENCES bookings (id),
    sent        INTEGER NOT NULL,
    error       TEXT NOT NULL DEFAULT '',
    sent_by     TEXT NOT NULL DEFAULT '',
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_by  TEXT
);
CREATE INDEX email_log_booking ON email_log (booking_id, at);
