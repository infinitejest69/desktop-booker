//! SQLite implementation of housekeeping, reports and guest records.

use booker_core::{
    BookingId, BookingListItem, BookingStatus, DateRange, DayOccupancy, GuestId, GuestProfile,
    HousekeepingStatus, Money, MonthRevenue, RevenueSource, RoomHousekeeping, SpaceId, SpaceKind,
    anonymised, occupancy_by_day, revenue_by_month,
};
use chrono::NaiveDate;
use sqlx::SqliteConnection;

use crate::audit::{Change, json, record};
use crate::bookings::{date, detail, load_guest, parse, upsert_guest};
use crate::repo::OperationsRepo;
use crate::rows::uuid;
use crate::sqlite::load_spaces;
use crate::{SqliteStore, StoreError, StoreResult};

/// Sets a room's housekeeping status inside the caller's transaction
/// (also used when a booking is checked out).
pub(crate) async fn mark_room(
    conn: &mut SqliteConnection,
    space: SpaceId,
    status: HousekeepingStatus,
    note: &str,
) -> StoreResult<()> {
    sqlx::query(
        "INSERT INTO housekeeping (id, space_id, status, note) VALUES (?, ?, ?, ?)
         ON CONFLICT (space_id) DO UPDATE SET status = excluded.status, note = excluded.note,
            updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')",
    )
    .bind(uuid::Uuid::now_v7().to_string())
    .bind(space.to_string())
    .bind(status.as_str())
    .bind(note.trim())
    .execute(conn)
    .await?;
    Ok(())
}

impl OperationsRepo for SqliteStore {
    async fn list_housekeeping(&self) -> StoreResult<Vec<RoomHousekeeping>> {
        let mut conn = self.pool.acquire().await?;
        let spaces = load_spaces(&mut conn, None).await?;
        let rows: Vec<(String, String, String)> =
            sqlx::query_as("SELECT space_id, status, note FROM housekeeping")
                .fetch_all(&mut *conn)
                .await?;
        spaces
            .iter()
            .filter(|s| s.kind == SpaceKind::Room)
            .map(|s| {
                let saved = rows.iter().find(|(id, _, _)| *id == s.id.to_string());
                Ok(RoomHousekeeping {
                    space_id: s.id,
                    status: match saved {
                        Some((_, status, _)) => parse(status)?,
                        None => HousekeepingStatus::default(),
                    },
                    note: saved.map(|(_, _, n)| n.clone()).unwrap_or_default(),
                })
            })
            .collect()
    }

    async fn set_housekeeping(
        &self,
        space: SpaceId,
        status: HousekeepingStatus,
        note: &str,
    ) -> StoreResult<()> {
        let mut tx = self.pool.begin().await?;
        let spaces = load_spaces(&mut tx, None).await?;
        let room = spaces
            .iter()
            .find(|s| s.id == space && s.kind == SpaceKind::Room)
            .ok_or(StoreError::NotFound { what: "Room" })?;
        mark_room(&mut tx, space, status, note).await?;
        record(
            &mut tx,
            Change {
                entity: "housekeeping",
                entity_id: space.to_string(),
                action: "updated",
                summary: format!("Marked room {} {}", room.name, status.as_str()),
                before: None,
                after: None,
                actor: self.actor.as_ref(),
            },
        )
        .await?;
        tx.commit().await?;
        Ok(())
    }

    async fn occupancy_report(&self, period: DateRange) -> StoreResult<Vec<DayOccupancy>> {
        let mut conn = self.pool.acquire().await?;
        let spaces = load_spaces(&mut conn, None).await?;
        let in_service = spaces
            .iter()
            .filter(|s| s.kind == SpaceKind::Room && !s.out_of_service)
            .count();
        // room_nights holds a row per room per night for every live booking.
        let rows: Vec<(String, String)> = sqlx::query_as(
            "SELECT night, space_id FROM room_nights WHERE night >= ? AND night < ?",
        )
        .bind(period.start.to_string())
        .bind(period.end.to_string())
        .fetch_all(&mut *conn)
        .await?;
        let nights = rows
            .into_iter()
            .map(|(n, s)| Ok((date(&n)?, SpaceId(uuid(&s)?))))
            .collect::<StoreResult<Vec<_>>>()?;
        Ok(occupancy_by_day(
            period,
            u32::try_from(in_service).unwrap_or(u32::MAX),
            &nights,
        ))
    }

    async fn revenue_report(&self, period: DateRange) -> StoreResult<Vec<MonthRevenue>> {
        let mut conn = self.pool.acquire().await?;
        let (from, to) = (period.start.to_string(), period.end.to_string());
        let mut items: Vec<(NaiveDate, RevenueSource, Money)> = Vec::new();

        // Rooms and board, as priced night by night when booked. A room
        // with a hand-set price counts that price on its first night.
        let nights: Vec<(String, i64, i64, Option<i64>, String)> = sqlx::query_as(
            "SELECT n.night, n.room_price + n.supplements, n.board_price, r.price_override,
                    r.check_in
             FROM room_nights n JOIN booking_rooms r ON r.id = n.booking_room_id
             WHERE n.night >= ? AND n.night < ?",
        )
        .bind(&from)
        .bind(&to)
        .fetch_all(&mut *conn)
        .await?;
        for (night, room, board, agreed, check_in) in nights {
            let night = date(&night)?;
            match agreed {
                Some(price) if night == date(&check_in)? => {
                    items.push((night, RevenueSource::Rooms, Money(price)));
                }
                Some(_) => {}
                None => {
                    items.push((night, RevenueSource::Rooms, Money(room)));
                    items.push((night, RevenueSource::Board, Money(board)));
                }
            }
        }

        let suites: Vec<(String, i64)> = sqlx::query_as(
            "SELECT substr(start_at, 1, 10), price FROM suite_bookings
             WHERE deleted_at IS NULL AND status <> 'cancelled'
               AND start_at >= ? AND start_at < ?",
        )
        .bind(&from)
        .bind(&to)
        .fetch_all(&mut *conn)
        .await?;
        for (day, price) in suites {
            items.push((date(&day)?, RevenueSource::Suites, Money(price)));
        }

        let extras: Vec<(String, i64)> = sqlx::query_as(
            "SELECT date, quantity * unit_price FROM charges
             WHERE deleted_at IS NULL AND date >= ? AND date < ?",
        )
        .bind(&from)
        .bind(&to)
        .fetch_all(&mut *conn)
        .await?;
        for (day, amount) in extras {
            items.push((date(&day)?, RevenueSource::Extras, Money(amount)));
        }
        Ok(revenue_by_month(&items))
    }

    async fn guest_profile(&self, id: GuestId) -> StoreResult<GuestProfile> {
        let mut conn = self.pool.acquire().await?;
        let guest = load_guest(&mut conn, id).await?;
        let rows: Vec<(String, String, String, String, String, String)> = sqlx::query_as(
            "SELECT b.id, b.reference, b.status, min(r.check_in), max(r.check_out),
                    group_concat(s.name, ', ')
             FROM bookings b
             JOIN booking_rooms r ON r.booking_id = b.id
             JOIN spaces s ON s.id = r.space_id
             WHERE b.guest_id = ? AND b.deleted_at IS NULL
             GROUP BY b.id ORDER BY min(r.check_in) DESC",
        )
        .bind(id.to_string())
        .fetch_all(&mut *conn)
        .await?;
        let mut stays = Vec::new();
        let mut spend = Money::ZERO;
        for (booking_id, reference, status, arrival, departure, rooms) in rows {
            let booking = BookingId(uuid(&booking_id)?);
            let status: BookingStatus = parse(&status)?;
            if status.holds_rooms() {
                spend += detail(&mut conn, booking).await?.total;
                let extras: Option<i64> = sqlx::query_scalar(
                    "SELECT sum(quantity * unit_price) FROM charges
                     WHERE owner_kind = 'booking' AND owner_id = ? AND deleted_at IS NULL",
                )
                .bind(&booking_id)
                .fetch_one(&mut *conn)
                .await?;
                spend += Money(extras.unwrap_or(0));
            }
            let rooms: Vec<String> = rooms.split(", ").map(str::to_string).collect();
            stays.push(BookingListItem {
                id: booking,
                reference,
                guest_name: guest.display_name(),
                status,
                arrival: date(&arrival)?,
                departure: date(&departure)?,
                boards: vec![None; rooms.len()],
                rooms,
            });
        }
        Ok(GuestProfile {
            guest,
            stays,
            total_spend: spend,
        })
    }

    async fn erase_guest(&self, id: GuestId) -> StoreResult<()> {
        let mut tx = self.pool.begin().await?;
        let guest = load_guest(&mut tx, id).await?;
        let erased = anonymised(&guest);
        upsert_guest(&mut tx, &erased).await?;
        record(
            &mut tx,
            Change {
                entity: "guest",
                entity_id: id.to_string(),
                action: "erased",
                // The log is kept, so the summary mustn't name them.
                summary: "Erased a guest's personal details".into(),
                before: None,
                after: json(&erased),
                actor: self.actor.as_ref(),
            },
        )
        .await?;
        tx.commit().await?;
        Ok(())
    }
}
