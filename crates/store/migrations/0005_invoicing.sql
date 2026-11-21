-- Phase 5: tax rates, folio charges and payments, invoices.
-- Money is pence; tax rates are basis points (2000 = 20%).
-- A folio belongs to a booking, a suite booking or an event: owner_kind + owner_id.

CREATE TABLE tax_rates (
    id            TEXT PRIMARY KEY NOT NULL,
    name          TEXT NOT NULL,
    basis_points  INTEGER NOT NULL CHECK (basis_points BETWEEN 0 AND 10000),
    created_at    TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at    TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_by    TEXT,
    deleted_at    TEXT
);

-- One row (id = 1), like property_settings.
CREATE TABLE invoice_settings (
    id                   INTEGER PRIMARY KEY CHECK (id = 1),
    prices_include_tax   INTEGER NOT NULL DEFAULT 1,
    prefix               TEXT NOT NULL DEFAULT 'INV-',
    footer               TEXT NOT NULL DEFAULT '',
    accommodation_rate   TEXT REFERENCES tax_rates (id),
    board_rate           TEXT REFERENCES tax_rates (id),
    suite_rate           TEXT REFERENCES tax_rates (id),
    food_rate            TEXT REFERENCES tax_rates (id),
    drink_rate           TEXT REFERENCES tax_rates (id),
    other_rate           TEXT REFERENCES tax_rates (id),
    created_at           TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at           TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_by           TEXT
);

-- Extras added by hand (bar, restaurant…). Stay and suite charges are worked
-- out from room_nights and suite_bookings, not stored here.
CREATE TABLE charges (
    id                TEXT PRIMARY KEY NOT NULL,
    owner_kind        TEXT NOT NULL CHECK (owner_kind IN ('booking', 'suite_booking', 'event')),
    owner_id          TEXT NOT NULL,
    date              TEXT NOT NULL,
    category          TEXT NOT NULL CHECK (category IN ('accommodation', 'board', 'suite', 'food', 'drink', 'other')),
    description       TEXT NOT NULL,
    quantity          INTEGER NOT NULL CHECK (quantity >= 1),
    unit_price        INTEGER NOT NULL,
    tax_basis_points  INTEGER NOT NULL CHECK (tax_basis_points BETWEEN 0 AND 10000),
    created_at        TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at        TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_by        TEXT,
    deleted_at        TEXT
);
CREATE INDEX charges_owner ON charges (owner_kind, owner_id) WHERE deleted_at IS NULL;

CREATE TABLE payments (
    id          TEXT PRIMARY KEY NOT NULL,
    owner_kind  TEXT NOT NULL CHECK (owner_kind IN ('booking', 'suite_booking', 'event')),
    owner_id    TEXT NOT NULL,
    date        TEXT NOT NULL,
    amount      INTEGER NOT NULL CHECK (amount <> 0),
    method      TEXT NOT NULL CHECK (method IN ('cash', 'card', 'bank_transfer', 'cheque', 'voucher', 'other')),
    is_deposit  INTEGER NOT NULL DEFAULT 0,
    reference   TEXT NOT NULL DEFAULT '',
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_by  TEXT,
    deleted_at  TEXT
);
CREATE INDEX payments_owner ON payments (owner_kind, owner_id) WHERE deleted_at IS NULL;

-- Issued invoices never change: `document` is the frozen invoice as JSON.
CREATE TABLE invoices (
    id          TEXT PRIMARY KEY NOT NULL,
    number      TEXT NOT NULL UNIQUE,
    owner_kind  TEXT NOT NULL CHECK (owner_kind IN ('booking', 'suite_booking', 'event')),
    owner_id    TEXT NOT NULL,
    issued_on   TEXT NOT NULL,
    gross       INTEGER NOT NULL,
    document    TEXT NOT NULL,
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_by  TEXT
);
CREATE INDEX invoices_owner ON invoices (owner_kind, owner_id);

-- The last invoice number used. Taken in the same transaction as the
-- invoice it numbers, so numbers have no gaps.
CREATE TABLE invoice_counter (
    id           INTEGER PRIMARY KEY CHECK (id = 1),
    last_number  INTEGER NOT NULL
);
