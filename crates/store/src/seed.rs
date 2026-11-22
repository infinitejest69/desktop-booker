//! Loads the demo hotel (The Lanterns Hotel, Kelmouth) built by
//! [`booker_core::demo_hotel`].

use std::collections::HashMap;

use booker_core::{
    BookingId, BookingStatus, DEMO_SEED, DemoAccount, DemoHotel, FolioOwner, Guest, GuestId, Money,
    Payment, PaymentId, demo_hotel, format_reference,
};
use chrono::{Datelike, Local, NaiveDate};
use sqlx::SqliteConnection;

use crate::audit::{Change, record};
use crate::bookings::{BookingContext, write_booking_with};
use crate::invoicing::{insert_charge, insert_invoice, insert_payment, load_folio};
use crate::operations::mark_room;
use crate::repo::{
    EventRepo, FloorRepo, InvoiceRepo, RateRepo, ReportRepo, RoomTypeRepo, SettingsRepo, SpaceRepo,
    SuiteRepo,
};
use crate::{SqliteStore, StoreError, StoreResult};

impl SqliteStore {
    /// Fills an empty database with the demo hotel as it looks today.
    /// Returns `false` and does nothing if any floors already exist.
    pub async fn seed_demo(&self) -> StoreResult<bool> {
        if !self.list_floors().await?.is_empty() {
            return Ok(false);
        }
        let today = Local::now().date_naive();
        self.load_demo_hotel(&demo_hotel(DEMO_SEED, today), today)
            .await?;
        Ok(true)
    }

    /// Writes a generated hotel. The building and prices go through the
    /// usual repository methods (so they're validated the same way as typed
    /// in); the thousands of stays and their bills go in one transaction,
    /// which keeps loading quick.
    pub async fn load_demo_hotel(&self, h: &DemoHotel, today: NaiveDate) -> StoreResult<()> {
        self.save_settings(&h.settings).await?;
        for t in &h.room_types {
            self.save_room_type(t).await?;
        }
        for f in &h.floors {
            self.save_floor(f).await?;
        }
        for s in &h.spaces {
            self.save_space(s).await?;
        }
        for b in &h.boards {
            self.save_board_option(b).await?;
        }
        for r in &h.rate_rules {
            self.save_rate_rule(r).await?;
        }
        for t in &h.tax_rates {
            self.save_tax_rate(t).await?;
        }
        self.save_invoice_settings(&h.invoice_settings).await?;
        for (suite, rates) in &h.suite_rates {
            self.save_suite_rates(*suite, rates).await?;
        }
        for r in &h.saved_reports {
            self.save_saved_report(r).await?;
        }
        for d in &h.dashboards {
            self.save_dashboard(d).await?;
        }

        // Stays first: wedding room blocks then take rooms the stays left free.
        let guests: HashMap<GuestId, &Guest> = h.guests.iter().map(|g| (g.id, g)).collect();
        let mut tx = self.pool.begin().await?;
        let context = BookingContext::load(&mut tx).await?;
        for s in &h.stays {
            let guest = guests
                .get(&s.booking.guest_id)
                .ok_or(StoreError::NotFound { what: "Guest" })?;
            write_booking_with(&mut tx, &context, &s.booking, guest).await?;
            if s.booking.status != BookingStatus::Cancelled {
                // A price agreed by hand (coach tours) needs the full folio.
                let quick = s.booking.rooms.iter().all(|r| r.price_override.is_none());
                apply_account(
                    &mut tx,
                    FolioOwner::Booking(s.booking.id),
                    &s.account,
                    quick,
                )
                .await?;
            }
        }
        date_bookings(&mut tx, h).await?;
        tx.commit().await?;

        for e in &h.events {
            self.save_event(&e.event, &e.organiser, today).await?;
            for b in &e.suite_bookings {
                self.save_suite_booking(b, &e.organiser).await?;
            }
        }
        let mut tx = self.pool.begin().await?;
        for e in &h.events {
            apply_account(&mut tx, FolioOwner::Event(e.event.id), &e.account, false).await?;
        }
        // Invoices last, in date order, so their numbers run with the calendar.
        let mut invoices: Vec<(NaiveDate, FolioOwner)> = h
            .stays
            .iter()
            .filter_map(|s| {
                s.account
                    .invoice_on
                    .map(|d| (d, FolioOwner::Booking(s.booking.id)))
            })
            .chain(h.events.iter().filter_map(|e| {
                e.account
                    .invoice_on
                    .map(|d| (d, FolioOwner::Event(e.event.id)))
            }))
            .collect();
        invoices.sort_by_key(|(d, _)| *d);
        for (date, owner) in invoices {
            insert_invoice(&mut tx, owner, date).await?;
        }
        for (space, status) in &h.housekeeping {
            mark_room(&mut tx, *space, *status, "").await?;
        }
        record(
            &mut tx,
            Change {
                entity: "setup",
                entity_id: "demo".into(),
                action: "created",
                summary: format!(
                    "Loaded the demo hotel ({}): {} bookings, {} events",
                    h.settings.hotel_name,
                    h.stays.len(),
                    h.events.len()
                ),
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

/// Adds the extras and deposit, then pays off what's left if the stay or
/// event is settled. `quick` works the balance out from the night prices
/// (see [`stay_balance`]) instead of building the folio.
async fn apply_account(
    tx: &mut SqliteConnection,
    owner: FolioOwner,
    account: &DemoAccount,
    quick: bool,
) -> StoreResult<()> {
    for c in &account.extras {
        insert_charge(tx, owner, c).await?;
    }
    if let Some(deposit) = &account.deposit {
        insert_payment(tx, owner, deposit).await?;
    }
    if let Some((date, method)) = account.settle {
        let balance = match owner {
            FolioOwner::Booking(id) if quick => stay_balance(tx, id).await?,
            _ => load_folio(tx, owner).await?.0.totals.balance,
        };
        if balance > Money::ZERO {
            let payment = Payment {
                id: PaymentId::new(),
                date,
                amount: balance,
                method,
                is_deposit: false,
                reference: String::new(),
            };
            insert_payment(tx, owner, &payment).await?;
        }
    }
    Ok(())
}

/// What a demo stay still owes. The demo's prices include VAT and this is
/// only used when no price is set by hand, so the bill is simply every night's price plus the
/// extras, less what's been paid; much quicker than building the whole
/// folio thousands of times. (A store test checks the folio agrees.)
async fn stay_balance(tx: &mut SqliteConnection, id: BookingId) -> StoreResult<Money> {
    let id = id.to_string();
    let owed: i64 = sqlx::query_scalar(
        "SELECT
            COALESCE((SELECT SUM(n.room_price + n.supplements + n.board_price + n.parking_price)
                      FROM room_nights n JOIN booking_rooms r ON r.id = n.booking_room_id
                      WHERE r.booking_id = ?1), 0)
          + COALESCE((SELECT SUM(quantity * unit_price) FROM charges
                      WHERE owner_kind = 'booking' AND owner_id = ?1 AND deleted_at IS NULL), 0)
          - COALESCE((SELECT SUM(amount) FROM payments
                      WHERE owner_kind = 'booking' AND owner_id = ?1 AND deleted_at IS NULL), 0)",
    )
    .bind(id)
    .fetch_one(&mut *tx)
    .await?;
    Ok(Money(owed))
}

/// Gives each booking the day it was made and a reference from that year
/// (2025-0001, 2025-0002 … 2026-0001), and moves the yearly counters on so
/// new bookings carry on from there.
async fn date_bookings(tx: &mut SqliteConnection, h: &DemoHotel) -> StoreResult<()> {
    // Clear the references first so renumbering never hits the unique index.
    sqlx::query("UPDATE bookings SET reference = 'demo-' || id")
        .execute(&mut *tx)
        .await?;
    let mut order: Vec<_> = h.stays.iter().collect();
    order.sort_by_key(|s| s.booked_on);
    let mut counters: HashMap<i32, u32> = HashMap::new();
    for (i, s) in order.iter().enumerate() {
        let year = s.booked_on.year();
        let n = counters.entry(year).or_default();
        *n += 1;
        // Booked some time in the working day, UTC.
        let made = format!(
            "{}T{:02}:{:02}:00.000Z",
            s.booked_on,
            9 + i % 9,
            (i * 7) % 60
        );
        sqlx::query("UPDATE bookings SET reference = ?, created_at = ? WHERE id = ?")
            .bind(format_reference(year, *n))
            .bind(made)
            .bind(s.booking.id.to_string())
            .execute(&mut *tx)
            .await?;
    }
    sqlx::query("DELETE FROM booking_counters")
        .execute(&mut *tx)
        .await?;
    for (year, last) in counters {
        sqlx::query("INSERT INTO booking_counters (year, last_number) VALUES (?, ?)")
            .bind(year)
            .bind(i64::from(last))
            .execute(&mut *tx)
            .await?;
    }
    Ok(())
}
