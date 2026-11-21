//! SQLite implementation of function-suite hire and events.
//!
//! Suite bookings follow the same pattern as bedroom bookings: validate in
//! `booker-core`, fix the price at save time, write and audit in one
//! transaction. An event's room block is made of ordinary bedroom bookings
//! linked by `bookings.event_id`; [`sync_block`] keeps them in line with the
//! block using the plan worked out by `booker_core::plan_room_block`.

use booker_core::{
    Booking, BookingId, BookingRoom, BookingRoomId, BookingSource, BookingStatus, CateringSitting,
    DateRange, Event, EventDetail, EventId, EventListItem, EventRoom, Guest, GuestId, LinkedRoom,
    Money, RoomBlock, RoomTypeId, SpaceId, SuiteBooking, SuiteBookingEntry, SuiteBookingId,
    SuiteBookingStatus, SuitePricing, SuiteRates, block_room_price, pick_block_rooms,
    plan_room_block, price_suite_booking, validate_event, validate_guest, validate_suite_booking,
    validate_suite_rates,
};
use chrono::{Duration, NaiveDate, NaiveDateTime, NaiveTime};
use sqlx::SqliteConnection;

use crate::audit::{Change, json, record};
use crate::bookings::{
    date, detail, full_name, load_booking, load_guest, load_occupancy, parse, upsert_guest, verb,
    write_booking,
};
use crate::repo::{EventRepo, SuiteRepo};
use crate::rows::{int, uuid};
use crate::sqlite::{load_room_types, load_spaces};
use crate::{SqliteStore, StoreError, StoreResult};

/// How date-times are stored: ISO text that sorts in time order.
const STAMP: &str = "%Y-%m-%d %H:%M:%S";

fn stamp(t: NaiveDateTime) -> String {
    t.format(STAMP).to_string()
}

fn parse_stamp(s: &str) -> StoreResult<NaiveDateTime> {
    NaiveDateTime::parse_from_str(s, STAMP)
        .map_err(|e| StoreError::Corrupt(format!("bad date-time {s:?}: {e}")))
}

fn parse_time(s: &str) -> StoreResult<NaiveTime> {
    NaiveTime::parse_from_str(s, "%H:%M")
        .map_err(|e| StoreError::Corrupt(format!("bad time {s:?}: {e}")))
}

/// The two columns a [`SuitePricing`] is stored in.
fn pricing_columns(p: SuitePricing) -> (&'static str, Option<i64>) {
    match p {
        SuitePricing::Hourly => ("hourly", None),
        SuitePricing::HalfDay => ("half_day", None),
        SuitePricing::FullDay => ("full_day", None),
        SuitePricing::PerHead { price } => ("per_head", Some(price.0)),
        SuitePricing::Manual { price } => ("manual", Some(price.0)),
    }
}

fn pricing_from(kind: &str, amount: Option<i64>) -> StoreResult<SuitePricing> {
    Ok(match (kind, amount) {
        ("hourly", _) => SuitePricing::Hourly,
        ("half_day", _) => SuitePricing::HalfDay,
        ("full_day", _) => SuitePricing::FullDay,
        ("per_head", Some(a)) => SuitePricing::PerHead { price: Money(a) },
        ("manual", Some(a)) => SuitePricing::Manual { price: Money(a) },
        _ => {
            return Err(StoreError::Corrupt(format!(
                "bad suite pricing {kind:?} / {amount:?}"
            )));
        }
    })
}

/// Converts every row, stopping at the first that can't be read.
fn convert<R, T: TryFrom<R, Error = StoreError>>(rows: Vec<R>) -> StoreResult<Vec<T>> {
    rows.into_iter().map(T::try_from).collect()
}

// ---------------------------------------------------------------------------
// Suite rate cards (columns on `spaces`)
// ---------------------------------------------------------------------------

#[derive(sqlx::FromRow)]
struct RatesRow {
    name: String,
    rate_hourly: Option<i64>,
    rate_min_hours: i64,
    rate_half_day: Option<i64>,
    rate_full_day: Option<i64>,
    setup_minutes: i64,
    cleardown_minutes: i64,
}

/// A suite's name and rate card.
async fn load_rates(
    conn: &mut SqliteConnection,
    suite: SpaceId,
) -> StoreResult<(String, SuiteRates)> {
    let row: Option<RatesRow> = sqlx::query_as(
        "SELECT name, rate_hourly, rate_min_hours, rate_half_day, rate_full_day, setup_minutes,
                cleardown_minutes
         FROM spaces WHERE id = ? AND kind = 'suite' AND deleted_at IS NULL",
    )
    .bind(suite.to_string())
    .fetch_optional(conn)
    .await?;
    let r = row.ok_or(StoreError::NotFound { what: "Suite" })?;
    let rates = SuiteRates {
        hourly: r.rate_hourly.map(Money),
        min_hours: int(r.rate_min_hours, "spaces.rate_min_hours")?,
        half_day: r.rate_half_day.map(Money),
        full_day: r.rate_full_day.map(Money),
        setup_minutes: int(r.setup_minutes, "spaces.setup_minutes")?,
        cleardown_minutes: int(r.cleardown_minutes, "spaces.cleardown_minutes")?,
    };
    Ok((r.name, rates))
}

// ---------------------------------------------------------------------------
// Suite bookings
// ---------------------------------------------------------------------------

/// The SELECT shared by suite booking queries (see `select_spaces!`).
macro_rules! select_suite_bookings {
    () => {
        "SELECT sb.id, sb.suite_id, sb.event_id, sb.guest_id, sb.title, sb.start_at, sb.end_at,
                sb.layout, sb.attendees, sb.setup_minutes, sb.cleardown_minutes,
                sb.pricing_kind, sb.pricing_amount, sb.price, sb.status, sb.notes,
                s.name AS suite_name, g.first_name, g.last_name, e.name AS event_name
         FROM suite_bookings sb
         JOIN spaces s ON s.id = sb.suite_id
         JOIN guests g ON g.id = sb.guest_id
         LEFT JOIN events e ON e.id = sb.event_id
         WHERE sb.deleted_at IS NULL"
    };
}

#[derive(sqlx::FromRow)]
struct SuiteBookingRow {
    id: String,
    suite_id: String,
    event_id: Option<String>,
    guest_id: String,
    title: String,
    start_at: String,
    end_at: String,
    layout: String,
    attendees: i64,
    setup_minutes: i64,
    cleardown_minutes: i64,
    pricing_kind: String,
    pricing_amount: Option<i64>,
    price: i64,
    status: String,
    notes: String,
    suite_name: String,
    first_name: String,
    last_name: String,
    event_name: Option<String>,
}

impl TryFrom<SuiteBookingRow> for SuiteBookingEntry {
    type Error = StoreError;
    fn try_from(r: SuiteBookingRow) -> StoreResult<Self> {
        Ok(SuiteBookingEntry {
            booking: SuiteBooking {
                id: SuiteBookingId(uuid(&r.id)?),
                suite_id: SpaceId(uuid(&r.suite_id)?),
                event_id: r.event_id.as_deref().map(uuid).transpose()?.map(EventId),
                guest_id: GuestId(uuid(&r.guest_id)?),
                title: r.title,
                start: parse_stamp(&r.start_at)?,
                end: parse_stamp(&r.end_at)?,
                layout: parse(&r.layout)?,
                attendees: int(r.attendees, "suite_bookings.attendees")?,
                setup_minutes: int(r.setup_minutes, "suite_bookings.setup_minutes")?,
                cleardown_minutes: int(r.cleardown_minutes, "suite_bookings.cleardown_minutes")?,
                pricing: pricing_from(&r.pricing_kind, r.pricing_amount)?,
                status: parse(&r.status)?,
                notes: r.notes,
            },
            price: Money(r.price),
            suite_name: r.suite_name,
            guest_name: full_name(&r.first_name, &r.last_name),
            event_name: r.event_name,
        })
    }
}

async fn load_suite_booking(
    conn: &mut SqliteConnection,
    id: SuiteBookingId,
) -> StoreResult<Option<SuiteBookingEntry>> {
    let row: Option<SuiteBookingRow> =
        sqlx::query_as(concat!(select_suite_bookings!(), " AND sb.id = ?"))
            .bind(id.to_string())
            .fetch_optional(conn)
            .await?;
    row.map(SuiteBookingEntry::try_from).transpose()
}

async fn require_suite_booking(
    conn: &mut SqliteConnection,
    id: SuiteBookingId,
) -> StoreResult<SuiteBookingEntry> {
    load_suite_booking(conn, id)
        .await?
        .ok_or(StoreError::NotFound {
            what: "Suite booking",
        })
}

async fn event_suite_bookings(
    conn: &mut SqliteConnection,
    event: EventId,
) -> StoreResult<Vec<SuiteBookingEntry>> {
    let rows: Vec<SuiteBookingRow> = sqlx::query_as(concat!(
        select_suite_bookings!(),
        " AND sb.event_id = ? ORDER BY sb.start_at"
    ))
    .bind(event.to_string())
    .fetch_all(conn)
    .await?;
    convert(rows)
}

/// True when the saved price still applies: nothing that affects it changed.
fn same_terms(old: &SuiteBooking, new: &SuiteBooking) -> bool {
    old.suite_id == new.suite_id
        && old.start == new.start
        && old.end == new.end
        && old.attendees == new.attendees
        && old.pricing == new.pricing
}

/// Checks, prices and writes a suite booking inside the caller's transaction.
async fn write_suite_booking(
    tx: &mut SqliteConnection,
    booking: &SuiteBooking,
    guest: &Guest,
) -> StoreResult<()> {
    validate_guest(guest)?;
    let spaces = load_spaces(tx, None).await?;
    let suite = spaces
        .iter()
        .find(|s| s.id == booking.suite_id)
        .ok_or(StoreError::NotFound { what: "Suite" })?;

    // Buffers are under a day each, so only bookings within a day or so
    // either side can clash.
    let rows: Vec<SuiteBookingRow> = sqlx::query_as(concat!(
        select_suite_bookings!(),
        " AND sb.suite_id = ? AND sb.status <> 'cancelled' AND sb.start_at < ? AND sb.end_at > ?"
    ))
    .bind(booking.suite_id.to_string())
    .bind(stamp(booking.end + Duration::days(2)))
    .bind(stamp(booking.start - Duration::days(2)))
    .fetch_all(&mut *tx)
    .await?;
    let others: Vec<SuiteBooking> = convert::<_, SuiteBookingEntry>(rows)?
        .into_iter()
        .map(|e| e.booking)
        .collect();
    validate_suite_booking(booking, suite, &others)?;

    if let Some(event) = booking.event_id {
        let found: Option<String> =
            sqlx::query_scalar("SELECT id FROM events WHERE id = ? AND deleted_at IS NULL")
                .bind(event.to_string())
                .fetch_optional(&mut *tx)
                .await?;
        if found.is_none() {
            return Err(StoreError::NotFound { what: "Event" });
        }
    }

    let existing = load_suite_booking(tx, booking.id).await?;
    let price = match existing {
        Some(old) if same_terms(&old.booking, booking) => old.price,
        _ => {
            let (_, rates) = load_rates(tx, booking.suite_id).await?;
            price_suite_booking(booking, &rates)?
        }
    };

    upsert_guest(tx, guest).await?;
    let (pricing_kind, pricing_amount) = pricing_columns(booking.pricing);
    sqlx::query(
        "INSERT INTO suite_bookings (id, suite_id, event_id, guest_id, title, start_at, end_at,
                                     layout, attendees, setup_minutes, cleardown_minutes,
                                     pricing_kind, pricing_amount, price, status, notes)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT (id) DO UPDATE SET
            suite_id = excluded.suite_id, event_id = excluded.event_id,
            guest_id = excluded.guest_id, title = excluded.title, start_at = excluded.start_at,
            end_at = excluded.end_at, layout = excluded.layout, attendees = excluded.attendees,
            setup_minutes = excluded.setup_minutes, cleardown_minutes = excluded.cleardown_minutes,
            pricing_kind = excluded.pricing_kind, pricing_amount = excluded.pricing_amount,
            price = excluded.price, status = excluded.status, notes = excluded.notes,
            updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')",
    )
    .bind(booking.id.to_string())
    .bind(booking.suite_id.to_string())
    .bind(booking.event_id.map(|e| e.to_string()))
    .bind(guest.id.to_string())
    .bind(booking.title.trim())
    .bind(stamp(booking.start))
    .bind(stamp(booking.end))
    .bind(booking.layout.as_str())
    .bind(i64::from(booking.attendees))
    .bind(i64::from(booking.setup_minutes))
    .bind(i64::from(booking.cleardown_minutes))
    .bind(pricing_kind)
    .bind(pricing_amount)
    .bind(price.0)
    .bind(booking.status.as_str())
    .bind(booking.notes.trim())
    .execute(&mut *tx)
    .await?;
    Ok(())
}

impl SuiteRepo for SqliteStore {
    async fn get_suite_rates(&self, suite: SpaceId) -> StoreResult<SuiteRates> {
        let mut conn = self.pool.acquire().await?;
        Ok(load_rates(&mut conn, suite).await?.1)
    }

    async fn save_suite_rates(&self, suite: SpaceId, rates: &SuiteRates) -> StoreResult<()> {
        validate_suite_rates(rates)?;
        let mut tx = self.pool.begin().await?;
        let (name, before) = load_rates(&mut tx, suite).await?;
        sqlx::query(
            "UPDATE spaces SET rate_hourly = ?, rate_min_hours = ?, rate_half_day = ?,
                    rate_full_day = ?, setup_minutes = ?, cleardown_minutes = ?,
                    updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE id = ?",
        )
        .bind(rates.hourly.map(|m| m.0))
        .bind(i64::from(rates.min_hours))
        .bind(rates.half_day.map(|m| m.0))
        .bind(rates.full_day.map(|m| m.0))
        .bind(i64::from(rates.setup_minutes))
        .bind(i64::from(rates.cleardown_minutes))
        .bind(suite.to_string())
        .execute(&mut *tx)
        .await?;
        record(
            &mut tx,
            Change {
                entity: "space",
                entity_id: suite.to_string(),
                action: "updated",
                summary: format!("Changed the rates for {name}"),
                before: json(&before),
                after: json(rates),
                actor: self.actor.as_ref(),
            },
        )
        .await?;
        tx.commit().await?;
        Ok(())
    }

    async fn list_suite_bookings(
        &self,
        from: NaiveDateTime,
        to: NaiveDateTime,
    ) -> StoreResult<Vec<SuiteBookingEntry>> {
        let rows: Vec<SuiteBookingRow> = sqlx::query_as(concat!(
            select_suite_bookings!(),
            " AND sb.status <> 'cancelled' AND sb.start_at < ? AND sb.end_at > ?
             ORDER BY sb.start_at"
        ))
        .bind(stamp(to))
        .bind(stamp(from))
        .fetch_all(&self.pool)
        .await?;
        convert(rows)
    }

    async fn get_suite_booking(&self, id: SuiteBookingId) -> StoreResult<SuiteBookingEntry> {
        let mut conn = self.pool.acquire().await?;
        require_suite_booking(&mut conn, id).await
    }

    async fn save_suite_booking(
        &self,
        booking: &SuiteBooking,
        guest: &Guest,
    ) -> StoreResult<SuiteBookingEntry> {
        let mut tx = self.pool.begin().await?;
        let before = load_suite_booking(&mut tx, booking.id).await?;
        write_suite_booking(&mut tx, booking, guest).await?;
        let saved = require_suite_booking(&mut tx, booking.id).await?;
        let (action, _) = verb(before.is_some());
        let what = if before.is_some() {
            "Changed the booking of"
        } else {
            "Booked"
        };
        record(
            &mut tx,
            Change {
                entity: "suite_booking",
                entity_id: booking.id.to_string(),
                action,
                summary: format!(
                    "{what} {} for {} ({})",
                    saved.suite_name,
                    saved.booking.title,
                    saved.booking.start.format("%-d %b %Y %H:%M")
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

    async fn set_suite_booking_status(
        &self,
        id: SuiteBookingId,
        status: SuiteBookingStatus,
    ) -> StoreResult<SuiteBookingEntry> {
        let mut tx = self.pool.begin().await?;
        let before = require_suite_booking(&mut tx, id).await?;
        let guest = load_guest(&mut tx, before.booking.guest_id).await?;
        let booking = SuiteBooking {
            status,
            ..before.booking.clone()
        };
        // Saving again re-checks the suite is free when reinstating a cancellation.
        write_suite_booking(&mut tx, &booking, &guest).await?;
        let saved = require_suite_booking(&mut tx, id).await?;
        let what = match status {
            SuiteBookingStatus::Provisional => "Marked provisional",
            SuiteBookingStatus::Confirmed => "Confirmed",
            SuiteBookingStatus::Cancelled => "Cancelled",
        };
        record(
            &mut tx,
            Change {
                entity: "suite_booking",
                entity_id: id.to_string(),
                action: status.as_str(),
                summary: format!("{what} {} for {}", saved.suite_name, saved.booking.title),
                before: json(&before),
                after: json(&saved),
                actor: self.actor.as_ref(),
            },
        )
        .await?;
        tx.commit().await?;
        Ok(saved)
    }
}

// ---------------------------------------------------------------------------
// Events
// ---------------------------------------------------------------------------

#[derive(sqlx::FromRow)]
struct EventRow {
    id: String,
    name: String,
    kind: String,
    organiser_id: String,
    start_date: String,
    end_date: String,
    notes: String,
    running_order: String,
    block_room_type_id: Option<String>,
    block_rooms: Option<i64>,
    block_check_in: Option<String>,
    block_check_out: Option<String>,
    block_rate: Option<i64>,
    block_release_date: Option<String>,
}

impl EventRow {
    /// The block lives in nullable columns; it exists when they're all set.
    fn block(&self) -> StoreResult<Option<RoomBlock>> {
        let (Some(rooms), Some(ci), Some(co), Some(rate), Some(release)) = (
            self.block_rooms,
            &self.block_check_in,
            &self.block_check_out,
            self.block_rate,
            &self.block_release_date,
        ) else {
            return Ok(None);
        };
        Ok(Some(RoomBlock {
            room_type_id: self
                .block_room_type_id
                .as_deref()
                .map(uuid)
                .transpose()?
                .map(RoomTypeId),
            rooms: int(rooms, "events.block_rooms")?,
            stay: DateRange {
                start: date(ci)?,
                end: date(co)?,
            },
            rate: Money(rate),
            release_date: date(release)?,
        }))
    }
}

#[derive(sqlx::FromRow)]
struct SittingRow {
    name: String,
    date: String,
    time: String,
    adults: i64,
    children: i64,
    dietary: String,
}

async fn load_event(conn: &mut SqliteConnection, id: EventId) -> StoreResult<Option<Event>> {
    let row: Option<EventRow> = sqlx::query_as(
        "SELECT id, name, kind, organiser_id, start_date, end_date, notes, running_order,
                block_room_type_id, block_rooms, block_check_in, block_check_out, block_rate,
                block_release_date
         FROM events WHERE deleted_at IS NULL AND id = ?",
    )
    .bind(id.to_string())
    .fetch_optional(&mut *conn)
    .await?;
    let Some(row) = row else { return Ok(None) };
    let sittings: Vec<SittingRow> = sqlx::query_as(
        "SELECT name, date, time, adults, children, dietary FROM event_sittings
         WHERE event_id = ? ORDER BY sort_order",
    )
    .bind(id.to_string())
    .fetch_all(&mut *conn)
    .await?;
    Ok(Some(Event {
        room_block: row.block()?,
        id: EventId(uuid(&row.id)?),
        name: row.name,
        kind: parse(&row.kind)?,
        organiser_id: GuestId(uuid(&row.organiser_id)?),
        start_date: date(&row.start_date)?,
        end_date: date(&row.end_date)?,
        notes: row.notes,
        running_order: row.running_order,
        sittings: sittings
            .into_iter()
            .map(|s| {
                Ok(CateringSitting {
                    name: s.name,
                    date: date(&s.date)?,
                    time: parse_time(&s.time)?,
                    adults: int(s.adults, "event_sittings.adults")?,
                    children: int(s.children, "event_sittings.children")?,
                    dietary: s.dietary,
                })
            })
            .collect::<StoreResult<_>>()?,
    }))
}

/// The bedroom bookings linked to an event, oldest reference first.
async fn linked_bookings(conn: &mut SqliteConnection, event: EventId) -> StoreResult<Vec<Booking>> {
    let ids: Vec<String> = sqlx::query_scalar(
        "SELECT id FROM bookings WHERE event_id = ? AND deleted_at IS NULL ORDER BY reference",
    )
    .bind(event.to_string())
    .fetch_all(&mut *conn)
    .await?;
    let mut bookings = Vec::with_capacity(ids.len());
    for id in ids {
        if let Some(b) = load_booking(conn, BookingId(uuid(&id)?)).await? {
            bookings.push(b);
        }
    }
    Ok(bookings)
}

async fn event_detail(conn: &mut SqliteConnection, id: EventId) -> StoreResult<EventDetail> {
    let event = load_event(conn, id)
        .await?
        .ok_or(StoreError::NotFound { what: "Event" })?;
    let organiser = load_guest(conn, event.organiser_id).await?;
    let suites = event_suite_bookings(conn, id).await?;
    let spaces = load_spaces(conn, None).await?;
    let room_name = |space: SpaceId| {
        spaces
            .iter()
            .find(|s| s.id == space)
            .map_or_else(|| "?".to_string(), |s| s.name.clone())
    };
    let mut rooms = Vec::new();
    for booking in linked_bookings(conn, id).await? {
        let d = detail(conn, booking.id).await?;
        let (Some(start), Some(end)) = (
            d.booking.rooms.iter().map(|r| r.stay.start).min(),
            d.booking.rooms.iter().map(|r| r.stay.end).max(),
        ) else {
            continue;
        };
        rooms.push(EventRoom {
            booking_id: d.booking.id,
            reference: d.booking.reference.clone(),
            guest_name: d.guest.display_name(),
            status: d.booking.status,
            rooms: d
                .booking
                .rooms
                .iter()
                .map(|r| room_name(r.space_id))
                .collect(),
            stay: DateRange { start, end },
            total: d.total,
        });
    }
    Ok(EventDetail::new(event, organiser, suites, rooms))
}

/// Brings an event's linked bookings in line with its room block: releases
/// (cancels) held rooms no longer needed, reprices held rooms at the group
/// rate and holds new rooms. Returns how many held rooms were released.
async fn sync_block(
    tx: &mut SqliteConnection,
    event: &Event,
    today: NaiveDate,
) -> StoreResult<usize> {
    let bookings = linked_bookings(tx, event.id).await?;
    let linked: Vec<LinkedRoom> = bookings
        .iter()
        .filter_map(|b| {
            let first = b.rooms.first()?;
            Some(LinkedRoom {
                booking_id: b.id,
                status: b.status,
                stay: first.stay,
                price_override: first.price_override,
            })
        })
        .collect();
    let plan = plan_room_block(event.room_block.as_ref(), &linked, today);

    for booking in &bookings {
        let release = plan.release.contains(&booking.id);
        if !release && !plan.reprice.contains(&booking.id) {
            continue;
        }
        let mut changed = booking.clone();
        if release {
            changed.status = BookingStatus::Cancelled;
        } else if let Some(block) = &event.room_block {
            let price = block_room_price(block);
            for room in &mut changed.rooms {
                room.price_override = Some(price);
            }
        }
        let guest = load_guest(tx, booking.guest_id).await?;
        write_booking(tx, &changed, &guest).await?;
    }

    // `if let … && …` (a "let chain") binds `block` only when there is
    // something to add.
    if let Some(block) = &event.room_block
        && plan.add > 0
    {
        let spaces = load_spaces(tx, None).await?;
        let room_types = load_room_types(tx).await?;
        let occupancy = load_occupancy(tx, block.stay.start, block.stay.end).await?;
        let picked = pick_block_rooms(block, &spaces, &occupancy, plan.add)?;
        let organiser = load_guest(tx, event.organiser_id).await?;
        let price = block_room_price(block);
        for space_id in picked {
            let adults = spaces
                .iter()
                .find(|s| s.id == space_id)
                .and_then(|s| s.room_type_id)
                .and_then(|t| room_types.iter().find(|rt| rt.id == t))
                .map_or(2, |t| t.default_occupancy);
            let booking = Booking {
                id: BookingId::new(),
                reference: String::new(),
                guest_id: organiser.id,
                status: BookingStatus::Provisional,
                source: BookingSource::Other,
                notes: format!("Held for {}", event.name.trim()),
                rooms: vec![BookingRoom {
                    id: BookingRoomId::new(),
                    space_id,
                    stay: block.stay,
                    adults,
                    children: 0,
                    board_option_id: None,
                    price_override: Some(price),
                }],
            };
            write_booking(tx, &booking, &organiser).await?;
            sqlx::query("UPDATE bookings SET event_id = ? WHERE id = ?")
                .bind(event.id.to_string())
                .bind(booking.id.to_string())
                .execute(&mut *tx)
                .await?;
        }
    }
    Ok(plan.release.len())
}

async fn write_event(tx: &mut SqliteConnection, event: &Event) -> StoreResult<()> {
    let block = event.room_block.as_ref();
    sqlx::query(
        "INSERT INTO events (id, name, kind, organiser_id, start_date, end_date, notes,
                             running_order, block_room_type_id, block_rooms, block_check_in,
                             block_check_out, block_rate, block_release_date)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT (id) DO UPDATE SET
            name = excluded.name, kind = excluded.kind, organiser_id = excluded.organiser_id,
            start_date = excluded.start_date, end_date = excluded.end_date,
            notes = excluded.notes, running_order = excluded.running_order,
            block_room_type_id = excluded.block_room_type_id,
            block_rooms = excluded.block_rooms, block_check_in = excluded.block_check_in,
            block_check_out = excluded.block_check_out, block_rate = excluded.block_rate,
            block_release_date = excluded.block_release_date,
            updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')",
    )
    .bind(event.id.to_string())
    .bind(event.name.trim())
    .bind(event.kind.as_str())
    .bind(event.organiser_id.to_string())
    .bind(event.start_date.to_string())
    .bind(event.end_date.to_string())
    .bind(event.notes.trim())
    .bind(event.running_order.trim())
    .bind(block.and_then(|b| b.room_type_id).map(|t| t.to_string()))
    .bind(block.map(|b| i64::from(b.rooms)))
    .bind(block.map(|b| b.stay.start.to_string()))
    .bind(block.map(|b| b.stay.end.to_string()))
    .bind(block.map(|b| b.rate.0))
    .bind(block.map(|b| b.release_date.to_string()))
    .execute(&mut *tx)
    .await?;

    sqlx::query("DELETE FROM event_sittings WHERE event_id = ?")
        .bind(event.id.to_string())
        .execute(&mut *tx)
        .await?;
    for (order, s) in event.sittings.iter().enumerate() {
        sqlx::query(
            "INSERT INTO event_sittings (id, event_id, sort_order, name, date, time, adults,
                                         children, dietary)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(uuid::Uuid::now_v7().to_string())
        .bind(event.id.to_string())
        .bind(i64::try_from(order).unwrap_or(i64::MAX))
        .bind(s.name.trim())
        .bind(s.date.to_string())
        .bind(s.time.format("%H:%M").to_string())
        .bind(i64::from(s.adults))
        .bind(i64::from(s.children))
        .bind(s.dietary.trim())
        .execute(&mut *tx)
        .await?;
    }
    Ok(())
}

#[derive(sqlx::FromRow)]
struct EventListRow {
    id: String,
    name: String,
    kind: String,
    start_date: String,
    end_date: String,
    block_release_date: Option<String>,
    first_name: String,
    last_name: String,
    suite_bookings: i64,
    held_rooms: i64,
}

impl EventRepo for SqliteStore {
    async fn list_events(&self, from: NaiveDate) -> StoreResult<Vec<EventListItem>> {
        let rows: Vec<EventListRow> = sqlx::query_as(
            "SELECT e.id, e.name, e.kind, e.start_date, e.end_date, e.block_release_date,
                    g.first_name, g.last_name,
                    (SELECT count(*) FROM suite_bookings sb
                     WHERE sb.event_id = e.id AND sb.deleted_at IS NULL
                       AND sb.status <> 'cancelled') AS suite_bookings,
                    (SELECT count(*) FROM bookings b
                     WHERE b.event_id = e.id AND b.deleted_at IS NULL
                       AND b.status = 'provisional') AS held_rooms
             FROM events e JOIN guests g ON g.id = e.organiser_id
             WHERE e.deleted_at IS NULL AND e.end_date >= ?
             ORDER BY e.start_date, e.name",
        )
        .bind(from.to_string())
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter()
            .map(|r| {
                Ok(EventListItem {
                    id: EventId(uuid(&r.id)?),
                    name: r.name,
                    kind: parse(&r.kind)?,
                    organiser_name: full_name(&r.first_name, &r.last_name),
                    start_date: date(&r.start_date)?,
                    end_date: date(&r.end_date)?,
                    suite_bookings: int(r.suite_bookings, "suite_bookings")?,
                    held_rooms: int(r.held_rooms, "held_rooms")?,
                    release_date: r.block_release_date.as_deref().map(date).transpose()?,
                })
            })
            .collect()
    }

    async fn get_event(&self, id: EventId) -> StoreResult<EventDetail> {
        let mut conn = self.pool.acquire().await?;
        event_detail(&mut conn, id).await
    }

    async fn save_event(
        &self,
        event: &Event,
        organiser: &Guest,
        today: NaiveDate,
    ) -> StoreResult<EventDetail> {
        validate_event(event)?;
        validate_guest(organiser)?;
        // The organiser passed in is the one saved, whatever the event said.
        let event = Event {
            organiser_id: organiser.id,
            ..event.clone()
        };
        let mut tx = self.pool.begin().await?;
        let before = match event_detail(&mut tx, event.id).await {
            Ok(d) => Some(d),
            Err(StoreError::NotFound { .. }) => None,
            Err(e) => return Err(e),
        };
        upsert_guest(&mut tx, organiser).await?;
        write_event(&mut tx, &event).await?;
        sync_block(&mut tx, &event, today).await?;
        let saved = event_detail(&mut tx, event.id).await?;
        let (action, what) = verb(before.is_some());
        record(
            &mut tx,
            Change {
                entity: "event",
                entity_id: event.id.to_string(),
                action,
                summary: format!("{what} event {}", event.name.trim()),
                before: before.as_ref().and_then(json),
                after: json(&saved),
                actor: self.actor.as_ref(),
            },
        )
        .await?;
        tx.commit().await?;
        Ok(saved)
    }

    async fn release_due_blocks(&self, today: NaiveDate) -> StoreResult<u32> {
        let mut tx = self.pool.begin().await?;
        let ids: Vec<String> = sqlx::query_scalar(
            "SELECT id FROM events WHERE deleted_at IS NULL AND block_release_date <= ?",
        )
        .bind(today.to_string())
        .fetch_all(&mut *tx)
        .await?;
        let mut total = 0;
        for id in ids {
            let Some(event) = load_event(&mut tx, EventId(uuid(&id)?)).await? else {
                continue;
            };
            let released = sync_block(&mut tx, &event, today).await?;
            if released == 0 {
                continue;
            }
            record(
                &mut tx,
                Change {
                    entity: "event",
                    entity_id: id,
                    action: "released",
                    summary: format!(
                        "Released {released} held room{} for {}",
                        if released == 1 { "" } else { "s" },
                        event.name
                    ),
                    before: None,
                    after: None,
                    actor: self.actor.as_ref(),
                },
            )
            .await?;
            total += u32::try_from(released).unwrap_or(u32::MAX);
        }
        tx.commit().await?;
        Ok(total)
    }
}
