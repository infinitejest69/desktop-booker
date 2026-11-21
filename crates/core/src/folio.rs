//! Building a folio: who it belongs to, the charges that come from the
//! stay or suite hire itself, and the invoices issued from it.
//!
//! Stay and suite charges are *worked out* from the saved night prices and
//! suite prices each time, so they always match the booking. Extras (bar,
//! restaurant…) are stored. Issuing an invoice copies every line, with its
//! amounts, under a sequential number.

use std::collections::BTreeMap;

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use crate::booking::BookingDetail;
use crate::ids::{BookingId, ChargeId, EventId, InvoiceId, SpaceId, SuiteBookingId, TaxRateId};
use crate::invoices::{
    Charge, ChargeCategory, FolioTotals, LineAmounts, Payment, TaxRate, folio_totals,
};
use crate::money::Money;
use crate::suites::{SuiteBookingEntry, SuiteBookingStatus};

/// What a folio is for. `#[serde(tag = "kind", content = "id")]` makes it
/// `{ "kind": "booking", "id": "…" }` in JSON.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", content = "id", rename_all = "snake_case")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub enum FolioOwner {
    Booking(BookingId),
    SuiteBooking(SuiteBookingId),
    Event(EventId),
}

impl FolioOwner {
    /// The two columns an owner is stored in.
    pub fn parts(self) -> (&'static str, String) {
        match self {
            Self::Booking(id) => ("booking", id.to_string()),
            Self::SuiteBooking(id) => ("suite_booking", id.to_string()),
            Self::Event(id) => ("event", id.to_string()),
        }
    }
}

/// How invoices are worked out and printed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct InvoiceSettings {
    /// True when prices already include tax (usual for UK hotels).
    pub prices_include_tax: bool,
    /// Put before the number, e.g. "INV-".
    pub prefix: String,
    /// Printed at the foot: VAT number, bank details, thanks.
    pub footer: String,
    /// The tax rate charged on each kind of charge; `None` means no tax.
    pub accommodation_rate: Option<TaxRateId>,
    pub board_rate: Option<TaxRateId>,
    pub suite_rate: Option<TaxRateId>,
    pub food_rate: Option<TaxRateId>,
    pub drink_rate: Option<TaxRateId>,
    pub other_rate: Option<TaxRateId>,
}

impl Default for InvoiceSettings {
    fn default() -> Self {
        Self {
            prices_include_tax: true,
            prefix: "INV-".into(),
            footer: String::new(),
            accommodation_rate: None,
            board_rate: None,
            suite_rate: None,
            food_rate: None,
            drink_rate: None,
            other_rate: None,
        }
    }
}

impl InvoiceSettings {
    /// The tax, in basis points, for a kind of charge.
    pub fn basis_points_for(&self, category: ChargeCategory, rates: &[TaxRate]) -> u32 {
        let id = match category {
            ChargeCategory::Accommodation => self.accommodation_rate,
            ChargeCategory::Board => self.board_rate,
            ChargeCategory::Suite => self.suite_rate,
            ChargeCategory::Food => self.food_rate,
            ChargeCategory::Drink => self.drink_rate,
            ChargeCategory::Other => self.other_rate,
        };
        id.and_then(|id| rates.iter().find(|r| r.id == id))
            .map_or(0, |r| r.basis_points)
    }
}

/// One line of a folio as shown: the charge, its amounts and whether it
/// was worked out from the booking (and so can't be removed by hand).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct FolioLine {
    pub charge: Charge,
    pub amounts: LineAmounts,
    pub automatic: bool,
}

/// An issued invoice, in short, for lists.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct InvoiceSummary {
    pub id: InvoiceId,
    pub number: String,
    pub issued_on: NaiveDate,
    pub gross: Money,
}

/// A booking's (or event's) account as the folio screen shows it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct Folio {
    pub owner: FolioOwner,
    /// e.g. "Booking 2026-0012" or the event's name.
    pub title: String,
    pub bill_to: String,
    pub lines: Vec<FolioLine>,
    pub payments: Vec<Payment>,
    pub totals: FolioTotals,
    pub prices_include_tax: bool,
    pub invoices: Vec<InvoiceSummary>,
}

/// The parts a [`Folio`] is made from, so `Folio::new` takes one argument
/// rather than eight.
pub struct FolioParts {
    pub owner: FolioOwner,
    pub title: String,
    pub bill_to: String,
    pub automatic: Vec<Charge>,
    pub extras: Vec<Charge>,
    pub payments: Vec<Payment>,
    pub prices_include_tax: bool,
    pub invoices: Vec<InvoiceSummary>,
}

impl Folio {
    /// Puts worked-out charges first, then extras by date, and adds it all up.
    pub fn new(parts: FolioParts) -> Self {
        let FolioParts {
            owner,
            title,
            bill_to,
            automatic,
            mut extras,
            payments,
            prices_include_tax,
            invoices,
        } = parts;
        extras.sort_by_key(|c| c.date);
        let flagged: Vec<(Charge, bool)> = automatic
            .into_iter()
            .map(|c| (c, true))
            .chain(extras.into_iter().map(|c| (c, false)))
            .collect();
        let charges: Vec<Charge> = flagged.iter().map(|(c, _)| c.clone()).collect();
        let totals = folio_totals(&charges, &payments, prices_include_tax);
        let lines = flagged
            .into_iter()
            .zip(&totals.lines)
            .map(|((charge, automatic), amounts)| FolioLine {
                charge,
                amounts: *amounts,
                automatic,
            })
            .collect();
        Self {
            owner,
            title,
            bill_to,
            lines,
            payments,
            totals,
            prices_include_tax,
            invoices,
        }
    }
}

/// An issued invoice: a frozen copy of the folio's lines and totals.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct Invoice {
    pub id: InvoiceId,
    pub number: String,
    pub owner: FolioOwner,
    pub issued_on: NaiveDate,
    pub bill_to: String,
    pub bill_address: String,
    pub title: String,
    pub lines: Vec<FolioLine>,
    pub payments: Vec<Payment>,
    pub totals: FolioTotals,
    pub prices_include_tax: bool,
    pub footer: String,
}

fn stay_line(
    date: NaiveDate,
    category: ChargeCategory,
    description: String,
    quantity: usize,
    unit_price: Money,
    tax_basis_points: u32,
) -> Charge {
    Charge {
        id: ChargeId::new(),
        date,
        category,
        description,
        quantity: u16::try_from(quantity).unwrap_or(u16::MAX),
        unit_price,
        tax_basis_points,
    }
}

/// Room and board charges for a bedroom booking, from its saved night
/// prices. Nights at the same price are grouped into one line; a price set
/// by hand becomes a single "agreed price" line. Cancelled and no-show
/// bookings have no nights, so no charges.
pub fn stay_charges(
    detail: &BookingDetail,
    room_name: impl Fn(SpaceId) -> String,
    settings: &InvoiceSettings,
    rates: &[TaxRate],
) -> Vec<Charge> {
    let room_tax = settings.basis_points_for(ChargeCategory::Accommodation, rates);
    let board_tax = settings.basis_points_for(ChargeCategory::Board, rates);
    let mut out = Vec::new();
    for (room, pricing) in detail.booking.rooms.iter().zip(&detail.pricing) {
        let Some(first) = pricing.nights.first() else {
            continue;
        };
        let name = room_name(room.space_id);
        if let Some(agreed) = room.price_override {
            let n = pricing.nights.len();
            out.push(stay_line(
                first.night,
                ChargeCategory::Accommodation,
                format!(
                    "Room {name}, {n} night{} from {} (agreed price)",
                    if n == 1 { "" } else { "s" },
                    first.night.format("%-d %b")
                ),
                1,
                agreed,
                room_tax,
            ));
            continue;
        }
        // Group nights by (room price, board price). A BTreeMap keeps the
        // groups in a fixed order, so the lines come out the same every time.
        let mut by_price: BTreeMap<(i64, i64), Vec<NaiveDate>> = BTreeMap::new();
        for n in &pricing.nights {
            by_price
                .entry(((n.room + n.supplements).0, n.board.0))
                .or_default()
                .push(n.night);
        }
        for ((room_price, board_price), nights) in by_price {
            let from = nights[0];
            let when = if nights.len() == 1 {
                from.format("%-d %b").to_string()
            } else {
                format!("{} nights from {}", nights.len(), from.format("%-d %b"))
            };
            out.push(stay_line(
                from,
                ChargeCategory::Accommodation,
                format!("Room {name}, {when}"),
                nights.len(),
                Money(room_price),
                room_tax,
            ));
            if board_price != 0 {
                out.push(stay_line(
                    from,
                    ChargeCategory::Board,
                    format!("Meals for room {name}, {when}"),
                    nights.len(),
                    Money(board_price),
                    board_tax,
                ));
            }
        }
    }
    // A stable sort keeps each room line before its meals line.
    out.sort_by_key(|c| c.date);
    out
}

/// One charge per live suite booking, at the price fixed when booked.
pub fn suite_charges(
    suites: &[SuiteBookingEntry],
    settings: &InvoiceSettings,
    rates: &[TaxRate],
) -> Vec<Charge> {
    let tax = settings.basis_points_for(ChargeCategory::Suite, rates);
    suites
        .iter()
        .filter(|s| s.booking.status != SuiteBookingStatus::Cancelled)
        .map(|s| {
            stay_line(
                s.booking.start.date(),
                ChargeCategory::Suite,
                format!(
                    "{}: {}, {} {}",
                    s.suite_name,
                    s.booking.title,
                    s.booking.start.format("%-d %b %H:%M"),
                    s.booking.end.format("to %H:%M")
                ),
                1,
                s.price,
                tax,
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::booking::{Booking, BookingRoom, BookingSource, BookingStatus, Guest, RoomPricing};
    use crate::dates::DateRange;
    use crate::ids::{BookingRoomId, GuestId};
    use crate::rates::NightPrice;

    fn d(s: &str) -> NaiveDate {
        s.parse().unwrap()
    }

    fn night(day: &str, room: i64, board: i64) -> NightPrice {
        NightPrice {
            night: d(day),
            room: Money(room),
            supplements: Money::ZERO,
            board: Money(board),
            total: Money(room + board),
            rules: Vec::new(),
        }
    }

    fn vat() -> (InvoiceSettings, Vec<TaxRate>) {
        let standard = TaxRate {
            id: TaxRateId::new(),
            name: "Standard".into(),
            basis_points: 2000,
        };
        let settings = InvoiceSettings {
            accommodation_rate: Some(standard.id),
            board_rate: Some(standard.id),
            ..InvoiceSettings::default()
        };
        (settings, vec![standard])
    }

    fn detail(nights: Vec<NightPrice>, price_override: Option<Money>) -> BookingDetail {
        let room = BookingRoom {
            id: BookingRoomId::new(),
            space_id: SpaceId::new(),
            stay: DateRange::new(d("2026-06-19"), d("2026-06-22")).unwrap(),
            adults: 2,
            children: 0,
            board_option_id: None,
            price_override,
        };
        BookingDetail {
            pricing: vec![RoomPricing {
                booking_room_id: room.id,
                calculated: nights.iter().map(|n| n.total).sum(),
                total: Money::ZERO,
                nights,
            }],
            booking: Booking {
                id: BookingId::new(),
                reference: "2026-0001".into(),
                guest_id: GuestId::new(),
                status: BookingStatus::Confirmed,
                source: BookingSource::Phone,
                notes: String::new(),
                rooms: vec![room],
            },
            guest: Guest {
                id: GuestId::new(),
                first_name: "Jo".into(),
                last_name: "Hughes".into(),
                email: String::new(),
                phone: String::new(),
                address: String::new(),
                notes: String::new(),
            },
            total: Money::ZERO,
        }
    }

    fn folio(owner: BookingId, automatic: Vec<Charge>, extras: Vec<Charge>) -> Folio {
        Folio::new(FolioParts {
            owner: FolioOwner::Booking(owner),
            title: "Booking 2026-0001".into(),
            bill_to: "Jo Hughes".into(),
            automatic,
            extras,
            payments: Vec::new(),
            prices_include_tax: true,
            invoices: Vec::new(),
        })
    }

    #[test]
    fn nights_at_the_same_price_share_a_line_and_board_is_separate() {
        let (settings, rates) = vat();
        let detail = detail(
            vec![
                night("2026-06-19", 11_000, 2_400),
                night("2026-06-20", 11_000, 2_400),
                night("2026-06-21", 9_500, 2_400),
            ],
            None,
        );
        let charges = stay_charges(&detail, |_| "101".into(), &settings, &rates);
        let summary: Vec<_> = charges
            .iter()
            .map(|c| (c.category, c.quantity, c.unit_price, c.description.as_str()))
            .collect();
        assert_eq!(
            summary,
            vec![
                (
                    ChargeCategory::Accommodation,
                    2,
                    Money(11_000),
                    "Room 101, 2 nights from 19 Jun"
                ),
                (
                    ChargeCategory::Board,
                    2,
                    Money(2_400),
                    "Meals for room 101, 2 nights from 19 Jun"
                ),
                (
                    ChargeCategory::Accommodation,
                    1,
                    Money(9_500),
                    "Room 101, 21 Jun"
                ),
                (
                    ChargeCategory::Board,
                    1,
                    Money(2_400),
                    "Meals for room 101, 21 Jun"
                ),
            ]
        );
        assert!(charges.iter().all(|c| c.tax_basis_points == 2000));
        // The folio adds up to the booking's own total.
        let f = folio(detail.booking.id, charges, Vec::new());
        assert_eq!(f.totals.gross, Money(3 * 2_400 + 2 * 11_000 + 9_500));
    }

    #[test]
    fn an_agreed_price_is_one_line_and_untaxed_categories_are_zero() {
        let (mut settings, rates) = vat();
        settings.accommodation_rate = None;
        let detail = detail(vec![night("2026-06-19", 11_000, 0)], Some(Money(8_500)));
        let charges = stay_charges(&detail, |_| "101".into(), &settings, &rates);
        assert_eq!(charges.len(), 1);
        assert_eq!(charges[0].unit_price, Money(8_500));
        assert_eq!(charges[0].tax_basis_points, 0);
        assert_eq!(
            charges[0].description,
            "Room 101, 1 night from 19 Jun (agreed price)"
        );
    }

    #[test]
    fn folio_lines_keep_extras_after_stay_charges() {
        let (settings, rates) = vat();
        let detail = detail(vec![night("2026-06-19", 10_000, 0)], None);
        let stay = stay_charges(&detail, |_| "101".into(), &settings, &rates);
        let bar = Charge {
            id: ChargeId::new(),
            date: d("2026-06-19"),
            category: ChargeCategory::Drink,
            description: "Bar".into(),
            quantity: 1,
            unit_price: Money(1_200),
            tax_basis_points: 2000,
        };
        let f = folio(detail.booking.id, stay, vec![bar]);
        assert_eq!(
            f.lines.iter().map(|l| l.automatic).collect::<Vec<_>>(),
            vec![true, false]
        );
        assert_eq!(f.lines[1].amounts.tax, Money(200));
        assert_eq!(f.totals.balance, Money(11_200));
    }
}
