//! SQLite implementation of tax rates, folios, payments and invoices.
//!
//! A folio's stay and suite charges are worked out from the booking each
//! time (see `booker_core::stay_charges`); only extras and payments are
//! stored. Issuing an invoice freezes the folio as JSON under the next
//! number from `invoice_counter`, inside one transaction, so numbers never
//! skip or repeat.

use booker_core::{
    BookingId, Charge, ChargeCategory, ChargeId, EventId, Folio, FolioOwner, FolioParts, Invoice,
    InvoiceId, InvoiceSettings, InvoiceSummary, Money, Payment, PaymentId, SuiteBookingId, TaxRate,
    TaxRateId, ValidationError, format_invoice_number, stay_charges, suite_charges,
    validate_charge, validate_payment, validate_tax_rate,
};
use chrono::NaiveDate;
use sqlx::SqliteConnection;

use crate::audit::{Change, json, record};
use crate::bookings::{date, detail, load_guest, parse, verb};
use crate::events::{event_suite_bookings, load_event, load_suite_booking};
use crate::repo::InvoiceRepo;
use crate::rows::{int, uuid};
use crate::sqlite::load_spaces;
use crate::{SqliteStore, StoreError, StoreResult};

// ---------------------------------------------------------------------------
// Tax rates and settings
// ---------------------------------------------------------------------------

async fn load_tax_rates(conn: &mut SqliteConnection) -> StoreResult<Vec<TaxRate>> {
    let rows: Vec<(String, String, i64)> = sqlx::query_as(
        "SELECT id, name, basis_points FROM tax_rates WHERE deleted_at IS NULL
         ORDER BY basis_points, name",
    )
    .fetch_all(conn)
    .await?;
    rows.into_iter()
        .map(|(id, name, bp)| {
            Ok(TaxRate {
                id: TaxRateId(uuid(&id)?),
                name,
                basis_points: int(bp, "tax_rates.basis_points")?,
            })
        })
        .collect()
}

#[derive(sqlx::FromRow)]
struct SettingsRow {
    prices_include_tax: bool,
    prefix: String,
    footer: String,
    accommodation_rate: Option<String>,
    board_rate: Option<String>,
    suite_rate: Option<String>,
    food_rate: Option<String>,
    drink_rate: Option<String>,
    other_rate: Option<String>,
}

fn rate_id(s: Option<String>) -> StoreResult<Option<TaxRateId>> {
    s.as_deref().map(uuid).transpose().map(|u| u.map(TaxRateId))
}

/// Saved settings, or the defaults if none were saved yet.
async fn load_settings(conn: &mut SqliteConnection) -> StoreResult<InvoiceSettings> {
    let row: Option<SettingsRow> = sqlx::query_as(
        "SELECT prices_include_tax, prefix, footer, accommodation_rate, board_rate, suite_rate,
                food_rate, drink_rate, other_rate
         FROM invoice_settings WHERE id = 1",
    )
    .fetch_optional(conn)
    .await?;
    let Some(r) = row else {
        return Ok(InvoiceSettings::default());
    };
    Ok(InvoiceSettings {
        prices_include_tax: r.prices_include_tax,
        prefix: r.prefix,
        footer: r.footer,
        accommodation_rate: rate_id(r.accommodation_rate)?,
        board_rate: rate_id(r.board_rate)?,
        suite_rate: rate_id(r.suite_rate)?,
        food_rate: rate_id(r.food_rate)?,
        drink_rate: rate_id(r.drink_rate)?,
        other_rate: rate_id(r.other_rate)?,
    })
}

// ---------------------------------------------------------------------------
// Charges, payments and invoices on a folio
// ---------------------------------------------------------------------------

const CATEGORIES: [(ChargeCategory, &str); 6] = [
    (ChargeCategory::Accommodation, "accommodation"),
    (ChargeCategory::Board, "board"),
    (ChargeCategory::Suite, "suite"),
    (ChargeCategory::Food, "food"),
    (ChargeCategory::Drink, "drink"),
    (ChargeCategory::Other, "other"),
];

fn category_str(c: ChargeCategory) -> &'static str {
    CATEGORIES
        .iter()
        .find(|(k, _)| *k == c)
        .map_or("other", |(_, s)| s)
}

fn parse_category(s: &str) -> StoreResult<ChargeCategory> {
    CATEGORIES
        .iter()
        .find(|(_, name)| *name == s)
        .map(|(c, _)| *c)
        .ok_or_else(|| StoreError::Corrupt(format!("unknown charge category {s:?}")))
}

#[derive(sqlx::FromRow)]
struct ChargeRow {
    id: String,
    date: String,
    category: String,
    description: String,
    quantity: i64,
    unit_price: i64,
    tax_basis_points: i64,
}

impl TryFrom<ChargeRow> for Charge {
    type Error = StoreError;
    fn try_from(r: ChargeRow) -> StoreResult<Self> {
        Ok(Charge {
            id: ChargeId(uuid(&r.id)?),
            date: date(&r.date)?,
            category: parse_category(&r.category)?,
            description: r.description,
            quantity: int(r.quantity, "charges.quantity")?,
            unit_price: Money(r.unit_price),
            tax_basis_points: int(r.tax_basis_points, "charges.tax_basis_points")?,
        })
    }
}

async fn load_extras(conn: &mut SqliteConnection, owner: FolioOwner) -> StoreResult<Vec<Charge>> {
    let (kind, id) = owner.parts();
    let rows: Vec<ChargeRow> = sqlx::query_as(
        "SELECT id, date, category, description, quantity, unit_price, tax_basis_points
         FROM charges WHERE owner_kind = ? AND owner_id = ? AND deleted_at IS NULL
         ORDER BY date, created_at",
    )
    .bind(kind)
    .bind(id)
    .fetch_all(conn)
    .await?;
    rows.into_iter().map(Charge::try_from).collect()
}

#[derive(sqlx::FromRow)]
struct PaymentRow {
    id: String,
    date: String,
    amount: i64,
    method: String,
    is_deposit: bool,
    reference: String,
}

async fn load_payments(
    conn: &mut SqliteConnection,
    owner: FolioOwner,
) -> StoreResult<Vec<Payment>> {
    let (kind, id) = owner.parts();
    let rows: Vec<PaymentRow> = sqlx::query_as(
        "SELECT id, date, amount, method, is_deposit, reference
         FROM payments WHERE owner_kind = ? AND owner_id = ? AND deleted_at IS NULL
         ORDER BY date, created_at",
    )
    .bind(kind)
    .bind(id)
    .fetch_all(conn)
    .await?;
    rows.into_iter()
        .map(|r| {
            Ok(Payment {
                id: PaymentId(uuid(&r.id)?),
                date: date(&r.date)?,
                amount: Money(r.amount),
                method: parse(&r.method)?,
                is_deposit: r.is_deposit,
                reference: r.reference,
            })
        })
        .collect()
}

async fn load_invoice_list(
    conn: &mut SqliteConnection,
    owner: FolioOwner,
) -> StoreResult<Vec<InvoiceSummary>> {
    let (kind, id) = owner.parts();
    let rows: Vec<(String, String, String, i64)> = sqlx::query_as(
        "SELECT id, number, issued_on, gross FROM invoices
         WHERE owner_kind = ? AND owner_id = ? ORDER BY number",
    )
    .bind(kind)
    .bind(id)
    .fetch_all(conn)
    .await?;
    rows.into_iter()
        .map(|(id, number, issued_on, gross)| {
            Ok(InvoiceSummary {
                id: InvoiceId(uuid(&id)?),
                number,
                issued_on: date(&issued_on)?,
                gross: Money(gross),
            })
        })
        .collect()
}

/// Who the folio is for, who pays and the charges worked out from it.
struct Basis {
    title: String,
    bill_to: String,
    bill_address: String,
    automatic: Vec<Charge>,
}

async fn basis(
    conn: &mut SqliteConnection,
    owner: FolioOwner,
    settings: &InvoiceSettings,
    rates: &[TaxRate],
) -> StoreResult<Basis> {
    Ok(match owner {
        FolioOwner::Booking(id) => {
            let d = detail(conn, id).await?;
            let spaces = load_spaces(conn, None).await?;
            let room_name = |space| {
                spaces
                    .iter()
                    .find(|s| s.id == space)
                    .map_or_else(|| "?".to_string(), |s| s.name.clone())
            };
            Basis {
                automatic: stay_charges(&d, room_name, settings, rates),
                title: format!("Booking {}", d.booking.reference),
                bill_to: d.guest.display_name(),
                bill_address: d.guest.address,
            }
        }
        FolioOwner::SuiteBooking(id) => {
            let entry = load_suite_booking(conn, id)
                .await?
                .ok_or(StoreError::NotFound {
                    what: "Suite booking",
                })?;
            let guest = load_guest(conn, entry.booking.guest_id).await?;
            // Hire that's part of an event is billed on the event's folio.
            let automatic = if entry.booking.event_id.is_some() {
                Vec::new()
            } else {
                suite_charges(std::slice::from_ref(&entry), settings, rates)
            };
            Basis {
                title: format!("{}: {}", entry.suite_name, entry.booking.title),
                bill_to: guest.display_name(),
                bill_address: guest.address,
                automatic,
            }
        }
        FolioOwner::Event(id) => {
            let event = load_event(conn, id)
                .await?
                .ok_or(StoreError::NotFound { what: "Event" })?;
            let organiser = load_guest(conn, event.organiser_id).await?;
            let suites = event_suite_bookings(conn, id).await?;
            Basis {
                title: event.name,
                bill_to: organiser.display_name(),
                bill_address: organiser.address,
                automatic: suite_charges(&suites, settings, rates),
            }
        }
    })
}

/// The folio and the billing address to print on an invoice.
pub(crate) async fn load_folio(
    conn: &mut SqliteConnection,
    owner: FolioOwner,
) -> StoreResult<(Folio, String)> {
    let settings = load_settings(conn).await?;
    let rates = load_tax_rates(conn).await?;
    let b = basis(conn, owner, &settings, &rates).await?;
    let folio = Folio::new(FolioParts {
        owner,
        title: b.title,
        bill_to: b.bill_to,
        automatic: b.automatic,
        extras: load_extras(conn, owner).await?,
        payments: load_payments(conn, owner).await?,
        prices_include_tax: settings.prices_include_tax,
        invoices: load_invoice_list(conn, owner).await?,
    });
    Ok((folio, b.bill_address))
}

/// Adds an extra to a bill inside the caller's transaction (no audit entry).
pub(crate) async fn insert_charge(
    tx: &mut SqliteConnection,
    owner: FolioOwner,
    charge: &Charge,
) -> StoreResult<()> {
    let (kind, id) = owner.parts();
    sqlx::query(
        "INSERT INTO charges (id, owner_kind, owner_id, date, category, description,
                              quantity, unit_price, tax_basis_points)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(charge.id.to_string())
    .bind(kind)
    .bind(&id)
    .bind(charge.date.to_string())
    .bind(category_str(charge.category))
    .bind(charge.description.trim())
    .bind(i64::from(charge.quantity))
    .bind(charge.unit_price.0)
    .bind(i64::from(charge.tax_basis_points))
    .execute(&mut *tx)
    .await?;
    Ok(())
}

/// Records a payment inside the caller's transaction (no audit entry).
pub(crate) async fn insert_payment(
    tx: &mut SqliteConnection,
    owner: FolioOwner,
    payment: &Payment,
) -> StoreResult<()> {
    let (kind, id) = owner.parts();
    sqlx::query(
        "INSERT INTO payments (id, owner_kind, owner_id, date, amount, method, is_deposit,
                               reference)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(payment.id.to_string())
    .bind(kind)
    .bind(&id)
    .bind(payment.date.to_string())
    .bind(payment.amount.0)
    .bind(payment.method.as_str())
    .bind(payment.is_deposit)
    .bind(payment.reference.trim())
    .execute(&mut *tx)
    .await?;
    Ok(())
}

/// Freezes a folio as the next-numbered invoice inside the caller's
/// transaction (no audit entry).
pub(crate) async fn insert_invoice(
    tx: &mut SqliteConnection,
    owner: FolioOwner,
    issued_on: NaiveDate,
) -> StoreResult<Invoice> {
    let settings = load_settings(&mut *tx).await?;
    let (folio, bill_address) = load_folio(&mut *tx, owner).await?;
    if folio.lines.is_empty() {
        return Err(ValidationError::Empty { field: "The folio" }.into());
    }
    // Taking the number inside this transaction means a failed save
    // gives it back, so numbers never skip.
    let n: i64 = sqlx::query_scalar(
        "INSERT INTO invoice_counter (id, last_number) VALUES (1, 1)
             ON CONFLICT (id) DO UPDATE SET last_number = last_number + 1
             RETURNING last_number",
    )
    .fetch_one(&mut *tx)
    .await?;
    let invoice = Invoice {
        id: InvoiceId::new(),
        number: format_invoice_number(&settings.prefix, int(n, "invoice_counter")?),
        owner,
        issued_on,
        bill_to: folio.bill_to,
        bill_address,
        title: folio.title,
        lines: folio.lines,
        payments: folio.payments,
        totals: folio.totals,
        prices_include_tax: folio.prices_include_tax,
        footer: settings.footer,
    };
    let document = serde_json::to_string(&invoice)
        .map_err(|e| StoreError::Corrupt(format!("can't save invoice: {e}")))?;
    let (kind, id) = owner.parts();
    sqlx::query(
        "INSERT INTO invoices (id, number, owner_kind, owner_id, issued_on, gross, document)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(invoice.id.to_string())
    .bind(&invoice.number)
    .bind(kind)
    .bind(id)
    .bind(issued_on.to_string())
    .bind(invoice.totals.gross.0)
    .bind(document)
    .execute(&mut *tx)
    .await?;
    Ok(invoice)
}

/// The owner a stored charge belongs to, from its two columns.
fn owner_from(kind: &str, id: &str) -> StoreResult<FolioOwner> {
    let u = uuid(id)?;
    Ok(match kind {
        "booking" => FolioOwner::Booking(BookingId(u)),
        "suite_booking" => FolioOwner::SuiteBooking(SuiteBookingId(u)),
        "event" => FolioOwner::Event(EventId(u)),
        _ => return Err(StoreError::Corrupt(format!("unknown folio owner {kind:?}"))),
    })
}

fn pounds(m: Money) -> String {
    format!("£{}.{:02}", m.0.abs() / 100, m.0.abs() % 100)
}

impl InvoiceRepo for SqliteStore {
    async fn list_tax_rates(&self) -> StoreResult<Vec<TaxRate>> {
        let mut conn = self.pool.acquire().await?;
        load_tax_rates(&mut conn).await
    }

    async fn save_tax_rate(&self, rate: &TaxRate) -> StoreResult<()> {
        validate_tax_rate(rate)?;
        let mut tx = self.pool.begin().await?;
        let existed: Option<String> =
            sqlx::query_scalar("SELECT id FROM tax_rates WHERE id = ? AND deleted_at IS NULL")
                .bind(rate.id.to_string())
                .fetch_optional(&mut *tx)
                .await?;
        sqlx::query(
            "INSERT INTO tax_rates (id, name, basis_points) VALUES (?, ?, ?)
             ON CONFLICT (id) DO UPDATE SET name = excluded.name,
                basis_points = excluded.basis_points,
                updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')",
        )
        .bind(rate.id.to_string())
        .bind(rate.name.trim())
        .bind(i64::from(rate.basis_points))
        .execute(&mut *tx)
        .await?;
        let (action, what) = verb(existed.is_some());
        record(
            &mut tx,
            Change {
                entity: "tax_rate",
                entity_id: rate.id.to_string(),
                action,
                summary: format!("{what} tax rate {}", rate.name.trim()),
                before: None,
                after: json(rate),
                actor: self.actor.as_ref(),
            },
        )
        .await?;
        tx.commit().await?;
        Ok(())
    }

    async fn get_invoice_settings(&self) -> StoreResult<InvoiceSettings> {
        let mut conn = self.pool.acquire().await?;
        load_settings(&mut conn).await
    }

    async fn save_invoice_settings(&self, settings: &InvoiceSettings) -> StoreResult<()> {
        let mut tx = self.pool.begin().await?;
        let before = load_settings(&mut tx).await?;
        let id = |r: Option<TaxRateId>| r.map(|r| r.to_string());
        sqlx::query(
            "INSERT INTO invoice_settings (id, prices_include_tax, prefix, footer,
                accommodation_rate, board_rate, suite_rate, food_rate, drink_rate, other_rate)
             VALUES (1, ?, ?, ?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT (id) DO UPDATE SET prices_include_tax = excluded.prices_include_tax,
                prefix = excluded.prefix, footer = excluded.footer,
                accommodation_rate = excluded.accommodation_rate,
                board_rate = excluded.board_rate, suite_rate = excluded.suite_rate,
                food_rate = excluded.food_rate, drink_rate = excluded.drink_rate,
                other_rate = excluded.other_rate,
                updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')",
        )
        .bind(settings.prices_include_tax)
        .bind(settings.prefix.trim())
        .bind(settings.footer.trim())
        .bind(id(settings.accommodation_rate))
        .bind(id(settings.board_rate))
        .bind(id(settings.suite_rate))
        .bind(id(settings.food_rate))
        .bind(id(settings.drink_rate))
        .bind(id(settings.other_rate))
        .execute(&mut *tx)
        .await?;
        record(
            &mut tx,
            Change {
                entity: "invoice_settings",
                entity_id: "1".into(),
                action: "updated",
                summary: "Changed the invoice settings".into(),
                before: json(&before),
                after: json(settings),
                actor: self.actor.as_ref(),
            },
        )
        .await?;
        tx.commit().await?;
        Ok(())
    }

    async fn get_folio(&self, owner: FolioOwner) -> StoreResult<Folio> {
        let mut conn = self.pool.acquire().await?;
        Ok(load_folio(&mut conn, owner).await?.0)
    }

    async fn add_charge(&self, owner: FolioOwner, charge: &Charge) -> StoreResult<Folio> {
        validate_charge(charge)?;
        let mut tx = self.pool.begin().await?;
        // Fails with NotFound if the booking or event doesn't exist.
        let (before, _) = load_folio(&mut tx, owner).await?;
        insert_charge(&mut tx, owner, charge).await?;
        record(
            &mut tx,
            Change {
                entity: "charge",
                entity_id: charge.id.to_string(),
                action: "created",
                summary: format!(
                    "Charged {} × {} to {}",
                    charge.quantity,
                    charge.description.trim(),
                    before.title
                ),
                before: None,
                after: json(charge),
                actor: self.actor.as_ref(),
            },
        )
        .await?;
        let (folio, _) = load_folio(&mut tx, owner).await?;
        tx.commit().await?;
        Ok(folio)
    }

    async fn remove_charge(&self, id: ChargeId) -> StoreResult<Folio> {
        let mut tx = self.pool.begin().await?;
        let row: Option<(String, String, String)> = sqlx::query_as(
            "SELECT owner_kind, owner_id, description FROM charges
             WHERE id = ? AND deleted_at IS NULL",
        )
        .bind(id.to_string())
        .fetch_optional(&mut *tx)
        .await?;
        let (kind, owner_id, description) = row.ok_or(StoreError::NotFound { what: "Charge" })?;
        let owner = owner_from(&kind, &owner_id)?;
        sqlx::query(
            "UPDATE charges SET deleted_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE id = ?",
        )
        .bind(id.to_string())
        .execute(&mut *tx)
        .await?;
        let (folio, _) = load_folio(&mut tx, owner).await?;
        record(
            &mut tx,
            Change {
                entity: "charge",
                entity_id: id.to_string(),
                action: "deleted",
                summary: format!("Removed {description} from {}", folio.title),
                before: None,
                after: None,
                actor: self.actor.as_ref(),
            },
        )
        .await?;
        tx.commit().await?;
        Ok(folio)
    }

    async fn add_payment(&self, owner: FolioOwner, payment: &Payment) -> StoreResult<Folio> {
        let mut tx = self.pool.begin().await?;
        let (before, _) = load_folio(&mut tx, owner).await?;
        validate_payment(payment, before.totals.paid)?;
        insert_payment(&mut tx, owner, payment).await?;
        let what = if payment.amount.0 < 0 {
            "Refunded"
        } else if payment.is_deposit {
            "Took a deposit of"
        } else {
            "Took"
        };
        record(
            &mut tx,
            Change {
                entity: "payment",
                entity_id: payment.id.to_string(),
                action: "created",
                summary: format!(
                    "{what} {} by {} for {}",
                    pounds(payment.amount),
                    payment.method.as_str().replace('_', " "),
                    before.title
                ),
                before: None,
                after: json(payment),
                actor: self.actor.as_ref(),
            },
        )
        .await?;
        let (folio, _) = load_folio(&mut tx, owner).await?;
        tx.commit().await?;
        Ok(folio)
    }

    async fn issue_invoice(&self, owner: FolioOwner, issued_on: NaiveDate) -> StoreResult<Invoice> {
        let mut tx = self.pool.begin().await?;
        let invoice = insert_invoice(&mut tx, owner, issued_on).await?;
        record(
            &mut tx,
            Change {
                entity: "invoice",
                entity_id: invoice.id.to_string(),
                action: "created",
                summary: format!("Issued invoice {} for {}", invoice.number, invoice.title),
                before: None,
                after: None,
                actor: self.actor.as_ref(),
            },
        )
        .await?;
        tx.commit().await?;
        Ok(invoice)
    }

    async fn get_invoice(&self, id: InvoiceId) -> StoreResult<Invoice> {
        let document: Option<String> =
            sqlx::query_scalar("SELECT document FROM invoices WHERE id = ?")
                .bind(id.to_string())
                .fetch_optional(&self.pool)
                .await?;
        let document = document.ok_or(StoreError::NotFound { what: "Invoice" })?;
        serde_json::from_str(&document)
            .map_err(|e| StoreError::Corrupt(format!("unreadable invoice: {e}")))
    }
}
