//! The report builder's data (ADR 0005): fact rows for a period, and saved
//! reports and dashboards. The figures themselves are worked out in
//! `booker_core::report_builder`.

use booker_core::{
    BoardOptionId, BookingId, ChargeFact, Dashboard, DashboardId, DashboardItem, DateRange,
    EventFact, GuestId, Money, PaymentFact, ReportFacts, ReportNames, RoomFact, RoomNightFact,
    SavedReport, SavedReportId, SpaceId, SpaceKind, StayFact, SuiteFact, validate_dashboard,
    validate_report_spec,
};

use crate::audit::{Change, json, record};
use crate::bookings::{date, load_boards, parse};
use crate::repo::ReportRepo;
use crate::rows::uuid;
use crate::sqlite::{load_floors, load_room_types, load_spaces};
use crate::{SqliteStore, StoreError, StoreResult};

/// A booked room-night with what the report needs from its booking.
type NightRow = (
    String,         // night
    String,         // booking id
    String,         // space id
    Option<String>, // board option id
    String,         // source
    String,         // status
    i64,            // adults + children
    i64,            // room price with supplements
    i64,            // board price
    i64,            // parking price
    Option<i64>,    // hand-set price for the whole stay
    String,         // check-in date
);

/// One room on a booking, with the guest's first ever arrival.
type StayRow = (
    String,         // booking id
    String,         // guest id
    String,         // space id
    Option<String>, // board option id
    String,         // source
    String,         // status
    String,         // check-in
    String,         // check-out
    String,         // day the booking was made
    Option<String>, // the guest's first arrival on a live booking
);

fn optional_board(id: Option<String>) -> StoreResult<Option<BoardOptionId>> {
    id.as_deref()
        .map(uuid)
        .transpose()
        .map(|u| u.map(BoardOptionId))
}

fn json_text<T: serde::Serialize>(value: &T) -> StoreResult<String> {
    serde_json::to_string(value).map_err(|e| StoreError::Corrupt(e.to_string()))
}

impl ReportRepo for SqliteStore {
    async fn report_facts(&self, period: DateRange) -> StoreResult<ReportFacts> {
        let mut conn = self.pool.acquire().await?;
        let (from, to) = (period.start.to_string(), period.end.to_string());

        // Rooms, and names for every kind of group.
        let spaces = load_spaces(&mut conn, None).await?;
        let mut names = ReportNames::default();
        for t in load_room_types(&mut conn).await? {
            names.room_types.insert(t.id, t.name);
        }
        for f in load_floors(&mut conn).await? {
            names.floors.insert(f.id, (f.name, f.level));
        }
        for b in load_boards(&mut conn).await? {
            names.boards.insert(b.id, b.name);
        }
        let mut rooms = Vec::new();
        for s in spaces {
            match s.kind {
                SpaceKind::Room => rooms.push(RoomFact {
                    space_id: s.id,
                    name: s.name,
                    room_type_id: s.room_type_id,
                    floor_id: s.floor_id,
                    in_service: !s.out_of_service,
                }),
                SpaceKind::Suite => {
                    names.suites.insert(s.id, s.name);
                }
            }
        }

        // Room-nights, priced as in the revenue report: a hand-set price
        // counts in full on the first night.
        let nights: Vec<NightRow> = sqlx::query_as(
            "SELECT n.night, b.id, n.space_id, r.board_option_id, b.source, b.status,
                    r.adults + r.children, n.room_price + n.supplements, n.board_price,
                    n.parking_price, r.price_override, r.check_in
             FROM room_nights n
             JOIN booking_rooms r ON r.id = n.booking_room_id
             JOIN bookings b ON b.id = r.booking_id
             WHERE n.night >= ? AND n.night < ?",
        )
        .bind(&from)
        .bind(&to)
        .fetch_all(&mut *conn)
        .await?;
        let mut room_nights = Vec::with_capacity(nights.len());
        for (
            night,
            booking,
            space,
            board,
            source,
            status,
            guests,
            room,
            board_price,
            parking,
            agreed,
            check_in,
        ) in nights
        {
            let night = date(&night)?;
            let (room, board_price, parking) = match agreed {
                Some(price) if night == date(&check_in)? => (price, 0, 0),
                Some(_) => (0, 0, 0),
                None => (room, board_price, parking),
            };
            room_nights.push(RoomNightFact {
                night,
                booking_id: BookingId(uuid(&booking)?),
                space_id: SpaceId(uuid(&space)?),
                board: optional_board(board)?,
                source: parse(&source)?,
                status: parse(&status)?,
                guests: u16::try_from(guests).unwrap_or(u16::MAX),
                room: Money(room),
                board_price: Money(board_price),
                parking: Money(parking),
            });
        }

        // Booked rooms arriving, leaving or booked in the period. The day a
        // booking was made is the UTC date it was saved.
        let stay_rows: Vec<StayRow> = sqlx::query_as(
            "SELECT b.id, b.guest_id, r.space_id, r.board_option_id, b.source, b.status,
                    r.check_in, r.check_out, substr(b.created_at, 1, 10),
                    (SELECT min(r2.check_in) FROM booking_rooms r2
                       JOIN bookings b2 ON b2.id = r2.booking_id
                      WHERE b2.guest_id = b.guest_id AND b2.deleted_at IS NULL
                        AND b2.status NOT IN ('cancelled', 'no_show'))
             FROM booking_rooms r JOIN bookings b ON b.id = r.booking_id
             WHERE b.deleted_at IS NULL
               AND ((r.check_in >= ?1 AND r.check_in < ?2)
                 OR (r.check_out >= ?1 AND r.check_out < ?2)
                 OR (substr(b.created_at, 1, 10) >= ?1 AND substr(b.created_at, 1, 10) < ?2))",
        )
        .bind(&from)
        .bind(&to)
        .fetch_all(&mut *conn)
        .await?;
        let mut stays = Vec::with_capacity(stay_rows.len());
        for (booking, guest, space, board, source, status, check_in, check_out, made, first) in
            stay_rows
        {
            stays.push(StayFact {
                booking_id: BookingId(uuid(&booking)?),
                guest_id: GuestId(uuid(&guest)?),
                space_id: SpaceId(uuid(&space)?),
                board: optional_board(board)?,
                source: parse(&source)?,
                status: parse(&status)?,
                arrival: date(&check_in)?,
                departure: date(&check_out)?,
                booked_on: date(&made)?,
                guest_first_arrival: first.as_deref().map(date).transpose()?,
            });
        }

        let payment_rows: Vec<(String, i64, String, bool)> = sqlx::query_as(
            "SELECT date, amount, method, is_deposit FROM payments
             WHERE deleted_at IS NULL AND date >= ? AND date < ?",
        )
        .bind(&from)
        .bind(&to)
        .fetch_all(&mut *conn)
        .await?;
        let payments = payment_rows
            .into_iter()
            .map(|(day, amount, method, deposit)| {
                Ok(PaymentFact {
                    date: date(&day)?,
                    amount: Money(amount),
                    method: parse(&method)?,
                    deposit,
                })
            })
            .collect::<StoreResult<Vec<_>>>()?;

        let charge_rows: Vec<(String, i64)> = sqlx::query_as(
            "SELECT date, quantity * unit_price FROM charges
             WHERE deleted_at IS NULL AND date >= ? AND date < ?",
        )
        .bind(&from)
        .bind(&to)
        .fetch_all(&mut *conn)
        .await?;
        let charges = charge_rows
            .into_iter()
            .map(|(day, amount)| {
                Ok(ChargeFact {
                    date: date(&day)?,
                    amount: Money(amount),
                })
            })
            .collect::<StoreResult<Vec<_>>>()?;

        // Suite hire that wasn't cancelled, on the day it starts.
        let suite_rows: Vec<(String, String, i64, i64, Option<String>)> = sqlx::query_as(
            "SELECT substr(s.start_at, 1, 10), s.suite_id,
                    CAST(round((julianday(s.end_at) - julianday(s.start_at)) * 1440) AS INTEGER),
                    s.price, e.kind
             FROM suite_bookings s
             LEFT JOIN events e ON e.id = s.event_id AND e.deleted_at IS NULL
             WHERE s.deleted_at IS NULL AND s.status <> 'cancelled'
               AND s.start_at >= ? AND s.start_at < ?",
        )
        .bind(&from)
        .bind(&to)
        .fetch_all(&mut *conn)
        .await?;
        let suites = suite_rows
            .into_iter()
            .map(|(day, suite, minutes, price, kind)| {
                Ok(SuiteFact {
                    date: date(&day)?,
                    suite_id: SpaceId(uuid(&suite)?),
                    minutes,
                    price: Money(price),
                    event_kind: kind.as_deref().map(parse).transpose()?,
                })
            })
            .collect::<StoreResult<Vec<_>>>()?;

        // Events by start date. Attendees: the largest catering sitting, or
        // the largest suite hire if there are no sittings.
        let event_rows: Vec<(String, String, i64, i64)> = sqlx::query_as(
            "SELECT e.start_date, e.kind,
                    coalesce((SELECT max(adults + children) FROM event_sittings
                               WHERE event_id = e.id), 0),
                    coalesce((SELECT max(attendees) FROM suite_bookings
                               WHERE event_id = e.id AND deleted_at IS NULL
                                 AND status <> 'cancelled'), 0)
             FROM events e
             WHERE e.deleted_at IS NULL AND e.start_date >= ? AND e.start_date < ?",
        )
        .bind(&from)
        .bind(&to)
        .fetch_all(&mut *conn)
        .await?;
        let events = event_rows
            .into_iter()
            .map(|(day, kind, sitting, hire)| {
                let attendees = if sitting > 0 { sitting } else { hire };
                Ok(EventFact {
                    date: date(&day)?,
                    kind: parse(&kind)?,
                    attendees: u32::try_from(attendees).unwrap_or(0),
                })
            })
            .collect::<StoreResult<Vec<_>>>()?;

        Ok(ReportFacts {
            rooms,
            room_nights,
            stays,
            payments,
            charges,
            suites,
            events,
            names,
        })
    }

    async fn list_saved_reports(&self) -> StoreResult<Vec<SavedReport>> {
        let rows: Vec<(String, String)> =
            sqlx::query_as("SELECT id, spec FROM saved_reports ORDER BY name COLLATE NOCASE")
                .fetch_all(&self.pool)
                .await?;
        rows.into_iter()
            .map(|(id, spec)| {
                Ok(SavedReport {
                    id: SavedReportId(uuid(&id)?),
                    spec: serde_json::from_str(&spec)
                        .map_err(|e| StoreError::Corrupt(format!("saved report {id}: {e}")))?,
                })
            })
            .collect()
    }

    async fn save_saved_report(&self, report: &SavedReport) -> StoreResult<()> {
        validate_report_spec(&report.spec)?;
        let mut tx = self.pool.begin().await?;
        let before: Option<String> =
            sqlx::query_scalar("SELECT spec FROM saved_reports WHERE id = ?")
                .bind(report.id.to_string())
                .fetch_optional(&mut *tx)
                .await?;
        let name = report.spec.name.trim();
        sqlx::query(
            "INSERT INTO saved_reports (id, name, spec, updated_by) VALUES (?, ?, ?, ?)
             ON CONFLICT (id) DO UPDATE SET name = excluded.name, spec = excluded.spec,
                updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                updated_by = excluded.updated_by",
        )
        .bind(report.id.to_string())
        .bind(name)
        .bind(json_text(&report.spec)?)
        .bind(
            self.actor
                .as_ref()
                .and_then(|a| a.user_id)
                .map(|id| id.to_string()),
        )
        .execute(&mut *tx)
        .await?;
        record(
            &mut tx,
            Change {
                entity: "saved_report",
                entity_id: report.id.to_string(),
                action: if before.is_some() { "update" } else { "create" },
                summary: format!("Saved report \"{name}\""),
                before,
                after: json(&report.spec),
                actor: self.actor.as_ref(),
            },
        )
        .await?;
        tx.commit().await?;
        Ok(())
    }

    async fn delete_saved_report(&self, id: SavedReportId) -> StoreResult<()> {
        let mut tx = self.pool.begin().await?;
        let row: Option<(String, String)> =
            sqlx::query_as("SELECT name, spec FROM saved_reports WHERE id = ?")
                .bind(id.to_string())
                .fetch_optional(&mut *tx)
                .await?;
        let Some((name, spec)) = row else {
            return Err(StoreError::NotFound { what: "Report" });
        };
        // Its dashboard places go too (ON DELETE CASCADE).
        sqlx::query("DELETE FROM saved_reports WHERE id = ?")
            .bind(id.to_string())
            .execute(&mut *tx)
            .await?;
        record(
            &mut tx,
            Change {
                entity: "saved_report",
                entity_id: id.to_string(),
                action: "delete",
                summary: format!("Deleted report \"{name}\""),
                before: Some(spec),
                after: None,
                actor: self.actor.as_ref(),
            },
        )
        .await?;
        tx.commit().await?;
        Ok(())
    }

    async fn list_dashboards(&self) -> StoreResult<Vec<Dashboard>> {
        let mut conn = self.pool.acquire().await?;
        let boards: Vec<(String, String)> =
            sqlx::query_as("SELECT id, name FROM dashboards ORDER BY name COLLATE NOCASE")
                .fetch_all(&mut *conn)
                .await?;
        let items: Vec<(String, String, bool)> = sqlx::query_as(
            "SELECT dashboard_id, report_id, wide FROM dashboard_items ORDER BY position",
        )
        .fetch_all(&mut *conn)
        .await?;
        boards
            .into_iter()
            .map(|(id, name)| {
                let dashboard_items = items
                    .iter()
                    .filter(|(board, _, _)| *board == id)
                    .map(|(_, report, wide)| {
                        Ok(DashboardItem {
                            report_id: SavedReportId(uuid(report)?),
                            wide: *wide,
                        })
                    })
                    .collect::<StoreResult<Vec<_>>>()?;
                Ok(Dashboard {
                    id: DashboardId(uuid(&id)?),
                    name,
                    items: dashboard_items,
                })
            })
            .collect()
    }

    async fn save_dashboard(&self, dashboard: &Dashboard) -> StoreResult<()> {
        validate_dashboard(dashboard)?;
        let mut tx = self.pool.begin().await?;
        for item in &dashboard.items {
            let exists: Option<i64> =
                sqlx::query_scalar("SELECT 1 FROM saved_reports WHERE id = ?")
                    .bind(item.report_id.to_string())
                    .fetch_optional(&mut *tx)
                    .await?;
            if exists.is_none() {
                return Err(StoreError::NotFound { what: "Report" });
            }
        }
        let existed: Option<String> =
            sqlx::query_scalar("SELECT name FROM dashboards WHERE id = ?")
                .bind(dashboard.id.to_string())
                .fetch_optional(&mut *tx)
                .await?;
        let by = self
            .actor
            .as_ref()
            .and_then(|a| a.user_id)
            .map(|id| id.to_string());
        let name = dashboard.name.trim();
        sqlx::query(
            "INSERT INTO dashboards (id, name, updated_by) VALUES (?, ?, ?)
             ON CONFLICT (id) DO UPDATE SET name = excluded.name,
                updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                updated_by = excluded.updated_by",
        )
        .bind(dashboard.id.to_string())
        .bind(name)
        .bind(&by)
        .execute(&mut *tx)
        .await?;
        // Replace the list: simpler than working out moves, and it's short.
        sqlx::query("DELETE FROM dashboard_items WHERE dashboard_id = ?")
            .bind(dashboard.id.to_string())
            .execute(&mut *tx)
            .await?;
        for (position, item) in dashboard.items.iter().enumerate() {
            sqlx::query(
                "INSERT INTO dashboard_items (id, dashboard_id, report_id, position, wide, updated_by)
                 VALUES (?, ?, ?, ?, ?, ?)",
            )
            .bind(uuid::Uuid::now_v7().to_string())
            .bind(dashboard.id.to_string())
            .bind(item.report_id.to_string())
            .bind(i64::try_from(position).unwrap_or(i64::MAX))
            .bind(item.wide)
            .bind(&by)
            .execute(&mut *tx)
            .await?;
        }
        record(
            &mut tx,
            Change {
                entity: "dashboard",
                entity_id: dashboard.id.to_string(),
                action: if existed.is_some() {
                    "update"
                } else {
                    "create"
                },
                summary: format!(
                    "Saved dashboard \"{name}\" ({} report{})",
                    dashboard.items.len(),
                    if dashboard.items.len() == 1 { "" } else { "s" }
                ),
                before: None,
                after: json(dashboard),
                actor: self.actor.as_ref(),
            },
        )
        .await?;
        tx.commit().await?;
        Ok(())
    }

    async fn delete_dashboard(&self, id: DashboardId) -> StoreResult<()> {
        let mut tx = self.pool.begin().await?;
        let name: Option<String> = sqlx::query_scalar("SELECT name FROM dashboards WHERE id = ?")
            .bind(id.to_string())
            .fetch_optional(&mut *tx)
            .await?;
        let Some(name) = name else {
            return Err(StoreError::NotFound { what: "Dashboard" });
        };
        sqlx::query("DELETE FROM dashboards WHERE id = ?")
            .bind(id.to_string())
            .execute(&mut *tx)
            .await?;
        record(
            &mut tx,
            Change {
                entity: "dashboard",
                entity_id: id.to_string(),
                action: "delete",
                summary: format!("Deleted dashboard \"{name}\""),
                before: None,
                after: None,
                actor: self.actor.as_ref(),
            },
        )
        .await?;
        tx.commit().await?;
        Ok(())
    }
}
