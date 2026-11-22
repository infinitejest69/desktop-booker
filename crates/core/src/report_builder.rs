//! The report builder (ADR 0005): a report is a *measure* (what to count or
//! add up), a *split* (what to group it by), a *period* and a chart. Admins
//! save reports and arrange them on dashboards.
//!
//! The store hands over plain *fact rows* for the period ([`ReportFacts`]);
//! everything here is pure, so it is tested without a database.

use std::collections::{BTreeMap, HashMap, HashSet};

use chrono::{Datelike, Duration, NaiveDate};
use serde::{Deserialize, Serialize};

use crate::booking::{BookingSource, BookingStatus};
use crate::dates::DateRange;
use crate::events::EventKind;
use crate::ids::{
    BoardOptionId, BookingId, DashboardId, FloorId, GuestId, RoomTypeId, SavedReportId, SpaceId,
};
use crate::invoices::PaymentMethod;
use crate::money::Money;
use crate::validation::{ValidationError, ValidationResult};

// ---------------------------------------------------------------------------
// What a report is
// ---------------------------------------------------------------------------

/// What a report counts or adds up.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub enum Measure {
    // Rooms
    RoomNightsSold,
    Occupancy,
    RoomRevenue,
    Adr,
    RevPar,
    GuestsStaying,
    // Bookings
    Arrivals,
    Departures,
    BookingsMade,
    Cancellations,
    NoShows,
    AverageStay,
    AverageLeadTime,
    // Money
    TotalRevenue,
    BoardRevenue,
    ExtrasRevenue,
    PaymentsTaken,
    DepositsTaken,
    // Suites and events
    SuiteHours,
    SuiteRevenue,
    Events,
    EventAttendees,
    // Guests
    NewGuests,
    ReturningGuests,
}

/// Every measure, in the order the builder lists them.
pub const ALL_MEASURES: [Measure; 24] = [
    Measure::RoomNightsSold,
    Measure::Occupancy,
    Measure::RoomRevenue,
    Measure::Adr,
    Measure::RevPar,
    Measure::GuestsStaying,
    Measure::Arrivals,
    Measure::Departures,
    Measure::BookingsMade,
    Measure::Cancellations,
    Measure::NoShows,
    Measure::AverageStay,
    Measure::AverageLeadTime,
    Measure::TotalRevenue,
    Measure::BoardRevenue,
    Measure::ExtrasRevenue,
    Measure::PaymentsTaken,
    Measure::DepositsTaken,
    Measure::SuiteHours,
    Measure::SuiteRevenue,
    Measure::Events,
    Measure::EventAttendees,
    Measure::NewGuests,
    Measure::ReturningGuests,
];

/// What a report is grouped by. `None` gives a single total.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub enum Split {
    None,
    Day,
    Week,
    Month,
    Year,
    RoomType,
    Floor,
    Room,
    Board,
    Source,
    Status,
    RevenueType,
    PaymentMethod,
    Suite,
    EventKind,
}

/// The dates a report covers: fixed, or relative to today so a saved report
/// stays current ("last month" is always last month).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub enum Period {
    /// From `start` up to (not including) `end`.
    Fixed {
        start: NaiveDate,
        end: NaiveDate,
    },
    Today,
    ThisWeek,
    ThisMonth,
    LastMonth,
    ThisYear,
    LastYear,
    /// The last twelve whole months, this one included.
    Last12Months,
    Next7Days,
    Next30Days,
    Next90Days,
}

/// How a report is drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub enum ChartKind {
    /// One big figure: the total.
    Number,
    Bar,
    /// Only for splits by day, week, month or year.
    Line,
    Table,
}

/// Everything needed to run a report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct ReportSpec {
    pub name: String,
    pub measure: Measure,
    pub split: Split,
    pub period: Period,
    pub chart: ChartKind,
}

/// A report saved under a name. It keeps the spec, not the figures, so it
/// always shows current data.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct SavedReport {
    pub id: SavedReportId,
    pub spec: ReportSpec,
}

/// Saved reports arranged on one page, in order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct Dashboard {
    pub id: DashboardId,
    pub name: String,
    pub items: Vec<DashboardItem>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct DashboardItem {
    pub report_id: SavedReportId,
    /// Takes the full width of the dashboard instead of half.
    pub wide: bool,
}

/// The most reports one dashboard can hold.
pub const DASHBOARD_MAX_ITEMS: usize = 12;

/// How a report's numbers are meant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub enum Unit {
    Count,
    /// Minor units (pence).
    Money,
    /// Per-mille: 675 is 67.5%.
    Percent,
    /// Tenths of a night: 25 is 2.5 nights.
    Nights,
    /// Tenths of a day.
    Days,
    /// Tenths of an hour.
    Hours,
}

/// One group of a report (one bar, one table row).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct ReportRow {
    /// Stable identifier for the group, e.g. "2026-06" or a room type's id.
    pub key: String,
    pub label: String,
    pub value: i64,
}

/// A report's figures.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct ReportResult {
    pub spec: ReportSpec,
    pub unit: Unit,
    /// The actual dates covered (a relative period resolved for today).
    pub period: DateRange,
    /// Empty when the split is `None`.
    pub rows: Vec<ReportRow>,
    /// The whole period. For rates and averages this is worked out over
    /// everything, not by adding the rows up.
    pub total: i64,
}

// ---------------------------------------------------------------------------
// Fact rows: what the store hands over
// ---------------------------------------------------------------------------

/// A bedroom, for room names and for counting rooms available.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoomFact {
    pub space_id: SpaceId,
    pub name: String,
    pub room_type_id: Option<RoomTypeId>,
    pub floor_id: FloorId,
    /// Out-of-service rooms aren't available to sell.
    pub in_service: bool,
}

/// One room sold for one night, priced as when booked. A hand-set price is
/// all on the first night (board and parking zero), as in the revenue report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoomNightFact {
    pub night: NaiveDate,
    pub booking_id: BookingId,
    pub space_id: SpaceId,
    pub board: Option<BoardOptionId>,
    pub source: BookingSource,
    pub status: BookingStatus,
    /// Adults + children.
    pub guests: u16,
    /// Room price with supplements.
    pub room: Money,
    pub board_price: Money,
    pub parking: Money,
}

/// One room on a booking, any status.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StayFact {
    pub booking_id: BookingId,
    pub guest_id: GuestId,
    pub space_id: SpaceId,
    pub board: Option<BoardOptionId>,
    pub source: BookingSource,
    pub status: BookingStatus,
    pub arrival: NaiveDate,
    pub departure: NaiveDate,
    /// The day the booking was made.
    pub booked_on: NaiveDate,
    /// The guest's earliest arrival on any booking that wasn't cancelled or a
    /// no-show, ever. `None` if they have none.
    pub guest_first_arrival: Option<NaiveDate>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaymentFact {
    pub date: NaiveDate,
    /// Refunds are negative.
    pub amount: Money,
    pub method: PaymentMethod,
    pub deposit: bool,
}

/// A charge added to a bill by hand (bar, restaurant, other extras).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChargeFact {
    pub date: NaiveDate,
    pub amount: Money,
}

/// A suite hire that wasn't cancelled, on the day it starts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SuiteFact {
    pub date: NaiveDate,
    pub suite_id: SpaceId,
    pub minutes: i64,
    pub price: Money,
    /// The kind of event it's for, if any.
    pub event_kind: Option<EventKind>,
}

/// An event, on the day it starts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventFact {
    pub date: NaiveDate,
    pub kind: EventKind,
    /// Its largest catering sitting, or its largest suite hire without sittings.
    pub attendees: u32,
}

/// Names for the groups, looked up by id.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ReportNames {
    pub room_types: HashMap<RoomTypeId, String>,
    /// Floor name and level (floors are listed by level).
    pub floors: HashMap<FloorId, (String, i32)>,
    pub boards: HashMap<BoardOptionId, String>,
    pub suites: HashMap<SpaceId, String>,
}

/// Everything a report needs, for the period being reported on.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ReportFacts {
    pub rooms: Vec<RoomFact>,
    pub room_nights: Vec<RoomNightFact>,
    pub stays: Vec<StayFact>,
    pub payments: Vec<PaymentFact>,
    pub charges: Vec<ChargeFact>,
    pub suites: Vec<SuiteFact>,
    pub events: Vec<EventFact>,
    pub names: ReportNames,
}

// ---------------------------------------------------------------------------
// Rules
// ---------------------------------------------------------------------------

impl Measure {
    pub const fn label(self) -> &'static str {
        match self {
            Measure::RoomNightsSold => "Room-nights sold",
            Measure::Occupancy => "Occupancy",
            Measure::RoomRevenue => "Room revenue",
            Measure::Adr => "Average daily rate (ADR)",
            Measure::RevPar => "Revenue per available room (RevPAR)",
            Measure::GuestsStaying => "Guests staying",
            Measure::Arrivals => "Arrivals",
            Measure::Departures => "Departures",
            Measure::BookingsMade => "Bookings made",
            Measure::Cancellations => "Cancellations",
            Measure::NoShows => "No-shows",
            Measure::AverageStay => "Average length of stay",
            Measure::AverageLeadTime => "Average lead time",
            Measure::TotalRevenue => "Total revenue",
            Measure::BoardRevenue => "Board revenue",
            Measure::ExtrasRevenue => "Extras revenue",
            Measure::PaymentsTaken => "Payments taken",
            Measure::DepositsTaken => "Deposits taken",
            Measure::SuiteHours => "Suite hours booked",
            Measure::SuiteRevenue => "Suite revenue",
            Measure::Events => "Events",
            Measure::EventAttendees => "Event attendees",
            Measure::NewGuests => "New guests",
            Measure::ReturningGuests => "Returning guests",
        }
    }

    pub const fn unit(self) -> Unit {
        match self {
            Measure::Occupancy => Unit::Percent,
            Measure::RoomRevenue
            | Measure::Adr
            | Measure::RevPar
            | Measure::TotalRevenue
            | Measure::BoardRevenue
            | Measure::ExtrasRevenue
            | Measure::PaymentsTaken
            | Measure::DepositsTaken
            | Measure::SuiteRevenue => Unit::Money,
            Measure::AverageStay => Unit::Nights,
            Measure::AverageLeadTime => Unit::Days,
            Measure::SuiteHours => Unit::Hours,
            _ => Unit::Count,
        }
    }
}

impl Split {
    /// Lower-case words for messages ("…can't be split by booking source").
    pub const fn words(self) -> &'static str {
        match self {
            Split::None => "nothing",
            Split::Day => "day",
            Split::Week => "week",
            Split::Month => "month",
            Split::Year => "year",
            Split::RoomType => "room type",
            Split::Floor => "floor",
            Split::Room => "room",
            Split::Board => "board",
            Split::Source => "booking source",
            Split::Status => "booking status",
            Split::RevenueType => "revenue type",
            Split::PaymentMethod => "payment method",
            Split::Suite => "suite",
            Split::EventKind => "event type",
        }
    }

    /// Day, week, month or year.
    pub const fn is_time(self) -> bool {
        matches!(self, Split::Day | Split::Week | Split::Month | Split::Year)
    }
}

/// The first of `date`'s month.
fn month_start(date: NaiveDate) -> NaiveDate {
    add_months(date, 0)
}

/// The first of the month `months` after `date`'s month (negative goes back).
fn add_months(date: NaiveDate, months: i32) -> NaiveDate {
    // Count months from year 0, so going back past January wraps the year.
    let index = date.year() * 12 + date.month0() as i32 + months;
    let month = u32::try_from(index.rem_euclid(12)).unwrap_or(0) + 1;
    NaiveDate::from_ymd_opt(index.div_euclid(12), month, 1).expect("the 1st is always a valid day")
}

fn year_start(year: i32) -> NaiveDate {
    NaiveDate::from_ymd_opt(year, 1, 1).expect("1 January is always a valid day")
}

/// The Monday on or before `date`.
fn monday_of(date: NaiveDate) -> NaiveDate {
    date - Duration::days(i64::from(date.weekday().num_days_from_monday()))
}

impl Period {
    /// The dates this period covers when the report runs on `today`.
    pub fn resolve(self, today: NaiveDate) -> DateRange {
        let days = |n: i64| today + Duration::days(n);
        let (start, end) = match self {
            Period::Fixed { start, end } => (start, end),
            Period::Today => (today, days(1)),
            Period::ThisWeek => {
                let monday = monday_of(today);
                (monday, monday + Duration::days(7))
            }
            Period::ThisMonth => (month_start(today), add_months(today, 1)),
            Period::LastMonth => (add_months(today, -1), month_start(today)),
            Period::ThisYear => (year_start(today.year()), year_start(today.year() + 1)),
            Period::LastYear => (year_start(today.year() - 1), year_start(today.year())),
            Period::Last12Months => (add_months(today, -11), add_months(today, 1)),
            Period::Next7Days => (today, days(7)),
            Period::Next30Days => (today, days(30)),
            Period::Next90Days => (today, days(90)),
        };
        DateRange { start, end }
    }
}

/// Whether `measure` can be split by `split`. Every measure can be split by
/// time (or not at all); the other splits depend on what the facts record.
pub fn allows(measure: Measure, split: Split) -> bool {
    if split == Split::None || split.is_time() {
        return true;
    }
    let by_room = matches!(split, Split::RoomType | Split::Floor | Split::Room);
    let by_booking = matches!(split, Split::Board | Split::Source);
    match measure {
        // Rooms available only know the room, not who might have booked it.
        Measure::Occupancy | Measure::RevPar => by_room,
        Measure::RoomNightsSold
        | Measure::RoomRevenue
        | Measure::Adr
        | Measure::GuestsStaying
        | Measure::BoardRevenue
        | Measure::Arrivals
        | Measure::Departures
        | Measure::BookingsMade
        | Measure::AverageStay
        | Measure::AverageLeadTime
        | Measure::NewGuests
        | Measure::ReturningGuests => by_room || by_booking || split == Split::Status,
        // All cancelled (or all no-shows): splitting by status says nothing.
        Measure::Cancellations | Measure::NoShows => by_room || by_booking,
        Measure::TotalRevenue => split == Split::RevenueType,
        Measure::ExtrasRevenue => false,
        Measure::PaymentsTaken | Measure::DepositsTaken => split == Split::PaymentMethod,
        Measure::SuiteHours | Measure::SuiteRevenue => {
            matches!(split, Split::Suite | Split::EventKind)
        }
        Measure::Events | Measure::EventAttendees => split == Split::EventKind,
    }
}

/// Longest fixed period: about ten years.
const MAX_PERIOD_DAYS: i64 = 3_660;
/// Longest period split by day.
const MAX_DAYS_SPLIT: i64 = 366;

pub fn validate_spec(spec: &ReportSpec) -> ValidationResult {
    if spec.name.trim().is_empty() {
        return Err(ValidationError::Empty {
            field: "Report name",
        });
    }
    if !allows(spec.measure, spec.split) {
        return Err(ValidationError::ReportSplitNotAllowed {
            measure: spec.measure.label().into(),
            split: spec.split.words().into(),
        });
    }
    if spec.chart == ChartKind::Line && !spec.split.is_time() {
        return Err(ValidationError::ReportLineNeedsTime);
    }
    // Relative periods are a year at most, so only fixed ones need checking.
    if let Period::Fixed { start, end } = spec.period {
        let days = (end - start).num_days();
        if days <= 0 || days > MAX_PERIOD_DAYS {
            return Err(ValidationError::InvalidReportPeriod);
        }
        if spec.split == Split::Day && days > MAX_DAYS_SPLIT {
            return Err(ValidationError::ReportTooManyDays);
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Running a report
// ---------------------------------------------------------------------------

/// The kinds of revenue in the revenue report.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RevenueType {
    Rooms,
    Board,
    Suites,
    Extras,
}

const REVENUE_TYPES: [(RevenueType, &str); 4] = [
    (RevenueType::Rooms, "Rooms"),
    (RevenueType::Board, "Board"),
    (RevenueType::Suites, "Suites"),
    (RevenueType::Extras, "Extras"),
];

const STATUSES: [(BookingStatus, &str); 6] = [
    (BookingStatus::Provisional, "Provisional"),
    (BookingStatus::Confirmed, "Confirmed"),
    (BookingStatus::CheckedIn, "Checked in"),
    (BookingStatus::CheckedOut, "Checked out"),
    (BookingStatus::Cancelled, "Cancelled"),
    (BookingStatus::NoShow, "No-show"),
];

const SOURCES: [(BookingSource, &str); 6] = [
    (BookingSource::Phone, "Phone"),
    (BookingSource::Email, "Email"),
    (BookingSource::WalkIn, "Walk-in"),
    (BookingSource::Website, "Website"),
    (BookingSource::Agent, "Agent"),
    (BookingSource::Other, "Other"),
];

const METHODS: [(PaymentMethod, &str); 6] = [
    (PaymentMethod::Cash, "Cash"),
    (PaymentMethod::Card, "Card"),
    (PaymentMethod::BankTransfer, "Bank transfer"),
    (PaymentMethod::Cheque, "Cheque"),
    (PaymentMethod::Voucher, "Voucher"),
    (PaymentMethod::Other, "Other"),
];

const EVENT_KINDS: [(EventKind, &str); 5] = [
    (EventKind::Wedding, "Wedding"),
    (EventKind::Conference, "Conference"),
    (EventKind::Dinner, "Dinner"),
    (EventKind::Party, "Party"),
    (EventKind::Other, "Other"),
];

/// A fixed list's position (for sorting) and label.
fn listed<T: PartialEq + Copy>(list: &[(T, &'static str)], value: T) -> (usize, &'static str) {
    list.iter()
        .position(|(v, _)| *v == value)
        .map_or((list.len(), "Other"), |i| (i, list[i].1))
}

/// What a fact can be grouped by. `None` means the fact doesn't know.
#[derive(Debug, Clone, Copy, Default)]
struct Dims {
    space: Option<SpaceId>,
    /// `Some(None)` is room only.
    board: Option<Option<BoardOptionId>>,
    source: Option<BookingSource>,
    status: Option<BookingStatus>,
    revenue: Option<RevenueType>,
    method: Option<PaymentMethod>,
    suite: Option<SpaceId>,
    /// `Some(None)` is a suite hire that isn't for an event.
    event_kind: Option<Option<EventKind>>,
}

/// What one fact adds to its group.
#[derive(Debug, Clone, Copy, Default)]
struct Part {
    /// Added up (a count, money, minutes, nights…).
    num: i64,
    /// The bottom of a rate or average (room-nights available, stays…).
    den: i64,
    /// For measures that count each booking or guest once.
    distinct: Option<uuid::Uuid>,
}

impl Part {
    fn add(num: i64) -> Self {
        Part {
            num,
            ..Part::default()
        }
    }
    fn of(num: i64, den: i64) -> Self {
        Part {
            num,
            den,
            distinct: None,
        }
    }
    fn once(id: uuid::Uuid) -> Self {
        Part {
            distinct: Some(id),
            ..Part::default()
        }
    }
}

#[derive(Debug, Default)]
struct Group {
    key: String,
    label: String,
    num: i64,
    den: i64,
    distinct: HashSet<uuid::Uuid>,
}

impl Group {
    fn take(&mut self, part: Part) {
        self.num += part.num;
        self.den += part.den;
        if let Some(id) = part.distinct {
            self.distinct.insert(id);
        }
    }
}

/// Where a group goes in the list, its key and its label.
struct Slot {
    sort: String,
    key: String,
    label: String,
}

impl Slot {
    fn new(sort: impl Into<String>, key: impl Into<String>, label: impl Into<String>) -> Self {
        Slot {
            sort: sort.into(),
            key: key.into(),
            label: label.into(),
        }
    }
}

fn time_slot(split: Split, date: NaiveDate) -> Slot {
    let (key, label) = match split {
        Split::Week => {
            let monday = monday_of(date);
            (
                monday.to_string(),
                format!("Week of {}", monday.format("%-d %b %Y")),
            )
        }
        Split::Month => (
            date.format("%Y-%m").to_string(),
            date.format("%B %Y").to_string(),
        ),
        Split::Year => (date.format("%Y").to_string(), date.format("%Y").to_string()),
        _ => (date.to_string(), date.format("%a %-d %b %Y").to_string()),
    };
    Slot::new(key.clone(), key, label)
}

/// `a / b` rounded to the nearest whole number (halves away from zero).
fn rounded(a: i64, b: i64) -> i64 {
    if b == 0 {
        return 0;
    }
    let half = if (a < 0) == (b < 0) {
        b.abs()
    } else {
        -b.abs()
    };
    (2 * a + half) / (2 * b)
}

struct Runner<'a> {
    split: Split,
    period: DateRange,
    facts: &'a ReportFacts,
    rooms: HashMap<SpaceId, &'a RoomFact>,
    groups: BTreeMap<String, Group>,
    total: Group,
}

impl Runner<'_> {
    fn slot(&self, date: NaiveDate, dims: &Dims) -> Option<Slot> {
        let names = &self.facts.names;
        let room = || dims.space.and_then(|id| self.rooms.get(&id).copied());
        let slot = match self.split {
            Split::None => Slot::new("", "total", "Total"),
            Split::Day | Split::Week | Split::Month | Split::Year => time_slot(self.split, date),
            Split::RoomType => match room()?.room_type_id {
                Some(id) => {
                    let name = names
                        .room_types
                        .get(&id)
                        .map_or("Unknown room type", String::as_str);
                    Slot::new(format!("0{}", name.to_lowercase()), id.to_string(), name)
                }
                None => Slot::new("1", "none", "No room type"),
            },
            Split::Floor => {
                let id = room()?.floor_id;
                let (name, level) = names
                    .floors
                    .get(&id)
                    .map_or(("Unknown floor", 0), |(n, l)| (n.as_str(), *l));
                // Levels can be negative (basements); shift them so they sort as text.
                Slot::new(
                    format!("{:06}", i64::from(level) + 100_000),
                    id.to_string(),
                    name,
                )
            }
            Split::Room => {
                let room = room()?;
                // Right-aligned, so "9" sorts before "10".
                Slot::new(
                    format!("{:>12}", room.name),
                    room.space_id.to_string(),
                    room.name.clone(),
                )
            }
            Split::Board => match dims.board? {
                None => Slot::new("0", "room_only", "Room only"),
                Some(id) => {
                    let name = names
                        .boards
                        .get(&id)
                        .map_or("Unknown board", String::as_str);
                    Slot::new(format!("1{}", name.to_lowercase()), id.to_string(), name)
                }
            },
            Split::Source => {
                let (i, label) = listed(&SOURCES, dims.source?);
                Slot::new(format!("{i:02}"), label.to_lowercase(), label)
            }
            Split::Status => {
                let (i, label) = listed(&STATUSES, dims.status?);
                Slot::new(format!("{i:02}"), label.to_lowercase(), label)
            }
            Split::RevenueType => {
                let (i, label) = listed(&REVENUE_TYPES, dims.revenue?);
                Slot::new(format!("{i:02}"), label.to_lowercase(), label)
            }
            Split::PaymentMethod => {
                let (i, label) = listed(&METHODS, dims.method?);
                Slot::new(format!("{i:02}"), label.to_lowercase(), label)
            }
            Split::Suite => {
                let id = dims.suite?;
                let name = names
                    .suites
                    .get(&id)
                    .map_or("Unknown suite", String::as_str);
                Slot::new(name.to_lowercase(), id.to_string(), name)
            }
            Split::EventKind => match dims.event_kind? {
                Some(kind) => {
                    let (i, label) = listed(&EVENT_KINDS, kind);
                    Slot::new(format!("{i:02}"), label.to_lowercase(), label)
                }
                None => Slot::new("99", "none", "Not for an event"),
            },
        };
        Some(slot)
    }

    /// Adds one fact's part, if its date is in the period.
    fn add(&mut self, date: NaiveDate, dims: Dims, part: Part) {
        if date < self.period.start || date >= self.period.end {
            return;
        }
        self.total.take(part);
        if self.split == Split::None {
            return;
        }
        // A fact whose group can't be told (a deleted room, say) still counts
        // in the total.
        if let Some(slot) = self.slot(date, &dims) {
            self.groups
                .entry(slot.sort)
                .or_insert_with(|| Group {
                    key: slot.key,
                    label: slot.label,
                    ..Group::default()
                })
                .take(part);
        }
    }

    /// Every day, week, month or year of the period gets a row, even if empty.
    fn add_empty_times(&mut self) {
        let mut day = self.period.start;
        while day < self.period.end {
            let slot = time_slot(self.split, day);
            self.groups.entry(slot.sort).or_insert_with(|| Group {
                key: slot.key,
                label: slot.label,
                ..Group::default()
            });
            day += Duration::days(1);
        }
    }

    fn feed(&mut self, measure: Measure) {
        let facts = self.facts;
        use Measure as M;
        // Rooms available: every in-service room on every day of the period.
        if matches!(measure, M::Occupancy | M::RevPar) {
            let mut day = self.period.start;
            while day < self.period.end {
                for room in facts.rooms.iter().filter(|r| r.in_service) {
                    let dims = Dims {
                        space: Some(room.space_id),
                        ..Dims::default()
                    };
                    self.add(day, dims, Part::of(0, 1));
                }
                day += Duration::days(1);
            }
        }
        for n in &facts.room_nights {
            let dims = Dims {
                space: Some(n.space_id),
                board: Some(n.board),
                source: Some(n.source),
                status: Some(n.status),
                ..Dims::default()
            };
            let revenue = |kind| Dims {
                revenue: Some(kind),
                ..dims
            };
            match measure {
                M::RoomNightsSold => self.add(n.night, dims, Part::add(1)),
                M::Occupancy => self.add(n.night, dims, Part::add(1)),
                M::RoomRevenue | M::RevPar => self.add(n.night, dims, Part::add(n.room.0)),
                M::Adr => self.add(n.night, dims, Part::of(n.room.0, 1)),
                M::GuestsStaying => self.add(n.night, dims, Part::add(i64::from(n.guests))),
                M::BoardRevenue => self.add(n.night, dims, Part::add(n.board_price.0)),
                M::ExtrasRevenue => self.add(n.night, dims, Part::add(n.parking.0)),
                M::TotalRevenue => {
                    self.add(n.night, revenue(RevenueType::Rooms), Part::add(n.room.0));
                    self.add(
                        n.night,
                        revenue(RevenueType::Board),
                        Part::add(n.board_price.0),
                    );
                    self.add(
                        n.night,
                        revenue(RevenueType::Extras),
                        Part::add(n.parking.0),
                    );
                }
                _ => {}
            }
        }
        for s in &facts.stays {
            let dims = Dims {
                space: Some(s.space_id),
                board: Some(s.board),
                source: Some(s.source),
                status: Some(s.status),
                ..Dims::default()
            };
            let live = s.status.holds_rooms();
            match measure {
                M::Arrivals if live => self.add(s.arrival, dims, Part::add(1)),
                M::Departures if live => self.add(s.departure, dims, Part::add(1)),
                M::BookingsMade => self.add(s.booked_on, dims, Part::once(s.booking_id.0)),
                M::Cancellations if s.status == BookingStatus::Cancelled => {
                    self.add(s.arrival, dims, Part::once(s.booking_id.0));
                }
                M::NoShows if s.status == BookingStatus::NoShow => {
                    self.add(s.arrival, dims, Part::once(s.booking_id.0));
                }
                M::AverageStay if live => {
                    let nights = (s.departure - s.arrival).num_days();
                    self.add(s.arrival, dims, Part::of(nights, 1));
                }
                M::AverageLeadTime if live => {
                    let ahead = (s.arrival - s.booked_on).num_days().max(0);
                    self.add(s.arrival, dims, Part::of(ahead, 1));
                }
                M::NewGuests if live && s.guest_first_arrival == Some(s.arrival) => {
                    self.add(s.arrival, dims, Part::once(s.guest_id.0));
                }
                M::ReturningGuests
                    if live && s.guest_first_arrival.is_some_and(|first| first < s.arrival) =>
                {
                    self.add(s.arrival, dims, Part::once(s.guest_id.0));
                }
                _ => {}
            }
        }
        for p in &facts.payments {
            let dims = Dims {
                method: Some(p.method),
                ..Dims::default()
            };
            match measure {
                M::PaymentsTaken => self.add(p.date, dims, Part::add(p.amount.0)),
                M::DepositsTaken if p.deposit => self.add(p.date, dims, Part::add(p.amount.0)),
                _ => {}
            }
        }
        for c in &facts.charges {
            let dims = Dims {
                revenue: Some(RevenueType::Extras),
                ..Dims::default()
            };
            if matches!(measure, M::TotalRevenue | M::ExtrasRevenue) {
                self.add(c.date, dims, Part::add(c.amount.0));
            }
        }
        for h in &facts.suites {
            let dims = Dims {
                suite: Some(h.suite_id),
                event_kind: Some(h.event_kind),
                revenue: Some(RevenueType::Suites),
                ..Dims::default()
            };
            match measure {
                M::SuiteHours => self.add(h.date, dims, Part::add(h.minutes)),
                M::SuiteRevenue | M::TotalRevenue => self.add(h.date, dims, Part::add(h.price.0)),
                _ => {}
            }
        }
        for e in &facts.events {
            let dims = Dims {
                event_kind: Some(Some(e.kind)),
                ..Dims::default()
            };
            match measure {
                M::Events => self.add(e.date, dims, Part::add(1)),
                M::EventAttendees => self.add(e.date, dims, Part::add(i64::from(e.attendees))),
                _ => {}
            }
        }
    }
}

/// A group's figure for `measure`.
fn value(measure: Measure, g: &Group) -> i64 {
    use Measure as M;
    match measure {
        M::Occupancy => rounded(g.num * 1000, g.den),
        M::Adr | M::RevPar => rounded(g.num, g.den),
        M::AverageStay | M::AverageLeadTime => rounded(g.num * 10, g.den),
        // Minutes to tenths of an hour.
        M::SuiteHours => rounded(g.num * 10, 60),
        M::BookingsMade | M::Cancellations | M::NoShows | M::NewGuests | M::ReturningGuests => {
            i64::try_from(g.distinct.len()).unwrap_or(i64::MAX)
        }
        _ => g.num,
    }
}

pub fn validate_dashboard(dashboard: &Dashboard) -> ValidationResult {
    if dashboard.name.trim().is_empty() {
        return Err(ValidationError::Empty {
            field: "Dashboard name",
        });
    }
    if dashboard.items.len() > DASHBOARD_MAX_ITEMS {
        return Err(ValidationError::DashboardTooFull {
            max: DASHBOARD_MAX_ITEMS,
        });
    }
    // `insert` returns false when the id was already there.
    let mut seen = HashSet::new();
    if !dashboard.items.iter().all(|i| seen.insert(i.report_id)) {
        return Err(ValidationError::DashboardReportTwice);
    }
    Ok(())
}

/// A value as plain text: money in pounds and pence, tenths with one decimal.
pub fn format_value(unit: Unit, value: i64) -> String {
    let sign = if value < 0 { "-" } else { "" };
    let v = value.abs();
    match unit {
        Unit::Count => value.to_string(),
        Unit::Money => format!("{sign}{}.{:02}", v / 100, v % 100),
        Unit::Percent | Unit::Nights | Unit::Days | Unit::Hours => {
            format!("{sign}{}.{}", v / 10, v % 10)
        }
    }
}

/// A report as CSV text, for spreadsheets.
pub fn report_csv(result: &ReportResult) -> String {
    let group = match result.spec.split {
        Split::None => "Period".to_string(),
        split => {
            let words = split.words();
            words[..1].to_uppercase() + &words[1..]
        }
    };
    let suffix = match result.unit {
        Unit::Percent => " (%)",
        Unit::Nights => " (nights)",
        Unit::Days => " (days)",
        Unit::Hours => " (hours)",
        Unit::Count | Unit::Money => "",
    };
    let heading = format!("{}{suffix}", result.spec.measure.label());
    let rows: Vec<Vec<String>> = result
        .rows
        .iter()
        .map(|r| vec![r.label.clone(), format_value(result.unit, r.value)])
        .chain(std::iter::once(vec![
            "Total".to_string(),
            format_value(result.unit, result.total),
        ]))
        .collect();
    crate::operations::to_csv(&[group.as_str(), heading.as_str()], &rows)
}

/// Every split, in the order the builder lists them.
pub const ALL_SPLITS: [Split; 15] = [
    Split::None,
    Split::Day,
    Split::Week,
    Split::Month,
    Split::Year,
    Split::RoomType,
    Split::Floor,
    Split::Room,
    Split::Board,
    Split::Source,
    Split::Status,
    Split::RevenueType,
    Split::PaymentMethod,
    Split::Suite,
    Split::EventKind,
];

/// A measure as the builder offers it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct MeasureInfo {
    pub measure: Measure,
    pub label: String,
    /// "Rooms", "Bookings", "Money", "Suites and events" or "Guests".
    pub area: String,
    pub unit: Unit,
    /// The splits it can use, in list order.
    pub splits: Vec<Split>,
}

/// What the builder can offer: every measure, and how each can be split.
pub fn catalogue() -> Vec<MeasureInfo> {
    ALL_MEASURES
        .iter()
        .map(|&measure| {
            use Measure as M;
            let area = match measure {
                M::RoomNightsSold
                | M::Occupancy
                | M::RoomRevenue
                | M::Adr
                | M::RevPar
                | M::GuestsStaying => "Rooms",
                M::Arrivals
                | M::Departures
                | M::BookingsMade
                | M::Cancellations
                | M::NoShows
                | M::AverageStay
                | M::AverageLeadTime => "Bookings",
                M::TotalRevenue
                | M::BoardRevenue
                | M::ExtrasRevenue
                | M::PaymentsTaken
                | M::DepositsTaken => "Money",
                M::SuiteHours | M::SuiteRevenue | M::Events | M::EventAttendees => {
                    "Suites and events"
                }
                M::NewGuests | M::ReturningGuests => "Guests",
            };
            MeasureInfo {
                measure,
                label: measure.label().into(),
                area: area.into(),
                unit: measure.unit(),
                splits: ALL_SPLITS
                    .into_iter()
                    .filter(|&s| allows(measure, s))
                    .collect(),
            }
        })
        .collect()
}

/// Runs a report on `today` over facts loaded for its period. Facts outside
/// the period are left out, so loading a little extra does no harm.
pub fn run(spec: &ReportSpec, facts: &ReportFacts, today: NaiveDate) -> ReportResult {
    let period = spec.period.resolve(today);
    let mut runner = Runner {
        split: spec.split,
        period,
        facts,
        rooms: facts.rooms.iter().map(|r| (r.space_id, r)).collect(),
        groups: BTreeMap::new(),
        total: Group::default(),
    };
    if spec.split.is_time() {
        runner.add_empty_times();
    }
    runner.feed(spec.measure);
    let rows = runner
        .groups
        .values()
        .map(|g| ReportRow {
            key: g.key.clone(),
            label: g.label.clone(),
            value: value(spec.measure, g),
        })
        .collect();
    ReportResult {
        spec: spec.clone(),
        unit: spec.measure.unit(),
        period,
        rows,
        total: value(spec.measure, &runner.total),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(s: &str) -> NaiveDate {
        s.parse().unwrap()
    }

    fn spec(measure: Measure, split: Split, period: Period) -> ReportSpec {
        ReportSpec {
            name: "Test".into(),
            measure,
            split,
            period,
            chart: ChartKind::Bar,
        }
    }

    fn fixed(start: &str, end: &str) -> Period {
        Period::Fixed {
            start: d(start),
            end: d(end),
        }
    }

    /// Two bedrooms on the first floor: 101 (Double) and 102 (Twin).
    struct Hotel {
        facts: ReportFacts,
        r101: SpaceId,
        r102: SpaceId,
        double: RoomTypeId,
        twin: RoomTypeId,
    }

    fn hotel() -> Hotel {
        let (double, twin, first) = (RoomTypeId::new(), RoomTypeId::new(), FloorId::new());
        let (r101, r102) = (SpaceId::new(), SpaceId::new());
        let room = |space_id, name: &str, room_type_id| RoomFact {
            space_id,
            name: name.into(),
            room_type_id: Some(room_type_id),
            floor_id: first,
            in_service: true,
        };
        let mut names = ReportNames::default();
        names.room_types.insert(double, "Double".into());
        names.room_types.insert(twin, "Twin".into());
        names.floors.insert(first, ("First floor".into(), 1));
        Hotel {
            facts: ReportFacts {
                rooms: vec![room(r101, "101", double), room(r102, "102", twin)],
                names,
                ..ReportFacts::default()
            },
            r101,
            r102,
            double,
            twin,
        }
    }

    fn night(space_id: SpaceId, day: &str, room: i64) -> RoomNightFact {
        RoomNightFact {
            night: d(day),
            booking_id: BookingId::new(),
            space_id,
            board: None,
            source: BookingSource::Phone,
            status: BookingStatus::Confirmed,
            guests: 2,
            room: Money(room),
            board_price: Money::ZERO,
            parking: Money::ZERO,
        }
    }

    fn stay(space_id: SpaceId, arrival: &str, departure: &str, status: BookingStatus) -> StayFact {
        StayFact {
            booking_id: BookingId::new(),
            guest_id: GuestId::new(),
            space_id,
            board: None,
            source: BookingSource::Phone,
            status,
            arrival: d(arrival),
            departure: d(departure),
            booked_on: d("2026-05-01"),
            guest_first_arrival: Some(d(arrival)),
        }
    }

    fn values(r: &ReportResult) -> Vec<(&str, i64)> {
        r.rows
            .iter()
            .map(|row| (row.label.as_str(), row.value))
            .collect()
    }

    // -- periods and rules --------------------------------------------------

    #[test]
    fn relative_periods_resolve_for_today() {
        let wed = d("2026-06-17");
        let range = |p: Period| {
            let r = p.resolve(wed);
            (r.start.to_string(), r.end.to_string())
        };
        let pair = |a: &str, b: &str| (a.to_string(), b.to_string());
        assert_eq!(range(Period::Today), pair("2026-06-17", "2026-06-18"));
        assert_eq!(range(Period::ThisWeek), pair("2026-06-15", "2026-06-22"));
        assert_eq!(range(Period::ThisMonth), pair("2026-06-01", "2026-07-01"));
        assert_eq!(range(Period::LastMonth), pair("2026-05-01", "2026-06-01"));
        assert_eq!(range(Period::ThisYear), pair("2026-01-01", "2027-01-01"));
        assert_eq!(range(Period::LastYear), pair("2025-01-01", "2026-01-01"));
        assert_eq!(
            range(Period::Last12Months),
            pair("2025-07-01", "2026-07-01")
        );
        assert_eq!(range(Period::Next7Days), pair("2026-06-17", "2026-06-24"));
        assert_eq!(range(Period::Next30Days), pair("2026-06-17", "2026-07-17"));
        assert_eq!(range(Period::Next90Days), pair("2026-06-17", "2026-09-15"));
        assert_eq!(
            range(fixed("2026-03-01", "2026-03-08")),
            pair("2026-03-01", "2026-03-08")
        );
        // Last month from January is December of the year before.
        let jan = Period::LastMonth.resolve(d("2026-01-10"));
        assert_eq!((jan.start, jan.end), (d("2025-12-01"), d("2026-01-01")));
    }

    #[test]
    fn only_splits_that_make_sense_are_allowed() {
        assert!(allows(Measure::Occupancy, Split::RoomType));
        assert!(!allows(Measure::Occupancy, Split::Source));
        assert!(allows(Measure::PaymentsTaken, Split::PaymentMethod));
        assert!(!allows(Measure::PaymentsTaken, Split::Board));
        assert!(allows(Measure::TotalRevenue, Split::RevenueType));
        assert!(!allows(Measure::RoomRevenue, Split::RevenueType));
        assert!(allows(Measure::SuiteHours, Split::EventKind));
        assert!(!allows(Measure::Cancellations, Split::Status));
        for m in ALL_MEASURES {
            for s in [
                Split::None,
                Split::Day,
                Split::Week,
                Split::Month,
                Split::Year,
            ] {
                assert!(allows(m, s), "{m:?} by {s:?}");
            }
        }
    }

    #[test]
    fn report_specs_are_checked() {
        let ok = spec(Measure::Arrivals, Split::Month, Period::ThisYear);
        assert_eq!(validate_spec(&ok), Ok(()));
        assert_eq!(
            validate_spec(&ReportSpec {
                name: " ".into(),
                ..ok.clone()
            }),
            Err(ValidationError::Empty {
                field: "Report name"
            })
        );
        assert_eq!(
            validate_spec(&spec(Measure::Occupancy, Split::Source, Period::ThisMonth)),
            Err(ValidationError::ReportSplitNotAllowed {
                measure: "Occupancy".into(),
                split: "booking source".into()
            })
        );
        assert_eq!(
            validate_spec(&ReportSpec {
                chart: ChartKind::Line,
                ..spec(Measure::RoomRevenue, Split::RoomType, Period::ThisMonth)
            }),
            Err(ValidationError::ReportLineNeedsTime)
        );
        assert_eq!(
            validate_spec(&spec(
                Measure::Arrivals,
                Split::Month,
                fixed("2026-06-01", "2026-05-01")
            )),
            Err(ValidationError::InvalidReportPeriod)
        );
        assert_eq!(
            validate_spec(&spec(
                Measure::Arrivals,
                Split::Day,
                fixed("2026-01-01", "2027-06-01")
            )),
            Err(ValidationError::ReportTooManyDays)
        );
    }

    // -- rooms --------------------------------------------------------------

    #[test]
    fn occupancy_adr_and_revpar_are_worked_out_per_group_and_overall() {
        let mut h = hotel();
        h.facts.room_nights = vec![
            night(h.r101, "2026-06-01", 10_000),
            night(h.r101, "2026-06-02", 10_000),
            night(h.r102, "2026-07-01", 8_000),
        ];
        let period = fixed("2026-06-01", "2026-08-01");
        let today = d("2026-08-15");

        let occ = run(
            &spec(Measure::Occupancy, Split::Month, period),
            &h.facts,
            today,
        );
        assert_eq!(occ.unit, Unit::Percent);
        // June: 2 of 60 room-nights; July: 1 of 62; overall 3 of 122.
        assert_eq!(values(&occ), vec![("June 2026", 33), ("July 2026", 16)]);
        assert_eq!(occ.total, 25);

        let adr = run(&spec(Measure::Adr, Split::Month, period), &h.facts, today);
        assert_eq!(adr.unit, Unit::Money);
        assert_eq!(
            values(&adr),
            vec![("June 2026", 10_000), ("July 2026", 8_000)]
        );
        assert_eq!(adr.total, 9_333); // 28,000 / 3

        let revpar = run(
            &spec(Measure::RevPar, Split::Month, period),
            &h.facts,
            today,
        );
        assert_eq!(
            values(&revpar),
            vec![("June 2026", 333), ("July 2026", 129)]
        );
    }

    #[test]
    fn time_splits_show_empty_periods_as_zero() {
        let mut h = hotel();
        h.facts.room_nights = vec![night(h.r101, "2026-06-03", 9_000)];
        let r = run(
            &spec(
                Measure::RoomNightsSold,
                Split::Month,
                fixed("2026-05-01", "2026-08-01"),
            ),
            &h.facts,
            d("2026-09-01"),
        );
        assert_eq!(
            values(&r),
            vec![("May 2026", 0), ("June 2026", 1), ("July 2026", 0)]
        );
        assert_eq!(r.rows[1].key, "2026-06");
    }

    #[test]
    fn weeks_start_on_monday() {
        let mut h = hotel();
        h.facts.room_nights = vec![night(h.r101, "2026-06-17", 9_000)];
        let r = run(
            &spec(
                Measure::RoomNightsSold,
                Split::Week,
                fixed("2026-06-15", "2026-06-22"),
            ),
            &h.facts,
            d("2026-07-01"),
        );
        assert_eq!(values(&r), vec![("Week of 15 Jun 2026", 1)]);
        assert_eq!(r.rows[0].key, "2026-06-15");
    }

    #[test]
    fn room_revenue_by_room_type() {
        let mut h = hotel();
        h.facts.room_nights = vec![
            night(h.r102, "2026-06-01", 8_000),
            night(h.r101, "2026-06-01", 10_000),
            night(h.r101, "2026-06-02", 10_000),
        ];
        let r = run(
            &spec(Measure::RoomRevenue, Split::RoomType, Period::ThisMonth),
            &h.facts,
            d("2026-06-20"),
        );
        assert_eq!(values(&r), vec![("Double", 20_000), ("Twin", 8_000)]);
        assert_eq!(r.rows[0].key, h.double.to_string());
        assert_eq!(r.rows[1].key, h.twin.to_string());
        assert_eq!(r.total, 28_000);
    }

    #[test]
    fn facts_outside_the_period_are_left_out() {
        let mut h = hotel();
        h.facts.room_nights = vec![
            night(h.r101, "2026-05-31", 10_000),
            night(h.r101, "2026-06-01", 10_000),
            night(h.r101, "2026-07-01", 10_000),
        ];
        let r = run(
            &spec(Measure::RoomRevenue, Split::None, Period::ThisMonth),
            &h.facts,
            d("2026-06-20"),
        );
        assert!(r.rows.is_empty());
        assert_eq!(r.total, 10_000);
        assert_eq!(
            r.period,
            DateRange {
                start: d("2026-06-01"),
                end: d("2026-07-01")
            }
        );
    }

    // -- bookings -----------------------------------------------------------

    #[test]
    fn arrivals_count_rooms_and_cancellations_count_bookings() {
        let h = hotel();
        let a1 = stay(h.r101, "2026-06-05", "2026-06-07", BookingStatus::Confirmed);
        let a2 = StayFact {
            space_id: h.r102,
            ..a1.clone()
        };
        let c1 = stay(h.r101, "2026-06-06", "2026-06-08", BookingStatus::Cancelled);
        let c2 = StayFact {
            space_id: h.r102,
            ..c1.clone()
        };
        let no_show = stay(h.r101, "2026-06-09", "2026-06-10", BookingStatus::NoShow);
        let mut facts = h.facts;
        facts.stays = vec![a1, a2, c1, c2, no_show];
        let total = |m| {
            run(
                &spec(m, Split::None, Period::ThisMonth),
                &facts,
                d("2026-06-20"),
            )
            .total
        };
        assert_eq!(total(Measure::Arrivals), 2);
        assert_eq!(total(Measure::Departures), 2);
        assert_eq!(total(Measure::Cancellations), 1);
        assert_eq!(total(Measure::NoShows), 1);
        assert_eq!(total(Measure::BookingsMade), 0); // made in May
    }

    #[test]
    fn bookings_made_go_by_the_day_they_were_made() {
        let h = hotel();
        let mut made = stay(h.r101, "2026-09-01", "2026-09-03", BookingStatus::Cancelled);
        made.booked_on = d("2026-06-10");
        let facts = ReportFacts {
            stays: vec![made],
            ..h.facts
        };
        let r = run(
            &spec(Measure::BookingsMade, Split::Source, Period::ThisMonth),
            &facts,
            d("2026-06-20"),
        );
        assert_eq!(values(&r), vec![("Phone", 1)]);
    }

    #[test]
    fn average_stay_and_lead_time_are_in_tenths() {
        let h = hotel();
        let mut two = stay(h.r101, "2026-06-10", "2026-06-12", BookingStatus::Confirmed);
        two.booked_on = d("2026-05-31"); // 10 days ahead
        let mut three = stay(
            h.r102,
            "2026-06-11",
            "2026-06-14",
            BookingStatus::CheckedOut,
        );
        three.booked_on = d("2026-06-06"); // 5 days ahead
        let cancelled = stay(h.r101, "2026-06-20", "2026-06-30", BookingStatus::Cancelled);
        let facts = ReportFacts {
            stays: vec![two, three, cancelled],
            ..h.facts
        };
        let run_total = |m| {
            run(
                &spec(m, Split::None, Period::ThisMonth),
                &facts,
                d("2026-06-20"),
            )
        };
        let alos = run_total(Measure::AverageStay);
        assert_eq!((alos.unit, alos.total), (Unit::Nights, 25));
        let lead = run_total(Measure::AverageLeadTime);
        assert_eq!((lead.unit, lead.total), (Unit::Days, 75));
    }

    #[test]
    fn new_and_returning_guests() {
        let h = hotel();
        let first_time = stay(h.r101, "2026-06-10", "2026-06-12", BookingStatus::Confirmed);
        let mut back_again = stay(h.r102, "2026-06-11", "2026-06-12", BookingStatus::Confirmed);
        back_again.guest_first_arrival = Some(d("2025-08-01"));
        let facts = ReportFacts {
            stays: vec![first_time, back_again],
            ..h.facts
        };
        let total = |m| {
            run(
                &spec(m, Split::None, Period::ThisMonth),
                &facts,
                d("2026-06-20"),
            )
            .total
        };
        assert_eq!(total(Measure::NewGuests), 1);
        assert_eq!(total(Measure::ReturningGuests), 1);
    }

    // -- money --------------------------------------------------------------

    #[test]
    fn total_revenue_by_type_counts_parking_and_charges_as_extras() {
        let mut h = hotel();
        let mut n = night(h.r101, "2026-06-01", 10_000);
        n.board_price = Money(2_000);
        n.parking = Money(500);
        h.facts.room_nights = vec![n];
        h.facts.suites = vec![SuiteFact {
            date: d("2026-06-05"),
            suite_id: SpaceId::new(),
            minutes: 240,
            price: Money(30_000),
            event_kind: None,
        }];
        h.facts.charges = vec![ChargeFact {
            date: d("2026-06-02"),
            amount: Money(1_500),
        }];
        let today = d("2026-06-20");
        let r = run(
            &spec(Measure::TotalRevenue, Split::RevenueType, Period::ThisMonth),
            &h.facts,
            today,
        );
        assert_eq!(
            values(&r),
            vec![
                ("Rooms", 10_000),
                ("Board", 2_000),
                ("Suites", 30_000),
                ("Extras", 2_000)
            ]
        );
        assert_eq!(r.total, 44_000);
        let extras = run(
            &spec(Measure::ExtrasRevenue, Split::None, Period::ThisMonth),
            &h.facts,
            today,
        );
        assert_eq!(extras.total, 2_000);
        let board = run(
            &spec(Measure::BoardRevenue, Split::None, Period::ThisMonth),
            &h.facts,
            today,
        );
        assert_eq!(board.total, 2_000);
    }

    #[test]
    fn payments_by_method_and_deposits() {
        let h = hotel();
        let pay = |day: &str, amount, method, deposit| PaymentFact {
            date: d(day),
            amount: Money(amount),
            method,
            deposit,
        };
        let facts = ReportFacts {
            payments: vec![
                pay("2026-06-01", 5_000, PaymentMethod::Card, true),
                pay("2026-06-03", 12_000, PaymentMethod::Card, false),
                pay("2026-06-04", 3_000, PaymentMethod::Cash, false),
            ],
            ..h.facts
        };
        let today = d("2026-06-20");
        let r = run(
            &spec(
                Measure::PaymentsTaken,
                Split::PaymentMethod,
                Period::ThisMonth,
            ),
            &facts,
            today,
        );
        assert_eq!(values(&r), vec![("Cash", 3_000), ("Card", 17_000)]);
        let deposits = run(
            &spec(Measure::DepositsTaken, Split::None, Period::ThisMonth),
            &facts,
            today,
        );
        assert_eq!(deposits.total, 5_000);
    }

    // -- suites and events --------------------------------------------------

    #[test]
    fn suite_hours_in_tenths_by_event_kind() {
        let h = hotel();
        let hire = |minutes, event_kind| SuiteFact {
            date: d("2026-06-05"),
            suite_id: SpaceId::new(),
            minutes,
            price: Money(10_000),
            event_kind,
        };
        let facts = ReportFacts {
            suites: vec![hire(240, Some(EventKind::Wedding)), hire(90, None)],
            ..h.facts
        };
        let r = run(
            &spec(Measure::SuiteHours, Split::EventKind, Period::ThisMonth),
            &facts,
            d("2026-06-20"),
        );
        assert_eq!(r.unit, Unit::Hours);
        assert_eq!(values(&r), vec![("Wedding", 40), ("Not for an event", 15)]);
        assert_eq!(r.total, 55);
    }

    #[test]
    fn events_and_attendees_by_kind() {
        let h = hotel();
        let event = |kind, attendees| EventFact {
            date: d("2026-06-12"),
            kind,
            attendees,
        };
        let facts = ReportFacts {
            events: vec![
                event(EventKind::Wedding, 120),
                event(EventKind::Conference, 40),
                event(EventKind::Wedding, 80),
            ],
            ..h.facts
        };
        let today = d("2026-06-20");
        let count = run(
            &spec(Measure::Events, Split::EventKind, Period::ThisMonth),
            &facts,
            today,
        );
        assert_eq!(values(&count), vec![("Wedding", 2), ("Conference", 1)]);
        let people = run(
            &spec(Measure::EventAttendees, Split::None, Period::ThisMonth),
            &facts,
            today,
        );
        assert_eq!(people.total, 240);
    }

    #[test]
    fn dashboards_need_a_name_and_at_most_twelve_different_reports() {
        let item = |id| DashboardItem {
            report_id: id,
            wide: false,
        };
        let one = SavedReportId::new();
        let mut dash = Dashboard {
            id: DashboardId::new(),
            name: "Monday meeting".into(),
            items: vec![item(one)],
        };
        assert_eq!(validate_dashboard(&dash), Ok(()));
        dash.items.push(item(one));
        assert_eq!(
            validate_dashboard(&dash),
            Err(ValidationError::DashboardReportTwice)
        );
        dash.items = (0..=DASHBOARD_MAX_ITEMS)
            .map(|_| item(SavedReportId::new()))
            .collect();
        assert_eq!(
            validate_dashboard(&dash),
            Err(ValidationError::DashboardTooFull {
                max: DASHBOARD_MAX_ITEMS
            })
        );
        dash.items.clear();
        dash.name = String::new();
        assert_eq!(
            validate_dashboard(&dash),
            Err(ValidationError::Empty {
                field: "Dashboard name"
            })
        );
    }

    #[test]
    fn values_are_written_out_for_people_and_spreadsheets() {
        assert_eq!(format_value(Unit::Money, 123_456), "1234.56");
        assert_eq!(format_value(Unit::Money, -250), "-2.50");
        assert_eq!(format_value(Unit::Percent, 675), "67.5");
        assert_eq!(format_value(Unit::Nights, 25), "2.5");
        assert_eq!(format_value(Unit::Count, 12), "12");
    }

    #[test]
    fn a_report_as_csv_has_its_rows_and_a_total() {
        let mut h = hotel();
        h.facts.room_nights = vec![
            night(h.r101, "2026-06-01", 10_000),
            night(h.r102, "2026-06-01", 8_000),
        ];
        let r = run(
            &spec(Measure::RoomRevenue, Split::RoomType, Period::ThisMonth),
            &h.facts,
            d("2026-06-20"),
        );
        assert_eq!(
            report_csv(&r),
            "Room type,Room revenue\r\nDouble,100.00\r\nTwin,80.00\r\nTotal,180.00\r\n"
        );
    }

    #[test]
    fn the_catalogue_lists_every_measure_with_its_allowed_splits() {
        let list = catalogue();
        assert_eq!(list.len(), ALL_MEASURES.len());
        for info in &list {
            for split in ALL_SPLITS {
                assert_eq!(
                    info.splits.contains(&split),
                    allows(info.measure, split),
                    "{info:?}"
                );
            }
            assert_eq!(info.unit, info.measure.unit());
        }
        assert_eq!(list[0].area, "Rooms");
        assert_eq!(list[0].label, "Room-nights sold");
    }

    #[test]
    fn room_names_and_floors_label_their_groups() {
        let mut h = hotel();
        h.facts.room_nights = vec![
            night(h.r102, "2026-06-01", 8_000),
            night(h.r101, "2026-06-01", 10_000),
        ];
        let today = d("2026-06-20");
        let by_room = run(
            &spec(Measure::RoomNightsSold, Split::Room, Period::ThisMonth),
            &h.facts,
            today,
        );
        assert_eq!(values(&by_room), vec![("101", 1), ("102", 1)]);
        let by_floor = run(
            &spec(Measure::RoomNightsSold, Split::Floor, Period::ThisMonth),
            &h.facts,
            today,
        );
        assert_eq!(values(&by_floor), vec![("First floor", 2)]);
        let by_board = run(
            &spec(Measure::RoomNightsSold, Split::Board, Period::ThisMonth),
            &h.facts,
            today,
        );
        assert_eq!(values(&by_board), vec![("Room only", 2)]);
    }
}
