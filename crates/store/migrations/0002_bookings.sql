-- Phase 2: rates, guests and room bookings.
-- Dates are ISO text 'YYYY-MM-DD'; money is INTEGER minor units (pence).

ALTER TABLE property_settings ADD COLUMN check_in_time TEXT NOT NULL DEFAULT '15:00';
ALTER TABLE property_settings ADD COLUMN check_out_time TEXT NOT NULL DEFAULT '11:00';

ALTER TABLE room_types ADD COLUMN base_rate INTEGER NOT NULL DEFAULT 0 CHECK (base_rate >= 0);
ALTER TABLE room_types ADD COLUMN extra_adult_rate INTEGER NOT NULL DEFAULT 0 CHECK (extra_adult_rate >= 0);
ALTER TABLE room_types ADD COLUMN child_rate INTEGER NOT NULL DEFAULT 0 CHECK (child_rate >= 0);

CREATE TABLE rate_rules (
    id                TEXT PRIMARY KEY NOT NULL,
    name              TEXT NOT NULL,
    room_type_id      TEXT REFERENCES room_types (id),
    start_date        TEXT NOT NULL,
    end_date          TEXT NOT NULL CHECK (end_date >= start_date),
    -- ISO weekdays as a comma list, e.g. '5,6' for Fri+Sat; '' = every day.
    weekdays          TEXT NOT NULL DEFAULT '',
    priority          INTEGER NOT NULL DEFAULT 0,
    adjustment_kind   TEXT NOT NULL CHECK (adjustment_kind IN ('set_price', 'add_amount', 'percent')),
    adjustment_value  INTEGER NOT NULL,
    active            INTEGER NOT NULL DEFAULT 1,
    created_at        TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at        TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_by        TEXT,
    deleted_at        TEXT
);

CREATE TABLE board_options (
    id           TEXT PRIMARY KEY NOT NULL,
    code         TEXT NOT NULL,
    name         TEXT NOT NULL,
    adult_price  INTEGER NOT NULL CHECK (adult_price >= 0),
    child_price  INTEGER NOT NULL CHECK (child_price >= 0),
    created_at   TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at   TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_by   TEXT,
    deleted_at   TEXT
);
CREATE UNIQUE INDEX board_options_code_unique ON board_options (upper(trim(code))) WHERE deleted_at IS NULL;

CREATE TABLE guests (
    id          TEXT PRIMARY KEY NOT NULL,
    first_name  TEXT NOT NULL DEFAULT '',
    last_name   TEXT NOT NULL DEFAULT '',
    email       TEXT NOT NULL DEFAULT '',
    phone       TEXT NOT NULL DEFAULT '',
    address     TEXT NOT NULL DEFAULT '',
    notes       TEXT NOT NULL DEFAULT '',
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_by  TEXT,
    deleted_at  TEXT
);
CREATE INDEX guests_last_name ON guests (lower(last_name)) WHERE deleted_at IS NULL;

-- Next booking number per year, for references like 2026-0001.
-- Keyed by year rather than a UUID: it's a counter, not an entity.
CREATE TABLE booking_counters (
    year         INTEGER PRIMARY KEY,
    last_number  INTEGER NOT NULL
);

CREATE TABLE bookings (
    id          TEXT PRIMARY KEY NOT NULL,
    reference   TEXT NOT NULL UNIQUE,
    guest_id    TEXT NOT NULL REFERENCES guests (id),
    status      TEXT NOT NULL CHECK (status IN
                  ('provisional', 'confirmed', 'checked_in', 'checked_out', 'cancelled', 'no_show')),
    source      TEXT NOT NULL CHECK (source IN
                  ('phone', 'email', 'walk_in', 'website', 'agent', 'other')),
    notes       TEXT NOT NULL DEFAULT '',
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_by  TEXT,
    deleted_at  TEXT
);
CREATE INDEX bookings_guest ON bookings (guest_id);

CREATE TABLE booking_rooms (
    id               TEXT PRIMARY KEY NOT NULL,
    booking_id       TEXT NOT NULL REFERENCES bookings (id) ON DELETE CASCADE,
    space_id         TEXT NOT NULL REFERENCES spaces (id),
    check_in         TEXT NOT NULL,
    check_out        TEXT NOT NULL CHECK (check_out > check_in),
    adults           INTEGER NOT NULL CHECK (adults >= 1),
    children         INTEGER NOT NULL DEFAULT 0 CHECK (children >= 0),
    board_option_id  TEXT REFERENCES board_options (id),
    price_override   INTEGER CHECK (price_override IS NULL OR price_override >= 0),
    sort_order       INTEGER NOT NULL DEFAULT 0,
    created_at       TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at       TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_by       TEXT
);
CREATE INDEX booking_rooms_booking ON booking_rooms (booking_id);
CREATE INDEX booking_rooms_dates ON booking_rooms (check_in, check_out);

-- One row per room per night held, with the price fixed when booked.
-- Rows exist only while the booking holds its rooms (not cancelled / no-show).
CREATE TABLE room_nights (
    booking_room_id  TEXT NOT NULL REFERENCES booking_rooms (id) ON DELETE CASCADE,
    space_id         TEXT NOT NULL REFERENCES spaces (id),
    night            TEXT NOT NULL,
    room_price       INTEGER NOT NULL,
    supplements      INTEGER NOT NULL,
    board_price      INTEGER NOT NULL,
    total            INTEGER NOT NULL,
    PRIMARY KEY (booking_room_id, night)
);
-- The database's own guarantee against double-booking.
CREATE UNIQUE INDEX room_nights_no_double_booking ON room_nights (space_id, night);
