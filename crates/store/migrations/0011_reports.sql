-- Phase 9: the report builder (ADR 0005). A saved report keeps its spec
-- (measure, split, period, chart) as JSON, never its figures, so it always
-- shows current data. A dashboard is an ordered list of saved reports.

CREATE TABLE saved_reports (
    id          TEXT PRIMARY KEY NOT NULL,
    name        TEXT NOT NULL,
    spec        TEXT NOT NULL,
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_by  TEXT
);

CREATE TABLE dashboards (
    id          TEXT PRIMARY KEY NOT NULL,
    name        TEXT NOT NULL,
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_by  TEXT
);

CREATE TABLE dashboard_items (
    id            TEXT PRIMARY KEY NOT NULL,
    dashboard_id  TEXT NOT NULL REFERENCES dashboards (id) ON DELETE CASCADE,
    report_id     TEXT NOT NULL REFERENCES saved_reports (id) ON DELETE CASCADE,
    position      INTEGER NOT NULL,
    wide          INTEGER NOT NULL DEFAULT 0,
    created_at    TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at    TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_by    TEXT
);
CREATE INDEX dashboard_items_dashboard ON dashboard_items (dashboard_id, position);
