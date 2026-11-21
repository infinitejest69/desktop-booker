//! SQLite implementation of rates, guests and bookings.

use std::collections::HashMap;

use booker_core::{
    Adjustment, BoardOption, BoardOptionId, Booking, BookingDetail, BookingId, BookingListItem,
    BookingRoom, BookingRoomId, BookingStatus, ChartEntry, DateRange, Guest, GuestId, Money,
    NightPrice, Occupancy, RateRule, RateRuleId, RoomPricing, RoomTypeId, SpaceId, StayQuote,
    ValidationError, format_reference, free_rooms, quote_stay, validate_board_option,
    validate_booking, validate_guest, validate_rate_rule,
};
use chrono::{Datelike, NaiveDate};
use sqlx::SqliteConnection;

use crate::audit::{Change, json, record};
use crate::repo::{BookingRepo, GuestRepo, RateRepo};
use crate::rows::{int, uuid};
use crate::sqlite::{load_room_types, load_spaces};
use crate::{SqliteStore, StoreError, StoreResult};

pub(crate) fn date(s: &str) -> StoreResult<NaiveDate> {
    s.parse()
        .map_err(|e| StoreError::Corrupt(format!("bad date {s:?}: {e}")))
}

pub(crate) fn parse<T: std::str::FromStr<Err = String>>(s: &str) -> StoreResult<T> {
    s.parse().map_err(StoreError::Corrupt)
}

pub(crate) fn full_name(first: &str, last: &str) -> String {
    format!("{} {}", first.trim(), last.trim())
        .trim()
        .to_string()
}

// ---------------------------------------------------------------------------
// Rows
// ---------------------------------------------------------------------------

#[derive(sqlx::FromRow)]
struct RateRuleRow {
    id: String,
    name: String,
    room_type_id: Option<String>,
    start_date: String,
    end_date: String,
    weekdays: String,
    priority: i64,
    adjustment_kind: String,
    adjustment_value: i64,
    active: bool,
}

impl TryFrom<RateRuleRow> for RateRule {
    type Error = StoreError;
    fn try_from(r: RateRuleRow) -> StoreResult<Self> {
        let adjustment = match r.adjustment_kind.as_str() {
            "set_price" => Adjustment::SetPrice {
                amount: Money(r.adjustment_value),
            },
            "add_amount" => Adjustment::AddAmount {
                amount: Money(r.adjustment_value),
            },
            "percent" => Adjustment::Percent {
                percent: int(r.adjustment_value, "percent")?,
            },
            other => return Err(StoreError::Corrupt(format!("unknown adjustment {other:?}"))),
        };
        let weekdays = r
            .weekdays
            .split(',')
            .filter(|p| !p.is_empty())
            .map(|p| {
                p.parse::<u8>()
                    .map_err(|e| StoreError::Corrupt(format!("weekday {p:?}: {e}")))
            })
            .collect::<StoreResult<Vec<u8>>>()?;
        Ok(RateRule {
            id: RateRuleId(uuid(&r.id)?),
            name: r.name,
            room_type_id: r
                .room_type_id
                .as_deref()
                .map(uuid)
                .transpose()?
                .map(RoomTypeId),
            start: date(&r.start_date)?,
            end: date(&r.end_date)?,
            weekdays,
            priority: int(r.priority, "rate_rules.priority")?,
            adjustment,
            active: r.active,
        })
    }
}

fn adjustment_columns(a: Adjustment) -> (&'static str, i64) {
    match a {
        Adjustment::SetPrice { amount } => ("set_price", amount.0),
        Adjustment::AddAmount { amount } => ("add_amount", amount.0),
        Adjustment::Percent { percent } => ("percent", i64::from(percent)),
    }
}

#[derive(sqlx::FromRow)]
struct BoardRow {
    id: String,
    code: String,
    name: String,
    adult_price: i64,
    child_price: i64,
}

impl TryFrom<BoardRow> for BoardOption {
    type Error = StoreError;
    fn try_from(r: BoardRow) -> StoreResult<Self> {
        Ok(BoardOption {
            id: BoardOptionId(uuid(&r.id)?),
            code: r.code,
            name: r.name,
            adult_price: Money(r.adult_price),
            child_price: Money(r.child_price),
        })
    }
}

#[derive(sqlx::FromRow)]
struct GuestRow {
    id: String,
    first_name: String,
    last_name: String,
    email: String,
    phone: String,
    address: String,
    notes: String,
}

impl TryFrom<GuestRow> for Guest {
    type Error = StoreError;
    fn try_from(r: GuestRow) -> StoreResult<Self> {
        Ok(Guest {
            id: GuestId(uuid(&r.id)?),
            first_name: r.first_name,
            last_name: r.last_name,
            email: r.email,
            phone: r.phone,
            address: r.address,
            notes: r.notes,
        })
    }
}

#[derive(sqlx::FromRow)]
struct BookingRow {
    id: String,
    reference: String,
    guest_id: String,
    status: String,
    source: String,
    notes: String,
}

#[derive(sqlx::FromRow)]
struct BookingRoomRow {
    id: String,
    space_id: String,
    check_in: String,
    check_out: String,
    adults: i64,
    children: i64,
    board_option_id: Option<String>,
    price_override: Option<i64>,
}

impl TryFrom<BookingRoomRow> for BookingRoom {
    type Error = StoreError;
    fn try_from(r: BookingRoomRow) -> StoreResult<Self> {
        Ok(BookingRoom {
            id: BookingRoomId(uuid(&r.id)?),
            space_id: SpaceId(uuid(&r.space_id)?),
            stay: DateRange {
                start: date(&r.check_in)?,
                end: date(&r.check_out)?,
            },
            adults: int(r.adults, "booking_rooms.adults")?,
            children: int(r.children, "booking_rooms.children")?,
            board_option_id: r
                .board_option_id
                .as_deref()
                .map(uuid)
                .transpose()?
                .map(BoardOptionId),
            price_override: r.price_override.map(Money),
        })
    }
}

#[derive(sqlx::FromRow)]
struct NightRow {
    booking_room_id: String,
    night: String,
    room_price: i64,
    supplements: i64,
    board_price: i64,
    total: i64,
}

#[derive(sqlx::FromRow)]
struct ChartRow {
    booking_id: String,
    booking_room_id: String,
    reference: String,
    first_name: String,
    last_name: String,
    status: String,
    space_id: String,
    check_in: String,
    check_out: String,
    adults: i64,
    children: i64,
    board: Option<String>,
}

#[derive(sqlx::FromRow)]
struct ListRow {
    id: String,
    reference: String,
    first_name: String,
    last_name: String,
    status: String,
    arrival: String,
    departure: String,
    rooms: String,
    boards: String,
}

// ---------------------------------------------------------------------------
// Loaders (work on a pooled connection or inside a transaction)
// ---------------------------------------------------------------------------

async fn load_rate_rules(conn: &mut SqliteConnection) -> StoreResult<Vec<RateRule>> {
    let rows: Vec<RateRuleRow> = sqlx::query_as(
        "SELECT id, name, room_type_id, start_date, end_date, weekdays, priority,
                adjustment_kind, adjustment_value, active
         FROM rate_rules WHERE deleted_at IS NULL ORDER BY priority, start_date, name",
    )
    .fetch_all(conn)
    .await?;
    rows.into_iter().map(RateRule::try_from).collect()
}

async fn load_boards(conn: &mut SqliteConnection) -> StoreResult<Vec<BoardOption>> {
    let rows: Vec<BoardRow> = sqlx::query_as(
        "SELECT id, code, name, adult_price, child_price FROM board_options
         WHERE deleted_at IS NULL ORDER BY adult_price, name",
    )
    .fetch_all(conn)
    .await?;
    rows.into_iter().map(BoardOption::try_from).collect()
}

pub(crate) async fn load_guest(conn: &mut SqliteConnection, id: GuestId) -> StoreResult<Guest> {
    let row: Option<GuestRow> = sqlx::query_as(
        "SELECT id, first_name, last_name, email, phone, address, notes FROM guests WHERE id = ?",
    )
    .bind(id.to_string())
    .fetch_optional(conn)
    .await?;
    row.ok_or(StoreError::NotFound { what: "Guest" })?
        .try_into()
}

pub(crate) async fn load_booking(
    conn: &mut SqliteConnection,
    id: BookingId,
) -> StoreResult<Option<Booking>> {
    let row: Option<BookingRow> = sqlx::query_as(
        "SELECT id, reference, guest_id, status, source, notes FROM bookings
         WHERE id = ? AND deleted_at IS NULL",
    )
    .bind(id.to_string())
    .fetch_optional(&mut *conn)
    .await?;
    let Some(row) = row else { return Ok(None) };
    let rooms: Vec<BookingRoomRow> = sqlx::query_as(
        "SELECT id, space_id, check_in, check_out, adults, children, board_option_id, price_override
         FROM booking_rooms WHERE booking_id = ? ORDER BY sort_order",
    )
    .bind(id.to_string())
    .fetch_all(&mut *conn)
    .await?;
    Ok(Some(Booking {
        id: BookingId(uuid(&row.id)?),
        reference: row.reference,
        guest_id: GuestId(uuid(&row.guest_id)?),
        status: parse(&row.status)?,
        source: parse(&row.source)?,
        notes: row.notes,
        rooms: rooms
            .into_iter()
            .map(BookingRoom::try_from)
            .collect::<StoreResult<_>>()?,
    }))
}

/// Saved nightly prices for a booking, grouped by booked room.
async fn load_nights(
    conn: &mut SqliteConnection,
    booking: BookingId,
) -> StoreResult<HashMap<BookingRoomId, Vec<NightPrice>>> {
    let rows: Vec<NightRow> = sqlx::query_as(
        "SELECT n.booking_room_id, n.night, n.room_price, n.supplements, n.board_price, n.total
         FROM room_nights n JOIN booking_rooms r ON r.id = n.booking_room_id
         WHERE r.booking_id = ? ORDER BY n.night",
    )
    .bind(booking.to_string())
    .fetch_all(conn)
    .await?;
    let mut map: HashMap<BookingRoomId, Vec<NightPrice>> = HashMap::new();
    for r in rows {
        map.entry(BookingRoomId(uuid(&r.booking_room_id)?))
            .or_default()
            .push(NightPrice {
                night: date(&r.night)?,
                room: Money(r.room_price),
                supplements: Money(r.supplements),
                board: Money(r.board_price),
                total: Money(r.total),
                rules: Vec::new(),
            });
    }
    Ok(map)
}

/// Rooms held (not cancelled / no-show) by any booking, overlapping `[from, to)`.
pub(crate) async fn load_occupancy(
    conn: &mut SqliteConnection,
    from: NaiveDate,
    to: NaiveDate,
) -> StoreResult<Vec<Occupancy>> {
    let rows: Vec<(String, String, String, String)> = sqlx::query_as(
        "SELECT b.id, r.space_id, r.check_in, r.check_out
         FROM booking_rooms r JOIN bookings b ON b.id = r.booking_id
         WHERE b.deleted_at IS NULL AND b.status NOT IN ('cancelled', 'no_show')
           AND r.check_in < ? AND r.check_out > ?",
    )
    .bind(to.to_string())
    .bind(from.to_string())
    .fetch_all(conn)
    .await?;
    rows.into_iter()
        .map(|(b, s, ci, co)| {
            Ok(Occupancy {
                booking_id: BookingId(uuid(&b)?),
                space_id: SpaceId(uuid(&s)?),
                stay: DateRange {
                    start: date(&ci)?,
                    end: date(&co)?,
                },
            })
        })
        .collect()
}

/// Allocates the next reference for `year` (e.g. "2026-0007").
async fn next_reference(conn: &mut SqliteConnection, year: i32) -> StoreResult<String> {
    let n: i64 = sqlx::query_scalar(
        "INSERT INTO booking_counters (year, last_number) VALUES (?, 1)
         ON CONFLICT (year) DO UPDATE SET last_number = last_number + 1
         RETURNING last_number",
    )
    .bind(year)
    .fetch_one(conn)
    .await?;
    Ok(format_reference(
        year,
        int(n, "booking_counters.last_number")?,
    ))
}

pub(crate) async fn detail(
    conn: &mut SqliteConnection,
    id: BookingId,
) -> StoreResult<BookingDetail> {
    let booking = load_booking(conn, id)
        .await?
        .ok_or(StoreError::NotFound { what: "Booking" })?;
    let guest = load_guest(conn, booking.guest_id).await?;
    let mut nights = load_nights(conn, id).await?;
    let pricing: Vec<RoomPricing> = booking
        .rooms
        .iter()
        .map(|room| {
            let nights = nights.remove(&room.id).unwrap_or_default();
            let calculated: Money = nights.iter().map(|n| n.total).sum();
            RoomPricing {
                booking_room_id: room.id,
                calculated,
                total: room.price_override.unwrap_or(calculated),
                nights,
            }
        })
        .collect();
    let total = pricing.iter().map(|p| p.total).sum();
    Ok(BookingDetail {
        booking,
        guest,
        pricing,
        total,
    })
}

/// True if a booked room's stay details match its saved version, so its
/// nightly prices are kept rather than re-priced at today's rates.
fn same_stay(old: &BookingRoom, new: &BookingRoom) -> bool {
    old.id == new.id
        && old.space_id == new.space_id
        && old.stay == new.stay
        && old.adults == new.adults
        && old.children == new.children
        && old.board_option_id == new.board_option_id
}

/// Inserts or updates a guest (already validated) inside the caller's transaction.
pub(crate) async fn upsert_guest(tx: &mut SqliteConnection, guest: &Guest) -> StoreResult<()> {
    sqlx::query(
        "INSERT INTO guests (id, first_name, last_name, email, phone, address, notes)
         VALUES (?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT (id) DO UPDATE SET
            first_name = excluded.first_name, last_name = excluded.last_name,
            email = excluded.email, phone = excluded.phone, address = excluded.address,
            notes = excluded.notes, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')",
    )
    .bind(guest.id.to_string())
    .bind(guest.first_name.trim())
    .bind(guest.last_name.trim())
    .bind(guest.email.trim())
    .bind(guest.phone.trim())
    .bind(guest.address.trim())
    .bind(guest.notes.trim())
    .execute(&mut *tx)
    .await?;
    Ok(())
}

/// Checks, prices and writes a booking inside the caller's transaction.
pub(crate) async fn write_booking(
    tx: &mut SqliteConnection,
    booking: &Booking,
    guest: &Guest,
) -> StoreResult<()> {
    validate_guest(guest)?;
    let spaces = load_spaces(tx, None).await?;
    let room_types = load_room_types(tx).await?;
    let rules = load_rate_rules(tx).await?;
    let boards = load_boards(tx).await?;

    let from = booking.rooms.iter().map(|r| r.stay.start).min();
    let to = booking.rooms.iter().map(|r| r.stay.end).max();
    let occupancy = match (from, to) {
        (Some(from), Some(to)) => load_occupancy(tx, from, to).await?,
        _ => Vec::new(),
    };
    validate_booking(booking, &spaces, &room_types, &occupancy)?;
    for room in &booking.rooms {
        if let Some(b) = room.board_option_id
            && !boards.iter().any(|x| x.id == b)
        {
            return Err(StoreError::NotFound {
                what: "Board option",
            });
        }
    }

    let existing = load_booking(tx, booking.id).await?;
    let mut saved_nights = load_nights(tx, booking.id).await?;

    upsert_guest(tx, guest).await?;

    let reference = match &existing {
        Some(b) => b.reference.clone(),
        None => next_reference(tx, chrono::Local::now().year()).await?,
    };
    sqlx::query(
        "INSERT INTO bookings (id, reference, guest_id, status, source, notes)
         VALUES (?, ?, ?, ?, ?, ?)
         ON CONFLICT (id) DO UPDATE SET
            guest_id = excluded.guest_id, status = excluded.status, source = excluded.source,
            notes = excluded.notes, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')",
    )
    .bind(booking.id.to_string())
    .bind(&reference)
    .bind(guest.id.to_string())
    .bind(booking.status.as_str())
    .bind(booking.source.as_str())
    .bind(booking.notes.trim())
    .execute(&mut *tx)
    .await?;

    // Replace the booked rooms; ON DELETE CASCADE removes their nights too.
    sqlx::query("DELETE FROM booking_rooms WHERE booking_id = ?")
        .bind(booking.id.to_string())
        .execute(&mut *tx)
        .await?;

    for (order, room) in booking.rooms.iter().enumerate() {
        sqlx::query(
            "INSERT INTO booking_rooms (id, booking_id, space_id, check_in, check_out, adults,
                                        children, board_option_id, price_override, sort_order)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(room.id.to_string())
        .bind(booking.id.to_string())
        .bind(room.space_id.to_string())
        .bind(room.stay.start.to_string())
        .bind(room.stay.end.to_string())
        .bind(i64::from(room.adults))
        .bind(i64::from(room.children))
        .bind(room.board_option_id.map(|b| b.to_string()))
        .bind(room.price_override.map(|m| m.0))
        .bind(i64::try_from(order).unwrap_or(i64::MAX))
        .execute(&mut *tx)
        .await?;

        if !booking.status.holds_rooms() {
            continue;
        }
        let unchanged = existing
            .as_ref()
            .is_some_and(|b| b.rooms.iter().any(|old| same_stay(old, room)));
        let nights = match saved_nights.remove(&room.id) {
            Some(nights) if unchanged => nights,
            _ => {
                let room_type = spaces
                    .iter()
                    .find(|s| s.id == room.space_id)
                    .and_then(|s| s.room_type_id)
                    .and_then(|id| room_types.iter().find(|t| t.id == id))
                    .ok_or(StoreError::NotFound { what: "Room type" })?;
                let board = room
                    .board_option_id
                    .and_then(|b| boards.iter().find(|x| x.id == b));
                quote_stay(
                    room_type,
                    &rules,
                    board,
                    room.stay,
                    room.adults,
                    room.children,
                )
                .nights
            }
        };
        for n in nights {
            let inserted = sqlx::query(
                "INSERT INTO room_nights (booking_room_id, space_id, night, room_price,
                                          supplements, board_price, total)
                 VALUES (?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(room.id.to_string())
            .bind(room.space_id.to_string())
            .bind(n.night.to_string())
            .bind(n.room.0)
            .bind(n.supplements.0)
            .bind(n.board.0)
            .bind(n.total.0)
            .execute(&mut *tx)
            .await;
            // The unique (space, night) index is the last line of defence.
            if let Err(sqlx::Error::Database(e)) = &inserted
                && e.is_unique_violation()
            {
                let name = spaces
                    .iter()
                    .find(|s| s.id == room.space_id)
                    .map(|s| s.name.clone());
                return Err(ValidationError::RoomUnavailable {
                    room: name.unwrap_or_default(),
                    night: n.night.format("%-d %b %Y").to_string(),
                }
                .into());
            }
            inserted?;
        }
    }
    Ok(())
}

/// Runs a soft-delete `UPDATE … WHERE id = ?` and logs it, reporting
/// NotFound if nothing matched. `sql: &'static str` keeps the query text fixed
/// at compile time, as sqlx requires.
async fn soft_delete(
    store: &SqliteStore,
    sql: &'static str,
    id: String,
    what: &'static str,
    entity: &'static str,
    before: Option<String>,
    summary: String,
) -> StoreResult<()> {
    let mut tx = store.pool.begin().await?;
    let done = sqlx::query(sql).bind(&id).execute(&mut *tx).await?;
    if done.rows_affected() == 0 {
        return Err(StoreError::NotFound { what });
    }
    record(
        &mut tx,
        Change {
            entity,
            entity_id: id,
            action: "deleted",
            summary,
            before,
            after: None,
            actor: store.actor.as_ref(),
        },
    )
    .await?;
    tx.commit().await?;
    Ok(())
}

/// "Added" for a new record, "Changed" for an existing one.
pub(crate) fn verb(existed: bool) -> (&'static str, &'static str) {
    if existed {
        ("updated", "Changed")
    } else {
        ("created", "Added")
    }
}

// ---------------------------------------------------------------------------
// Repository implementations
// ---------------------------------------------------------------------------

impl RateRepo for SqliteStore {
    async fn list_rate_rules(&self) -> StoreResult<Vec<RateRule>> {
        let mut conn = self.pool.acquire().await?;
        load_rate_rules(&mut conn).await
    }

    async fn save_rate_rule(&self, rule: &RateRule) -> StoreResult<()> {
        validate_rate_rule(rule)?;
        let (kind, value) = adjustment_columns(rule.adjustment);
        let mut weekdays = rule.weekdays.clone();
        weekdays.sort_unstable();
        weekdays.dedup();
        let weekdays = weekdays
            .iter()
            .map(u8::to_string)
            .collect::<Vec<_>>()
            .join(",");
        let mut tx = self.pool.begin().await?;
        let before = load_rate_rules(&mut tx)
            .await?
            .into_iter()
            .find(|r| r.id == rule.id);
        sqlx::query(
            "INSERT INTO rate_rules (id, name, room_type_id, start_date, end_date, weekdays,
                                     priority, adjustment_kind, adjustment_value, active)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT (id) DO UPDATE SET
                name = excluded.name, room_type_id = excluded.room_type_id,
                start_date = excluded.start_date, end_date = excluded.end_date,
                weekdays = excluded.weekdays, priority = excluded.priority,
                adjustment_kind = excluded.adjustment_kind,
                adjustment_value = excluded.adjustment_value, active = excluded.active,
                updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')",
        )
        .bind(rule.id.to_string())
        .bind(rule.name.trim())
        .bind(rule.room_type_id.map(|id| id.to_string()))
        .bind(rule.start.to_string())
        .bind(rule.end.to_string())
        .bind(weekdays)
        .bind(rule.priority)
        .bind(kind)
        .bind(value)
        .bind(rule.active)
        .execute(&mut *tx)
        .await?;
        let (action, word) = verb(before.is_some());
        record(
            &mut tx,
            Change {
                entity: "rate_rule",
                entity_id: rule.id.to_string(),
                action,
                summary: format!("{word} rate rule {}", rule.name.trim()),
                before: before.as_ref().and_then(json),
                after: json(rule),
                actor: self.actor.as_ref(),
            },
        )
        .await?;
        tx.commit().await?;
        Ok(())
    }

    async fn delete_rate_rule(&self, id: RateRuleId) -> StoreResult<()> {
        let rule = self
            .list_rate_rules()
            .await?
            .into_iter()
            .find(|r| r.id == id);
        let name = rule.as_ref().map(|r| r.name.clone()).unwrap_or_default();
        soft_delete(
            self,
            "UPDATE rate_rules SET deleted_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE id = ? AND deleted_at IS NULL",
            id.to_string(),
            "Rate rule",
            "rate_rule",
            rule.as_ref().and_then(json),
            format!("Deleted rate rule {name}"),
        )
        .await
    }

    async fn list_board_options(&self) -> StoreResult<Vec<BoardOption>> {
        let mut conn = self.pool.acquire().await?;
        load_boards(&mut conn).await
    }

    async fn save_board_option(&self, board: &BoardOption) -> StoreResult<()> {
        let mut tx = self.pool.begin().await?;
        let others = load_boards(&mut tx).await?;
        validate_board_option(board, &others)?;
        let before = others.iter().find(|b| b.id == board.id).cloned();
        sqlx::query(
            "INSERT INTO board_options (id, code, name, adult_price, child_price)
             VALUES (?, ?, ?, ?, ?)
             ON CONFLICT (id) DO UPDATE SET
                code = excluded.code, name = excluded.name,
                adult_price = excluded.adult_price, child_price = excluded.child_price,
                updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')",
        )
        .bind(board.id.to_string())
        .bind(board.code.trim().to_uppercase())
        .bind(board.name.trim())
        .bind(board.adult_price.0)
        .bind(board.child_price.0)
        .execute(&mut *tx)
        .await?;
        let (action, word) = verb(before.is_some());
        record(
            &mut tx,
            Change {
                entity: "board_option",
                entity_id: board.id.to_string(),
                action,
                summary: format!("{word} board option {}", board.name.trim()),
                before: before.as_ref().and_then(json),
                after: json(board),
                actor: self.actor.as_ref(),
            },
        )
        .await?;
        tx.commit().await?;
        Ok(())
    }

    async fn delete_board_option(&self, id: BoardOptionId) -> StoreResult<()> {
        let board = self
            .list_board_options()
            .await?
            .into_iter()
            .find(|b| b.id == id);
        let name = board.as_ref().map(|b| b.name.clone()).unwrap_or_default();
        soft_delete(
            self,
            "UPDATE board_options SET deleted_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE id = ? AND deleted_at IS NULL",
            id.to_string(),
            "Board option",
            "board_option",
            board.as_ref().and_then(json),
            format!("Deleted board option {name}"),
        )
        .await
    }
}

impl GuestRepo for SqliteStore {
    async fn search_guests(&self, query: &str) -> StoreResult<Vec<Guest>> {
        let like = format!("%{}%", query.trim().to_lowercase());
        let rows: Vec<GuestRow> = sqlx::query_as(
            "SELECT id, first_name, last_name, email, phone, address, notes FROM guests
             WHERE deleted_at IS NULL AND (
                lower(first_name || ' ' || last_name) LIKE ?1
                OR lower(email) LIKE ?1 OR phone LIKE ?1)
             ORDER BY lower(last_name), lower(first_name) LIMIT 20",
        )
        .bind(like)
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter().map(Guest::try_from).collect()
    }
}

impl BookingRepo for SqliteStore {
    async fn quote_room(
        &self,
        space_id: SpaceId,
        stay: DateRange,
        adults: u8,
        children: u8,
        board: Option<BoardOptionId>,
    ) -> StoreResult<StayQuote> {
        stay.validate()?;
        let mut conn = self.pool.acquire().await?;
        let spaces = load_spaces(&mut conn, None).await?;
        let room_types = load_room_types(&mut conn).await?;
        let room_type = spaces
            .iter()
            .find(|s| s.id == space_id)
            .and_then(|s| s.room_type_id)
            .and_then(|id| room_types.iter().find(|t| t.id == id))
            .ok_or(StoreError::NotFound { what: "Room" })?;
        let rules = load_rate_rules(&mut conn).await?;
        let boards = load_boards(&mut conn).await?;
        let board = board.and_then(|b| boards.iter().find(|x| x.id == b));
        Ok(quote_stay(room_type, &rules, board, stay, adults, children))
    }

    async fn free_rooms(
        &self,
        stay: DateRange,
        ignore: Option<BookingId>,
    ) -> StoreResult<Vec<SpaceId>> {
        stay.validate()?;
        let mut conn = self.pool.acquire().await?;
        let spaces = load_spaces(&mut conn, None).await?;
        let occupancy = load_occupancy(&mut conn, stay.start, stay.end).await?;
        Ok(free_rooms(&stay, &spaces, &occupancy, ignore)
            .into_iter()
            .map(|s| s.id)
            .collect())
    }

    async fn chart(&self, from: NaiveDate, to: NaiveDate) -> StoreResult<Vec<ChartEntry>> {
        let rows: Vec<ChartRow> = sqlx::query_as(
            "SELECT b.id AS booking_id, r.id AS booking_room_id, b.reference, g.first_name,
                    g.last_name, b.status, r.space_id, r.check_in, r.check_out, r.adults,
                    r.children, bo.name AS board
             FROM booking_rooms r
             JOIN bookings b ON b.id = r.booking_id
             JOIN guests g ON g.id = b.guest_id
             LEFT JOIN board_options bo ON bo.id = r.board_option_id
             WHERE b.deleted_at IS NULL AND b.status NOT IN ('cancelled', 'no_show')
               AND r.check_in < ? AND r.check_out > ?
             ORDER BY r.check_in",
        )
        .bind(to.to_string())
        .bind(from.to_string())
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter()
            .map(|r| {
                Ok(ChartEntry {
                    booking_id: BookingId(uuid(&r.booking_id)?),
                    booking_room_id: BookingRoomId(uuid(&r.booking_room_id)?),
                    reference: r.reference,
                    guest_name: full_name(&r.first_name, &r.last_name),
                    status: parse(&r.status)?,
                    space_id: SpaceId(uuid(&r.space_id)?),
                    stay: DateRange {
                        start: date(&r.check_in)?,
                        end: date(&r.check_out)?,
                    },
                    adults: int(r.adults, "adults")?,
                    children: int(r.children, "children")?,
                    board: r.board,
                })
            })
            .collect()
    }

    async fn get_booking(&self, id: BookingId) -> StoreResult<BookingDetail> {
        let mut conn = self.pool.acquire().await?;
        detail(&mut conn, id).await
    }

    async fn save_booking(&self, booking: &Booking, guest: &Guest) -> StoreResult<BookingDetail> {
        let mut tx = self.pool.begin().await?;
        let before = match detail(&mut tx, booking.id).await {
            Ok(d) => Some(d),
            Err(StoreError::NotFound { .. }) => None,
            Err(e) => return Err(e),
        };
        write_booking(&mut tx, booking, guest).await?;
        let saved = detail(&mut tx, booking.id).await?;
        let (action, _) = verb(before.is_some());
        let what = if before.is_some() { "Changed" } else { "Made" };
        record(
            &mut tx,
            Change {
                entity: "booking",
                entity_id: booking.id.to_string(),
                action,
                summary: format!(
                    "{what} booking {} for {}",
                    saved.booking.reference,
                    guest.display_name()
                ),
                before: before.as_ref().and_then(json),
                after: json(&saved),
                actor: self.actor.as_ref(),
            },
        )
        .await?;
        tx.commit().await?;
        Ok(saved)
    }

    async fn set_booking_status(
        &self,
        id: BookingId,
        status: BookingStatus,
    ) -> StoreResult<BookingDetail> {
        let mut tx = self.pool.begin().await?;
        let mut booking = load_booking(&mut tx, id)
            .await?
            .ok_or(StoreError::NotFound { what: "Booking" })?;
        if !booking.status.can_change_to(status) {
            return Err(ValidationError::InvalidStatusChange {
                from: booking.status.as_str().replace('_', " "),
                to: status.as_str().replace('_', " "),
            }
            .into());
        }
        let guest = load_guest(&mut tx, booking.guest_id).await?;
        let before = detail(&mut tx, id).await?;
        booking.status = status;
        write_booking(&mut tx, &booking, &guest).await?;
        let saved = detail(&mut tx, id).await?;
        let what = match status {
            BookingStatus::Provisional => "Marked provisional",
            BookingStatus::Confirmed => "Confirmed",
            BookingStatus::CheckedIn => "Checked in",
            BookingStatus::CheckedOut => "Checked out",
            BookingStatus::Cancelled => "Cancelled",
            BookingStatus::NoShow => "Marked no-show",
        };
        record(
            &mut tx,
            Change {
                entity: "booking",
                entity_id: id.to_string(),
                action: status.as_str(),
                summary: format!(
                    "{what} booking {} for {}",
                    saved.booking.reference,
                    guest.display_name()
                ),
                before: json(&before),
                after: json(&saved),
                actor: self.actor.as_ref(),
            },
        )
        .await?;
        tx.commit().await?;
        Ok(saved)
    }

    async fn search_bookings(&self, query: &str) -> StoreResult<Vec<BookingListItem>> {
        let q = query.trim().to_lowercase();
        let today = chrono::Local::now().date_naive().to_string();
        let rows: Vec<ListRow> = sqlx::query_as(
            "SELECT b.id, b.reference, g.first_name, g.last_name, b.status,
                    min(r.check_in) AS arrival, max(r.check_out) AS departure,
                    group_concat(s.name, ', ') AS rooms,
                    group_concat(coalesce(bo.name, ''), char(31)) AS boards
             FROM bookings b
             JOIN guests g ON g.id = b.guest_id
             JOIN booking_rooms r ON r.booking_id = b.id
             JOIN spaces s ON s.id = r.space_id
             LEFT JOIN board_options bo ON bo.id = r.board_option_id
             WHERE b.deleted_at IS NULL AND (
                (?1 = '' AND r.check_out >= ?2)
                OR (?1 <> '' AND (lower(b.reference) LIKE '%' || ?1 || '%'
                    OR lower(g.first_name || ' ' || g.last_name) LIKE '%' || ?1 || '%'
                    OR lower(g.email) LIKE '%' || ?1 || '%')))
             GROUP BY b.id
             ORDER BY min(r.check_in) LIMIT 50",
        )
        .bind(&q)
        .bind(today)
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter()
            .map(|r| {
                Ok(BookingListItem {
                    id: BookingId(uuid(&r.id)?),
                    reference: r.reference,
                    guest_name: full_name(&r.first_name, &r.last_name),
                    status: parse(&r.status)?,
                    arrival: date(&r.arrival)?,
                    departure: date(&r.departure)?,
                    rooms: r.rooms.split(", ").map(str::to_string).collect(),
                    // Unit separator between names; an empty name means room only.
                    boards: r
                        .boards
                        .split('\u{1f}')
                        .map(|b| (!b.is_empty()).then(|| b.to_string()))
                        .collect(),
                })
            })
            .collect()
    }
}
