-- Phase 1: property configuration.
--
-- Conventions (see docs/REQUIREMENTS.md):
--   * UUID primary keys stored as TEXT (readable in any SQLite browser).
--   * created_at / updated_at as ISO-8601 UTC text; updated_by = user (from Phase 3).
--   * Soft delete via deleted_at; queries filter on `deleted_at IS NULL`.

-- Exactly one row (id = 1): hotel-wide settings. The only table without a
-- UUID key, because it is a singleton.
CREATE TABLE property_settings (
    id          INTEGER PRIMARY KEY CHECK (id = 1),
    hotel_name  TEXT NOT NULL,
    address     TEXT NOT NULL DEFAULT '',
    phone       TEXT NOT NULL DEFAULT '',
    email       TEXT NOT NULL DEFAULT '',
    currency    TEXT NOT NULL,
    timezone    TEXT NOT NULL,
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_by  TEXT
);

CREATE TABLE floors (
    id          TEXT PRIMARY KEY NOT NULL,
    name        TEXT NOT NULL,
    level       INTEGER NOT NULL,
    grid_cols   INTEGER NOT NULL CHECK (grid_cols BETWEEN 1 AND 200),
    grid_rows   INTEGER NOT NULL CHECK (grid_rows BETWEEN 1 AND 200),
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_by  TEXT,
    deleted_at  TEXT
);

CREATE TABLE room_types (
    id                 TEXT PRIMARY KEY NOT NULL,
    name               TEXT NOT NULL,
    code               TEXT NOT NULL,
    default_occupancy  INTEGER NOT NULL CHECK (default_occupancy >= 1),
    max_occupancy      INTEGER NOT NULL CHECK (max_occupancy >= default_occupancy),
    description        TEXT NOT NULL DEFAULT '',
    created_at         TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at         TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_by         TEXT,
    deleted_at         TEXT
);

-- Rooms and function suites share one table: both are bookable boxes on a floor plan.
CREATE TABLE spaces (
    id              TEXT PRIMARY KEY NOT NULL,
    floor_id        TEXT NOT NULL REFERENCES floors (id),
    kind            TEXT NOT NULL CHECK (kind IN ('room', 'suite')),
    name            TEXT NOT NULL,
    room_type_id    TEXT REFERENCES room_types (id),
    cap_theatre     INTEGER,
    cap_boardroom   INTEGER,
    cap_banquet     INTEGER,
    cap_cabaret     INTEGER,
    cap_reception   INTEGER,
    grid_x          INTEGER NOT NULL CHECK (grid_x >= 0),
    grid_y          INTEGER NOT NULL CHECK (grid_y >= 0),
    grid_w          INTEGER NOT NULL CHECK (grid_w >= 1),
    grid_h          INTEGER NOT NULL CHECK (grid_h >= 1),
    out_of_service  INTEGER NOT NULL DEFAULT 0,
    notes           TEXT NOT NULL DEFAULT '',
    created_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_by      TEXT,
    deleted_at      TEXT,
    -- Rooms need a type; suites must not have one (also enforced in booker-core).
    CHECK ((kind = 'room' AND room_type_id IS NOT NULL) OR (kind = 'suite' AND room_type_id IS NULL))
);

CREATE INDEX spaces_floor ON spaces (floor_id) WHERE deleted_at IS NULL;
-- Database backstop for the "names are unique" rule in booker-core.
CREATE UNIQUE INDEX spaces_name_unique ON spaces (lower(trim(name))) WHERE deleted_at IS NULL;
