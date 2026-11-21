-- Phase 7: where backups go and how many to keep. One row (id = 1);
-- with no row the app uses its own data folder, 14 daily and 12 monthly.

CREATE TABLE backup_settings (
    id            INTEGER PRIMARY KEY CHECK (id = 1),
    folder        TEXT NOT NULL DEFAULT '',
    keep_daily    INTEGER NOT NULL DEFAULT 14 CHECK (keep_daily >= 0),
    keep_monthly  INTEGER NOT NULL DEFAULT 12 CHECK (keep_monthly >= 0),
    created_at    TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at    TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_by    TEXT
);
