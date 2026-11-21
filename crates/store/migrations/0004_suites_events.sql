-- Phase 4: function suite hire and events.
-- Times are ISO text 'YYYY-MM-DD HH:MM:SS' in hotel local time; money is pence.

-- A suite's rate card and default changeover time (NULL price = not offered).
ALTER TABLE spaces ADD COLUMN rate_hourly INTEGER CHECK (rate_hourly IS NULL OR rate_hourly >= 0);
ALTER TABLE spaces ADD COLUMN rate_min_hours INTEGER NOT NULL DEFAULT 0;
ALTER TABLE spaces ADD COLUMN rate_half_day INTEGER CHECK (rate_half_day IS NULL OR rate_half_day >= 0);
ALTER TABLE spaces ADD COLUMN rate_full_day INTEGER CHECK (rate_full_day IS NULL OR rate_full_day >= 0);
ALTER TABLE spaces ADD COLUMN setup_minutes INTEGER NOT NULL DEFAULT 60;
ALTER TABLE spaces ADD COLUMN cleardown_minutes INTEGER NOT NULL DEFAULT 30;

CREATE TABLE events (
    id                  TEXT PRIMARY KEY NOT NULL,
    name                TEXT NOT NULL,
    kind                TEXT NOT NULL CHECK (kind IN ('wedding', 'conference', 'dinner', 'party', 'other')),
    organiser_id        TEXT NOT NULL REFERENCES guests (id),
    start_date          TEXT NOT NULL,
    end_date            TEXT NOT NULL CHECK (end_date >= start_date),
    notes               TEXT NOT NULL DEFAULT '',
    running_order       TEXT NOT NULL DEFAULT '',
    -- Room block (all NULL when the event holds no rooms).
    block_room_type_id  TEXT REFERENCES room_types (id),
    block_rooms         INTEGER,
    block_check_in      TEXT,
    block_check_out     TEXT,
    block_rate          INTEGER,
    block_release_date  TEXT,
    created_at          TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at          TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_by          TEXT,
    deleted_at          TEXT
);
CREATE INDEX events_dates ON events (start_date, end_date) WHERE deleted_at IS NULL;

CREATE TABLE event_sittings (
    id          TEXT PRIMARY KEY NOT NULL,
    event_id    TEXT NOT NULL REFERENCES events (id) ON DELETE CASCADE,
    sort_order  INTEGER NOT NULL DEFAULT 0,
    name        TEXT NOT NULL,
    date        TEXT NOT NULL,
    time        TEXT NOT NULL,
    adults      INTEGER NOT NULL DEFAULT 0 CHECK (adults >= 0),
    children    INTEGER NOT NULL DEFAULT 0 CHECK (children >= 0),
    dietary     TEXT NOT NULL DEFAULT '',
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_by  TEXT
);
CREATE INDEX event_sittings_event ON event_sittings (event_id);

CREATE TABLE suite_bookings (
    id                 TEXT PRIMARY KEY NOT NULL,
    suite_id           TEXT NOT NULL REFERENCES spaces (id),
    event_id           TEXT REFERENCES events (id),
    guest_id           TEXT NOT NULL REFERENCES guests (id),
    title              TEXT NOT NULL,
    start_at           TEXT NOT NULL,
    end_at             TEXT NOT NULL CHECK (end_at > start_at),
    layout             TEXT NOT NULL CHECK (layout IN ('theatre', 'boardroom', 'banquet', 'cabaret', 'reception')),
    attendees          INTEGER NOT NULL CHECK (attendees >= 1),
    setup_minutes      INTEGER NOT NULL DEFAULT 0,
    cleardown_minutes  INTEGER NOT NULL DEFAULT 0,
    pricing_kind       TEXT NOT NULL CHECK (pricing_kind IN ('hourly', 'half_day', 'full_day', 'per_head', 'manual')),
    pricing_amount     INTEGER,
    -- Price fixed when booked, like room nights.
    price              INTEGER NOT NULL,
    status             TEXT NOT NULL CHECK (status IN ('provisional', 'confirmed', 'cancelled')),
    notes              TEXT NOT NULL DEFAULT '',
    created_at         TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at         TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_by         TEXT,
    deleted_at         TEXT
);
CREATE INDEX suite_bookings_suite_time ON suite_bookings (suite_id, start_at) WHERE deleted_at IS NULL;
CREATE INDEX suite_bookings_event ON suite_bookings (event_id);

-- Bedroom bookings made for an event (its room block, or guests' own rooms).
ALTER TABLE bookings ADD COLUMN event_id TEXT REFERENCES events (id);
