//! The demo hotel: **The Lanterns Hotel, Kelmouth**, an invented seaside
//! hotel in an invented town.
//!
//! [`demo_hotel`] builds the whole hotel (building, prices, guests, about
//! two years of bookings, events, bills and today's housekeeping) from a
//! seed and today's date. It is pure: the same seed and date always give the
//! same hotel, IDs included, and every date is worked out from `today`, so the
//! demo always looks current. The store writes the result to the database;
//! nothing here touches I/O.
//!
//! All people, emails and phone numbers are made up: emails use the reserved
//! `example.com/.org/.net` domains and phone numbers come from Ofcom's ranges
//! kept for TV and drama (07700 900xxx, 01632 960xxx), so nothing reaches a
//! real person.

use std::collections::HashSet;

use chrono::{Datelike, Days, NaiveDate, NaiveDateTime, NaiveTime, Weekday};
use serde::Serialize;
use uuid::Uuid;

use crate::{
    Adjustment, BoardOption, BoardOptionId, Booking, BookingId, BookingRoom, BookingRoomId,
    BookingSource, BookingStatus, CateringSitting, Charge, ChargeCategory, ChargeId, DateRange,
    Event, EventId, EventKind, Floor, FloorId, GridRect, Guest, GuestId, HousekeepingStatus,
    InvoiceSettings, Layout, Money, Payment, PaymentId, PaymentMethod, PropertySettings, RateRule,
    RateRuleId, RoomBlock, RoomFeature, RoomType, RoomTypeId, Space, SpaceId, SpaceKind,
    SuiteBooking, SuiteBookingId, SuiteBookingStatus, SuiteCapacities, SuitePricing, SuiteRates,
    TaxRate, TaxRateId,
};
use crate::{
    ChartKind, Dashboard, DashboardId, DashboardItem, Measure, Period, ReportSpec, SavedReport,
    SavedReportId, Split,
};

/// The seed the app uses ("LANTERNS" in ASCII).
pub const DEMO_SEED: u64 = 0x4C41_4E54_4552_4E53;
/// How far back the bookings go: about 18 months.
pub const HISTORY_DAYS: u64 = 548;
/// How far ahead: about 6 months.
pub const FUTURE_DAYS: u64 = 183;
/// Parking, per car per night.
pub const PARKING_PRICE: Money = Money(800);
/// UK standard VAT, in basis points.
const VAT: u32 = 2000;

/// Everything the demo hotel is made of, ready for the store to save in this
/// order: settings and building, then stays, then events.
#[derive(Debug, Clone, Serialize)]
pub struct DemoHotel {
    pub settings: PropertySettings,
    pub room_types: Vec<RoomType>,
    pub floors: Vec<Floor>,
    pub spaces: Vec<Space>,
    pub boards: Vec<BoardOption>,
    pub rate_rules: Vec<RateRule>,
    pub suite_rates: Vec<(SpaceId, SuiteRates)>,
    pub tax_rates: Vec<TaxRate>,
    pub invoice_settings: InvoiceSettings,
    /// Everyone who has stayed or will stay (event organisers are on their event).
    pub guests: Vec<Guest>,
    /// Bedroom bookings, in order of arrival.
    pub stays: Vec<DemoStay>,
    pub events: Vec<DemoEvent>,
    /// Rooms housekeeping has already marked this morning. Rooms slept in
    /// last night and not listed here show as dirty.
    pub housekeeping: Vec<(SpaceId, HousekeepingStatus)>,
    /// A few reports built with the report builder, and a dashboard of them.
    pub saved_reports: Vec<SavedReport>,
    pub dashboards: Vec<Dashboard>,
}

/// Money that goes with a booking or event: extras on the bill, a deposit,
/// paying what's left, and an invoice.
#[derive(Debug, Clone, Default, Serialize)]
pub struct DemoAccount {
    pub extras: Vec<Charge>,
    pub deposit: Option<Payment>,
    /// Pay whatever is still owed, on this date and by this method. The
    /// store works out the amount, because the price comes from the rates.
    pub settle: Option<(NaiveDate, PaymentMethod)>,
    /// Issue an invoice on this date (after settling).
    pub invoice_on: Option<NaiveDate>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DemoStay {
    /// The booking's guest is in [`DemoHotel::guests`].
    pub booking: Booking,
    /// The day it was made, for lead time and "bookings made".
    pub booked_on: NaiveDate,
    pub account: DemoAccount,
}

#[derive(Debug, Clone, Serialize)]
pub struct DemoEvent {
    pub event: Event,
    pub organiser: Guest,
    pub suite_bookings: Vec<SuiteBooking>,
    pub account: DemoAccount,
}

/// A small, fast, deterministic random number generator (SplitMix64).
/// We write our own rather than use the `rand` crate because `rand` may
/// change its output between versions, and the demo must never change.
#[derive(Debug, Clone)]
pub struct DemoRng(u64);

impl DemoRng {
    pub const fn new(seed: u64) -> Self {
        Self(seed)
    }

    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// A number in `0..n` (0 when n is 0).
    pub fn below(&mut self, n: u64) -> u64 {
        if n == 0 { 0 } else { self.next_u64() % n }
    }

    /// A number in `lo..=hi`.
    pub fn between(&mut self, lo: u64, hi: u64) -> u64 {
        lo + self.below(hi.saturating_sub(lo) + 1)
    }

    /// True `percent` times in a hundred.
    pub fn chance(&mut self, percent: u64) -> bool {
        self.below(100) < percent
    }

    pub fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        &items[self.below(items.len() as u64) as usize]
    }

    /// Picks a value, each with its own weight.
    pub fn weighted<T: Copy>(&mut self, items: &[(T, u32)]) -> T {
        let total: u64 = items.iter().map(|(_, w)| u64::from(*w)).sum();
        let mut n = self.below(total);
        for (item, weight) in items {
            if n < u64::from(*weight) {
                return *item;
            }
            n -= u64::from(*weight);
        }
        items[items.len() - 1].0
    }

    /// A random (v4) UUID drawn from this generator, so IDs repeat too.
    pub fn uuid(&mut self) -> Uuid {
        let mut bytes = [0u8; 16];
        bytes[..8].copy_from_slice(&self.next_u64().to_le_bytes());
        bytes[8..].copy_from_slice(&self.next_u64().to_le_bytes());
        uuid::Builder::from_random_bytes(bytes).into_uuid()
    }
}

/// Builds The Lanterns Hotel as it would look on `today`.
pub fn demo_hotel(seed: u64, today: NaiveDate) -> DemoHotel {
    let mut g = Gen::new(seed, today);
    let (room_types, floors, spaces) = g.building();
    let boards = g.boards();
    let rate_rules = g.rate_rules();
    let (tax_rates, invoice_settings) = g.tax();
    let suite_rates = g.suite_rates(&spaces);

    let mut cal = Calendar::new(&g, &spaces, &room_types);
    // Events first, so wedding room blocks keep rooms free; then coach tours;
    // then everyone else fills the hotel up to each night's target.
    let events = g.events(&spaces, &room_types, &mut cal);
    let mut stays = g.coach_tours(&mut cal, &boards);
    stays.extend(g.fill(&mut cal, &boards));
    stays.sort_by_key(|s| (arrival(&s.booking), s.booked_on));
    g.duplicate_guest(&mut stays);
    let housekeeping = g.housekeeping(&stays);
    let (saved_reports, dashboards) = g.reports();

    DemoHotel {
        settings: PropertySettings {
            hotel_name: "The Lanterns Hotel".into(),
            address: "1 Harbour Parade\nKelmouth".into(),
            phone: "01632 960100".into(),
            email: "stay@lanterns-hotel.example.com".into(),
            parking_price: PARKING_PRICE,
            ..PropertySettings::default()
        },
        room_types,
        floors,
        spaces,
        boards,
        rate_rules,
        suite_rates,
        tax_rates,
        invoice_settings,
        guests: g.guests,
        stays,
        events,
        housekeeping,
        saved_reports,
        dashboards,
    }
}

fn arrival(b: &Booking) -> NaiveDate {
    b.rooms
        .iter()
        .map(|r| r.stay.start)
        .min()
        .unwrap_or(NaiveDate::MIN)
}

fn add_days(d: NaiveDate, n: i64) -> NaiveDate {
    let days = Days::new(n.unsigned_abs());
    let moved = if n >= 0 {
        d.checked_add_days(days)
    } else {
        d.checked_sub_days(days)
    };
    moved.unwrap_or(d)
}

fn at(day: NaiveDate, h: u32, m: u32) -> NaiveDateTime {
    day.and_time(NaiveTime::from_hms_opt(h, m, 0).unwrap_or_default())
}

// ---------------------------------------------------------------------------
// The building
// ---------------------------------------------------------------------------

const COLS: u16 = 24;
const ROWS: u16 = 10;
const ROOM_W: u16 = 3;
const ROOM_H: u16 = 4;

/// Room types: name, code, usual guests, most guests, price per night, description.
const ROOM_TYPES: [(&str, &str, u8, u8, i64, &str); 6] = [
    (
        "Single",
        "SGL",
        1,
        1,
        65,
        "Cosy single at the back of the house.",
    ),
    (
        "Double",
        "DBL",
        2,
        3,
        95,
        "Double room looking over the old town.",
    ),
    ("Twin", "TWN", 2, 2, 95, "Two single beds, town view."),
    (
        "Sea View Double",
        "SEA",
        2,
        3,
        125,
        "Double with a view across Kelmouth Bay.",
    ),
    (
        "Family",
        "FAM",
        3,
        4,
        145,
        "A double and two singles; room for a cot.",
    ),
    (
        "Lantern Suite",
        "STE",
        2,
        3,
        195,
        "Top-floor suite with a balcony over the harbour.",
    ),
];

/// Every bedroom: floor level, number on that floor, type code, extra
/// features (all rooms have wifi), and a note for the front desk.
const ROOMS: [(i32, u16, &str, &[RoomFeature], &str); 40] = {
    use RoomFeature::{Accessible as A, PetsAllowed as P};
    [
        (0, 1, "DBL", &[A], "Level access, roll-in shower"),
        (0, 2, "TWN", &[A], "Level access, wet room"),
        (0, 3, "DBL", &[P], "Garden door: handy for dogs"),
        (0, 4, "DBL", &[P], ""),
        (0, 5, "TWN", &[P], ""),
        (0, 6, "SGL", &[], ""),
        (
            0,
            7,
            "SGL",
            &[],
            "Next to the bar: can be lively on Fridays",
        ),
        (0, 8, "FAM", &[P], "Garden-facing family room"),
        (1, 1, "SEA", &[], "Bay window"),
        (1, 2, "SEA", &[], ""),
        (1, 3, "SEA", &[], ""),
        (1, 4, "FAM", &[], "Sea-view family room"),
        (1, 5, "SEA", &[], ""),
        (1, 6, "SEA", &[], "Connecting door to 107"),
        (1, 7, "FAM", &[], "Connecting door to 106"),
        (1, 8, "SEA", &[], "Corner room, two windows"),
        (1, 9, "DBL", &[], ""),
        (1, 10, "DBL", &[], ""),
        (1, 11, "TWN", &[], ""),
        (1, 12, "TWN", &[], ""),
        (1, 13, "DBL", &[], ""),
        (1, 14, "SGL", &[], ""),
        (1, 15, "TWN", &[], ""),
        (1, 16, "FAM", &[], ""),
        (2, 1, "SEA", &[], ""),
        (2, 2, "SEA", &[], ""),
        (2, 3, "SEA", &[], ""),
        (2, 4, "SEA", &[], ""),
        (2, 5, "STE", &[], "Balcony over the harbour; roll-top bath"),
        (2, 6, "STE", &[], "Balcony, telescope by the window"),
        (2, 7, "SEA", &[], ""),
        (2, 8, "SEA", &[], "Sloping ceiling: mind your head"),
        (2, 9, "DBL", &[], ""),
        (2, 10, "DBL", &[], ""),
        (2, 11, "TWN", &[], ""),
        (2, 12, "DBL", &[], ""),
        (2, 13, "SGL", &[], ""),
        (2, 14, "DBL", &[], "Out of service: bathroom refit"),
        (2, 15, "TWN", &[], ""),
        (2, 16, "FAM", &[], ""),
    ]
};

/// The one room out of service while its bathroom is refitted.
const OUT_OF_SERVICE: (i32, u16) = (2, 14);

const BALLROOM: &str = "Harbour Ballroom";
const MEETING_ROOM: &str = "Lookout Room";

// ---------------------------------------------------------------------------
// Names (all invented combinations of common names)
// ---------------------------------------------------------------------------

const FIRST_NAMES: [&str; 56] = [
    "Margaret", "Tom", "Aisha", "David", "Sofia", "James", "Priya", "Gareth", "Chloe", "Oliver",
    "Fatima", "Rhys", "Hannah", "Liam", "Mei", "Owen", "Grace", "Arjun", "Ellie", "Dylan", "Ruth",
    "Sam", "Nia", "Peter", "Zara", "Ian", "Lucy", "Kwame", "Jenny", "Mark", "Anna", "Callum",
    "Bethan", "Hassan", "Isla", "George", "Megan", "Ravi", "Sian", "Paul", "Amelia", "Joe",
    "Katie", "Tariq", "Elaine", "Nathan", "Freya", "Stuart", "Leah", "Alan", "Rosie", "Daniel",
    "Carys", "Martin", "Jade", "Ewan",
];

const LAST_NAMES: [&str; 64] = [
    "Hughes",
    "Price",
    "Khan",
    "Lewis",
    "Rossi",
    "Evans",
    "Patel",
    "Jones",
    "Morgan",
    "Davies",
    "Williams",
    "Thomas",
    "Roberts",
    "Walker",
    "Chen",
    "Wright",
    "Taylor",
    "Murphy",
    "Novak",
    "Ahmed",
    "Clarke",
    "Bennett",
    "Owen",
    "Hall",
    "Fisher",
    "Smith",
    "Brown",
    "Wilson",
    "Green",
    "Baker",
    "Turner",
    "Cooper",
    "Ward",
    "King",
    "Hill",
    "Scott",
    "Moore",
    "Jackson",
    "White",
    "Harris",
    "Martin",
    "Thompson",
    "Young",
    "Allen",
    "Mitchell",
    "Campbell",
    "Kelly",
    "Stewart",
    "Reid",
    "Begum",
    "Ali",
    "Kowalski",
    "O'Brien",
    "Fraser",
    "Pearce",
    "Hart",
    "Lloyd",
    "Marsh",
    "Ellis",
    "Dixon",
    "Holt",
    "Penrose",
    "Tregear",
    "Nankervis",
];

const TOWNS: [&str; 16] = [
    "Bristol",
    "Birmingham",
    "Cardiff",
    "Exeter",
    "London",
    "Leeds",
    "Manchester",
    "Bath",
    "Reading",
    "Swindon",
    "Gloucester",
    "Oxford",
    "Nottingham",
    "Plymouth",
    "Taunton",
    "Kelmouth",
];

const STREETS: [&str; 12] = [
    "Elm Grove",
    "Station Road",
    "Church Lane",
    "Mill Street",
    "Park Avenue",
    "Victoria Road",
    "The Crescent",
    "Orchard Close",
    "High Street",
    "Meadow Way",
    "Queens Road",
    "Chapel Row",
];

const GUEST_NOTES: [&str; 10] = [
    "Prefers a quiet room away from the lift",
    "Allergic to feathers: synthetic pillows",
    "Likes a sea view if one is free",
    "Vegetarian",
    "Coeliac: gluten-free breakfast",
    "Hard of hearing: knock loudly",
    "Celebrating their anniversary most years",
    "Usually arrives late on the last train",
    "Brings a small dog",
    "Takes the Kelmouth Herald at breakfast",
];

const BOOKING_NOTES: [&str; 9] = [
    "Late arrival, after 9pm",
    "Anniversary: flowers in the room",
    "Ground floor if possible",
    "Gluten-free breakfast",
    "Cot needed",
    "Birthday: card in the room",
    "Early breakfast, 6:30",
    "Walking the coast path: packed lunches",
    "Arriving by train: may need a taxi",
];

/// Wedding couples' surnames are drawn from LAST_NAMES; these are the firms
/// behind conferences (invented).
/// Coach operators (invented) and their web domains.
const OPERATORS: [(&str, &str); 4] = [
    ("Shoreline Coach Holidays", "shoreline-coaches"),
    ("Westway Tours", "westway-tours"),
    ("Heritage Coast Travel", "heritage-coast"),
    ("Bluebird Coaches", "bluebird-coaches"),
];

const COMPANIES: [&str; 10] = [
    "Brightwater Software",
    "Coastline Surveyors",
    "Kel Valley Growers",
    "Harbourside Housing",
    "Westbay Marine",
    "Southcove Solar",
    "Kelmouth College",
    "Tidewater Insurance",
    "Penhallow Estates",
    "Seabright Health",
];

// ---------------------------------------------------------------------------
// The generator
// ---------------------------------------------------------------------------

struct Gen {
    rng: DemoRng,
    today: NaiveDate,
    /// First night of the history.
    start: NaiveDate,
    /// Number of nights covered, history and future.
    days: usize,
    guests: Vec<Guest>,
    emails: HashSet<String>,
}

impl Gen {
    fn new(seed: u64, today: NaiveDate) -> Self {
        Self {
            rng: DemoRng::new(seed),
            today,
            start: add_days(today, -(HISTORY_DAYS as i64)),
            days: (HISTORY_DAYS + FUTURE_DAYS) as usize,
            guests: Vec::new(),
            emails: HashSet::new(),
        }
    }

    fn date(&self, day: usize) -> NaiveDate {
        add_days(self.start, day as i64)
    }

    fn day_index(&self, date: NaiveDate) -> Option<usize> {
        let n = (date - self.start).num_days();
        (0..self.days as i64).contains(&n).then_some(n as usize)
    }

    fn building(&mut self) -> (Vec<RoomType>, Vec<Floor>, Vec<Space>) {
        let types: Vec<RoomType> = ROOM_TYPES
            .iter()
            .map(|&(name, code, usual, most, rate, about)| RoomType {
                id: RoomTypeId(self.rng.uuid()),
                name: name.into(),
                code: code.into(),
                default_occupancy: usual,
                max_occupancy: most,
                description: about.into(),
                base_rate: Money::from_major(rate),
                extra_adult_rate: Money::from_major(25),
                child_rate: Money::from_major(15),
            })
            .collect();
        let floors: Vec<Floor> = [(0, "Ground floor"), (1, "First floor"), (2, "Second floor")]
            .iter()
            .map(|&(level, name)| Floor {
                id: FloorId(self.rng.uuid()),
                name: name.into(),
                level,
                grid_cols: COLS,
                grid_rows: ROWS,
            })
            .collect();

        let mut spaces = Vec::new();
        for &(level, number, code, extra, note) in &ROOMS {
            // Eight rooms along the top (sea side), eight along the bottom.
            let slot = (number - 1) % 8;
            let top = number <= 8;
            let mut features = vec![RoomFeature::Wifi];
            features.extend_from_slice(extra);
            spaces.push(Space {
                id: SpaceId(self.rng.uuid()),
                floor_id: floors[level as usize].id,
                kind: SpaceKind::Room,
                name: if level == 0 {
                    format!("G{number:02}")
                } else {
                    format!("{level}{number:02}")
                },
                room_type_id: types.iter().find(|t| t.code == code).map(|t| t.id),
                suite_capacities: SuiteCapacities::default(),
                rect: GridRect::new(
                    slot * ROOM_W,
                    if top { 0 } else { ROWS - ROOM_H },
                    ROOM_W,
                    ROOM_H,
                ),
                out_of_service: (level, number) == OUT_OF_SERVICE,
                features,
                notes: note.into(),
            });
        }
        let suites = [
            (
                BALLROOM,
                0,
                14,
                SuiteCapacities {
                    theatre: Some(160),
                    banquet: Some(120),
                    cabaret: Some(90),
                    reception: Some(200),
                    ..SuiteCapacities::default()
                },
                "Sprung dance floor; doors onto the terrace",
            ),
            (
                MEETING_ROOM,
                14,
                10,
                SuiteCapacities {
                    theatre: Some(40),
                    boardroom: Some(20),
                    banquet: Some(32),
                    cabaret: Some(24),
                    reception: Some(50),
                },
                "Licensed for civil ceremonies; screen and projector",
            ),
        ];
        for (name, x, w, caps, note) in suites {
            spaces.push(Space {
                id: SpaceId(self.rng.uuid()),
                floor_id: floors[0].id,
                kind: SpaceKind::Suite,
                name: name.into(),
                room_type_id: None,
                suite_capacities: caps,
                rect: GridRect::new(x, ROWS - ROOM_H, w, ROOM_H),
                out_of_service: false,
                features: Vec::new(),
                notes: note.into(),
            });
        }
        (types, floors, spaces)
    }

    fn boards(&mut self) -> Vec<BoardOption> {
        [
            ("RO", "Room only", 0, 0),
            ("BB", "Bed & breakfast", 12, 7),
            ("DBB", "Dinner, bed & breakfast", 38, 19),
        ]
        .iter()
        .map(|&(code, name, adult, child)| BoardOption {
            id: BoardOptionId(self.rng.uuid()),
            code: code.into(),
            name: name.into(),
            adult_price: Money::from_major(adult),
            child_price: Money::from_major(child),
        })
        .collect()
    }

    /// Weekends cost more all year; each year has a summer season, a
    /// Christmas rate and a winter midweek offer.
    fn rate_rules(&mut self) -> Vec<RateRule> {
        let first = self.start;
        let last = self.date(self.days - 1);
        let ymd = |y: i32, m: u32, d: u32| NaiveDate::from_ymd_opt(y, m, d);
        let mut rules = Vec::new();
        let mut rule = |rng: &mut DemoRng,
                        name: String,
                        start: Option<NaiveDate>,
                        end: Option<NaiveDate>,
                        weekdays: Vec<u8>,
                        priority: i32,
                        adjustment: Adjustment| {
            let (Some(start), Some(end)) = (start, end) else {
                return;
            };
            if end < first || start > last {
                return;
            }
            rules.push(RateRule {
                id: RateRuleId(rng.uuid()),
                name,
                room_type_id: None,
                start,
                end,
                weekdays,
                priority,
                adjustment,
                active: true,
            });
        };
        rule(
            &mut self.rng,
            "Weekend".into(),
            ymd(first.year(), 1, 1),
            ymd(last.year(), 12, 31),
            vec![5, 6],
            10,
            Adjustment::AddAmount {
                amount: Money::from_major(20),
            },
        );
        for y in first.year()..=last.year() {
            rule(
                &mut self.rng,
                format!("Summer season {y}"),
                ymd(y, 7, 1),
                ymd(y, 8, 31),
                vec![],
                5,
                Adjustment::Percent { percent: 20 },
            );
            rule(
                &mut self.rng,
                format!("Christmas & New Year {y}"),
                ymd(y, 12, 22),
                ymd(y + 1, 1, 2),
                vec![],
                5,
                Adjustment::Percent { percent: 25 },
            );
            rule(
                &mut self.rng,
                format!("Winter midweek {y}"),
                ymd(y, 1, 6),
                ymd(y, 2, 28),
                vec![1, 2, 3, 4],
                20,
                Adjustment::Percent { percent: -15 },
            );
        }
        rules
    }

    fn tax(&mut self) -> (Vec<TaxRate>, InvoiceSettings) {
        let rates: Vec<TaxRate> = [
            ("Standard VAT", VAT),
            ("Reduced VAT", 500),
            ("Zero rated", 0),
        ]
        .iter()
        .map(|&(name, basis_points)| TaxRate {
            id: TaxRateId(self.rng.uuid()),
            name: name.into(),
            basis_points,
        })
        .collect();
        let all = Some(rates[0].id);
        let settings = InvoiceSettings {
            prices_include_tax: true,
            prefix: "LAN-".into(),
            footer: "The Lanterns Hotel Ltd (demo) · VAT no. GB 000 0000 00\n\
                     Bank transfer: sort code 00-00-00, account 00000000\n\
                     Thank you for staying with us in Kelmouth."
                .into(),
            accommodation_rate: all,
            board_rate: all,
            suite_rate: all,
            food_rate: all,
            drink_rate: all,
            other_rate: all,
        };
        (rates, settings)
    }

    fn suite_rates(&mut self, spaces: &[Space]) -> Vec<(SpaceId, SuiteRates)> {
        let rate = |hourly, min_hours, half, full| SuiteRates {
            hourly: Some(Money::from_major(hourly)),
            min_hours,
            half_day: Some(Money::from_major(half)),
            full_day: Some(Money::from_major(full)),
            setup_minutes: 60,
            cleardown_minutes: 30,
        };
        spaces
            .iter()
            .filter(|s| s.kind == SpaceKind::Suite)
            .map(|s| {
                let rates = if s.name == BALLROOM {
                    rate(140, 3, 520, 900)
                } else {
                    rate(45, 2, 160, 290)
                };
                (s.id, rates)
            })
            .collect()
    }

    // ---- guests -------------------------------------------------------------

    /// A new invented guest, with an email no one else has.
    fn new_guest(&mut self) -> Guest {
        let first = *self.rng.pick(&FIRST_NAMES);
        let last = *self.rng.pick(&LAST_NAMES);
        let domain = *self
            .rng
            .pick(&["example.com", "example.org", "example.net"]);
        let base = format!(
            "{}.{}",
            first.to_lowercase(),
            last.to_lowercase().replace('\'', "")
        );
        let mut email = format!("{base}@{domain}");
        let mut n = 2;
        while !self.emails.insert(email.clone()) {
            email = format!("{base}{n}@{domain}");
            n += 1;
        }
        let phone = if self.rng.chance(75) {
            format!("07700 900{:03}", self.rng.below(1000))
        } else {
            format!("01632 960{:03}", self.rng.below(1000))
        };
        let address = if self.rng.chance(60) {
            format!(
                "{} {}\n{}",
                self.rng.between(1, 120),
                self.rng.pick(&STREETS),
                self.rng.pick(&TOWNS)
            )
        } else {
            String::new()
        };
        let notes = if self.rng.chance(8) {
            (*self.rng.pick(&GUEST_NOTES)).into()
        } else {
            String::new()
        };
        Guest {
            id: GuestId(self.rng.uuid()),
            first_name: first.into(),
            last_name: last.into(),
            email,
            phone,
            address,
            notes,
        }
    }

    /// About one guest in five has stayed before.
    fn guest_for_booking(&mut self) -> GuestId {
        if self.guests.len() > 20 && self.rng.chance(20) {
            return self.rng.pick(&self.guests).id;
        }
        let guest = self.new_guest();
        let id = guest.id;
        self.guests.push(guest);
        id
    }

    /// A contact who isn't one of the hotel's staying guests.
    fn organiser(&mut self, email: Option<String>) -> Guest {
        let mut guest = self.new_guest();
        if let Some(email) = email {
            guest.email = email;
        }
        guest.address = String::new();
        guest.notes = String::new();
        guest
    }

    /// A regular, Margaret Trevithick, who was once booked as "Maggie" with
    /// a different phone and no email: a duplicate to show merging.
    fn duplicate_guest(&mut self, stays: &mut [DemoStay]) {
        let mut visits: std::collections::HashMap<GuestId, Vec<usize>> = Default::default();
        for (i, s) in stays.iter().enumerate() {
            if s.booking.status == BookingStatus::CheckedOut && s.booking.rooms.len() == 1 {
                visits.entry(s.booking.guest_id).or_default().push(i);
            }
        }
        // The regular with the most stays (lowest id wins a tie, so it's repeatable).
        let Some((&id, list)) = visits
            .iter()
            .filter(|(id, _)| {
                !self
                    .guests
                    .iter()
                    .any(|g| g.id == **id && g.notes.starts_with("Tour manager"))
            })
            .max_by_key(|(id, v)| (v.len(), std::cmp::Reverse(**id)))
        else {
            return;
        };
        let Some(&moved) = list.get(1) else {
            return;
        };
        let Some(regular) = self.guests.iter_mut().find(|g| g.id == id) else {
            return;
        };
        regular.first_name = "Margaret".into();
        regular.last_name = "Trevithick".into();
        regular.email = "margaret.trevithick@example.org".into();
        regular.notes = "Comes every spring and autumn; likes room 104 or a sea view".into();
        let mut maggie = self.new_guest();
        maggie.first_name = "Maggie".into();
        maggie.last_name = "Trevithick".into();
        maggie.email = String::new();
        maggie.notes = String::new();
        stays[moved].booking.guest_id = maggie.id;
        self.guests.push(maggie);
    }

    // ---- demand -------------------------------------------------------------

    /// How full the hotel is on a night, in percent: busy summers and school
    /// holidays, quiet Januaries, busier at weekends.
    fn target_percent(night: NaiveDate) -> u64 {
        const MONTH: [i64; 12] = [36, 44, 52, 66, 72, 82, 92, 95, 80, 64, 42, 50];
        let (m, d) = (night.month(), night.day());
        let mut p = MONTH[night.month0() as usize];
        p += match night.weekday() {
            Weekday::Fri | Weekday::Sat => 12,
            Weekday::Sun => -10,
            Weekday::Mon => -4,
            _ => 0,
        };
        // February half-term, Easter (roughly), October half-term.
        if (m == 2 && (12..=20).contains(&d))
            || (m == 4 && (1..=14).contains(&d))
            || (m == 10 && d >= 24)
        {
            p += 14;
        }
        if (m == 12 && d >= 23) || (m == 1 && d == 1) {
            p = 88;
        }
        if m == 12 && (24..=25).contains(&d) {
            p = 70;
        }
        p.clamp(10, 98) as u64
    }

    /// Days between booking and arriving.
    fn lead_days(&mut self, summer: bool) -> u64 {
        let bucket = self.rng.weighted(&[
            (0, 4),
            (1, 18),
            (2, 28),
            (3, if summer { 26 } else { 30 }),
            (4, if summer { 18 } else { 14 }),
            (5, if summer { 8 } else { 4 }),
        ]);
        match bucket {
            0 => 0,
            1 => self.rng.between(1, 7),
            2 => self.rng.between(8, 30),
            3 => self.rng.between(31, 90),
            4 => self.rng.between(91, 180),
            _ => self.rng.between(181, 300),
        }
    }

    fn nights(&mut self, summer: bool) -> u64 {
        self.rng.weighted(&[
            (1, 22),
            (2, 30),
            (3, 20),
            (4, 8),
            (5, 6),
            (6, 3),
            (7, if summer { 22 } else { 8 }),
        ])
    }

    fn source(&mut self, lead: u64) -> BookingSource {
        if lead == 0 {
            return self
                .rng
                .weighted(&[(BookingSource::WalkIn, 6), (BookingSource::Phone, 4)]);
        }
        self.rng.weighted(&[
            (BookingSource::Phone, 30),
            (BookingSource::Email, 14),
            (BookingSource::Website, 36),
            (BookingSource::Agent, 12),
            (BookingSource::Other, 4),
        ])
    }

    /// Status for a stay as of today.
    fn status(&mut self, stay: DateRange) -> BookingStatus {
        let today = self.today;
        if stay.end < today || (stay.end == today && self.rng.chance(55)) {
            if self.rng.chance(2) {
                BookingStatus::NoShow
            } else {
                BookingStatus::CheckedOut
            }
        } else if stay.start < today {
            BookingStatus::CheckedIn
        } else if stay.start == today {
            // Arriving today: a few are already in.
            if self.rng.chance(20) {
                BookingStatus::CheckedIn
            } else {
                BookingStatus::Confirmed
            }
        } else if self.rng.chance(10) {
            BookingStatus::Provisional
        } else {
            BookingStatus::Confirmed
        }
    }

    fn party(&mut self, room_type: &RoomType) -> (u8, u8) {
        match room_type.code.as_str() {
            "SGL" => (1, 0),
            "FAM" => (2, self.rng.between(1, 2) as u8),
            "TWN" => {
                if self.rng.chance(15) {
                    (1, 1)
                } else {
                    (2, 0)
                }
            }
            _ => {
                let adults = if self.rng.chance(12) { 1 } else { 2 };
                let children = u8::from(adults == 2 && self.rng.chance(8));
                (adults, children.min(room_type.max_occupancy - adults))
            }
        }
    }

    fn board(&mut self, night: NaiveDate, boards: &[BoardOption]) -> Option<BoardOptionId> {
        let winter = matches!(night.month(), 11 | 12 | 1 | 2 | 3);
        let pick = self
            .rng
            .weighted(&[(0, 18), (1, 56), (2, if winter { 34 } else { 20 })]);
        // Room only is stored as no board, as the booking form does.
        (pick > 0).then(|| boards[pick].id)
    }

    // ---- bookings -----------------------------------------------------------

    /// A booking for rooms already checked free, with its bill.
    #[allow(clippy::too_many_arguments)]
    fn stay(
        &mut self,
        cal: &Calendar,
        rooms: &[usize],
        stay: DateRange,
        booked_on: NaiveDate,
        boards: &[BoardOption],
        source: BookingSource,
        guest_id: GuestId,
        notes: String,
    ) -> DemoStay {
        let status = self.status(stay);
        let board = self.board(stay.start, boards);
        let mut booked = Vec::new();
        let mut has_dog = false;
        for &r in rooms {
            let room = &cal.rooms[r];
            let (adults, children) = self.party(&room.room_type);
            let cars = if self.rng.chance(if children > 0 { 55 } else { 30 }) {
                1
            } else {
                0
            };
            has_dog |= room.pets && self.rng.chance(40);
            booked.push(BookingRoom {
                id: BookingRoomId(self.rng.uuid()),
                space_id: room.id,
                stay,
                adults,
                children,
                board_option_id: board,
                price_override: None,
                cars,
            });
        }
        let notes = if has_dog && notes.is_empty() {
            "Bringing a dog".to_string()
        } else {
            notes
        };
        let booking = Booking {
            id: BookingId(self.rng.uuid()),
            reference: String::new(),
            guest_id,
            status,
            source,
            notes,
            rooms: booked,
        };
        let account = self.stay_account(
            &booking,
            stay,
            booked_on,
            has_dog,
            board == Some(boards[2].id),
        );
        DemoStay {
            booking,
            booked_on,
            account,
        }
    }

    /// Bar, restaurant and other extras for nights already spent, a deposit
    /// for bookings made well ahead, and the bill paid at check-out.
    fn stay_account(
        &mut self,
        booking: &Booking,
        stay: DateRange,
        booked_on: NaiveDate,
        has_dog: bool,
        dinner_included: bool,
    ) -> DemoAccount {
        let mut account = DemoAccount::default();
        let rooms = booking.rooms.len() as i64;
        let lead = (stay.start - booked_on).num_days();
        if lead > 14 && booking.status != BookingStatus::Provisional && self.rng.chance(60) {
            let method = if booking.source == BookingSource::Website {
                PaymentMethod::Card
            } else {
                self.rng
                    .weighted(&[(PaymentMethod::Card, 80), (PaymentMethod::BankTransfer, 20)])
            };
            account.deposit = Some(Payment {
                id: PaymentId(self.rng.uuid()),
                date: booked_on.min(self.today),
                amount: Money::from_major(50 * rooms),
                method,
                is_deposit: true,
                reference: String::new(),
            });
        }
        let in_house = matches!(
            booking.status,
            BookingStatus::CheckedIn | BookingStatus::CheckedOut
        );
        if !in_house {
            return account;
        }
        let last_night = add_days(stay.end, -1).min(self.today);
        let mut night = stay.start;
        while night <= last_night {
            let guests: i64 = booking.rooms.iter().map(|r| i64::from(r.adults)).sum();
            if !dinner_included && self.rng.chance(28) {
                let pence = guests * self.rng.between(24, 38) as i64 * 100;
                let dinner = self.charge(
                    night,
                    ChargeCategory::Food,
                    "Dinner, Lanterns Restaurant",
                    1,
                    pence,
                );
                account.extras.push(dinner);
            }
            if self.rng.chance(45) {
                let pence = self.rng.between(6, 48) as i64 * 100 + 50 * self.rng.below(2) as i64;
                let bar = self.charge(night, ChargeCategory::Drink, "Bar", 1, pence);
                account.extras.push(bar);
            }
            if self.rng.chance(6) {
                account.extras.push(self.charge(
                    night,
                    ChargeCategory::Food,
                    "Afternoon tea",
                    guests.clamp(1, 4) as u16,
                    2_200,
                ));
            }
            if self.rng.chance(4) {
                account.extras.push(self.charge(
                    night,
                    ChargeCategory::Food,
                    "Packed lunch",
                    guests.clamp(1, 4) as u16,
                    950,
                ));
            }
            if has_dog {
                account.extras.push(self.charge(
                    night,
                    ChargeCategory::Other,
                    "Dog stay",
                    1,
                    1_500,
                ));
            }
            night = add_days(night, 1);
        }
        if booking.status == BookingStatus::CheckedOut {
            if self.rng.chance(5) {
                account.extras.push(self.charge(
                    stay.end,
                    ChargeCategory::Other,
                    "Late check-out",
                    1,
                    2_000,
                ));
            }
            let method = if booking.source == BookingSource::Agent {
                PaymentMethod::BankTransfer
            } else {
                self.rng.weighted(&[
                    (PaymentMethod::Card, 76),
                    (PaymentMethod::Cash, 10),
                    (PaymentMethod::BankTransfer, 6),
                    (PaymentMethod::Voucher, 4),
                    (PaymentMethod::Cheque, 2),
                    (PaymentMethod::Other, 2),
                ])
            };
            account.settle = Some((stay.end, method));
            if booking.source == BookingSource::Agent || self.rng.chance(15) {
                account.invoice_on = Some(stay.end);
            }
        }
        account
    }

    fn charge(
        &mut self,
        date: NaiveDate,
        category: ChargeCategory,
        description: &str,
        quantity: u16,
        pence: i64,
    ) -> Charge {
        Charge {
            id: ChargeId(self.rng.uuid()),
            date,
            category,
            description: description.into(),
            quantity,
            unit_price: Money(pence),
            tax_basis_points: VAT,
        }
    }

    /// Fills each night up to its target. A stay whose booking date is still
    /// in the future hasn't been made yet: its nights are counted as taken so
    /// the target isn't refilled, but no booking is written. That gives the
    /// real-looking pick-up curve: the further ahead, the emptier.
    fn fill(&mut self, cal: &mut Calendar, boards: &[BoardOption]) -> Vec<DemoStay> {
        let mut stays = Vec::new();
        let bookable = cal.bookable();
        for day in 0..self.days {
            let night = self.date(day);
            let target = (Self::target_percent(night) * bookable.len() as u64 + 50) / 100;
            let mut taken = bookable.iter().filter(|&&r| cal.taken(r, day)).count() as u64;
            // Even a quiet night after a busy weekend has a few arrivals.
            // Today gets a few more, so the front desk has people to check in.
            let mut arrivals = if night == self.today {
                self.rng.between(4, 6)
            } else {
                self.rng.between(1, 3)
            };
            while taken < target || arrivals > 0 {
                arrivals = arrivals.saturating_sub(1);
                let free: Vec<usize> = bookable
                    .iter()
                    .copied()
                    .filter(|&r| !cal.taken(r, day))
                    .collect();
                if free.is_empty() {
                    break;
                }
                let summer = matches!(night.month(), 7 | 8);
                let r = *self.rng.pick(&free);
                let wanted = self.nights(summer) as usize;
                let nights = cal.free_run(r, day, wanted);
                cal.take(r, day, nights);
                taken += 1;
                let lead = self.lead_days(summer);
                let booked_on = add_days(night, -(lead as i64));
                if booked_on > self.today {
                    continue;
                }
                let stay = DateRange {
                    start: night,
                    end: add_days(night, nights as i64),
                };
                // Now and then a family or friends take two rooms together.
                let mut rooms = vec![r];
                if self.rng.chance(6)
                    && let Some(&other) = free
                        .iter()
                        .find(|&&o| o != r && cal.free_run(o, day, nights) == nights)
                {
                    cal.take(other, day, nights);
                    taken += 1;
                    rooms.push(other);
                }
                let source = self.source(lead);
                let guest = self.guest_for_booking();
                let notes = if self.rng.chance(12) {
                    (*self.rng.pick(&BOOKING_NOTES)).into()
                } else {
                    String::new()
                };
                let made = self.stay(cal, &rooms, stay, booked_on, boards, source, guest, notes);
                // About one booking in thirteen is also cancelled by someone
                // else for the same room and dates.
                if self.rng.chance(8) {
                    let mut cancelled = made.clone();
                    cancelled.booking.id = BookingId(self.rng.uuid());
                    for room in &mut cancelled.booking.rooms {
                        room.id = BookingRoomId(self.rng.uuid());
                    }
                    cancelled.booking.status = BookingStatus::Cancelled;
                    cancelled.booking.guest_id = self.guest_for_booking();
                    cancelled.booking.notes = String::new();
                    cancelled.account = DemoAccount::default();
                    cancelled.booked_on =
                        add_days(booked_on, -(self.rng.between(5, 40) as i64)).min(self.today);
                    stays.push(cancelled);
                }
                stays.push(made);
            }
        }
        stays
    }

    /// Coach tours from March to November: a dozen or so rooms for two to
    /// four nights, dinner included, at a group rate agreed with the
    /// operator, booked through them months ahead. The driver's room is free.
    /// Each operator sends the same tour manager every time.
    fn coach_tours(&mut self, cal: &mut Calendar, boards: &[BoardOption]) -> Vec<DemoStay> {
        let mut stays = Vec::new();
        let mut managers: Vec<Option<GuestId>> = vec![None; OPERATORS.len()];
        let first = self.start.year();
        let last = self.date(self.days - 1).year();
        for year in first..=last {
            // Fewer in high summer, when families fill the hotel.
            for (month, tours) in [
                (3, 1),
                (4, 2),
                (5, 2),
                (6, 2),
                (7, 1),
                (8, 1),
                (9, 2),
                (10, 2),
                (11, 1),
            ] {
                let mut weeks: Vec<u32> = vec![0, 1, 2, 3];
                for _ in 0..tours {
                    let week = weeks.remove(self.rng.below(weeks.len() as u64) as usize);
                    let weekday = *self.rng.pick(&[Weekday::Sun, Weekday::Mon, Weekday::Tue]);
                    let Some(mut arrive) = NaiveDate::from_ymd_opt(year, month, 1 + week * 7)
                    else {
                        continue;
                    };
                    while arrive.weekday() != weekday {
                        arrive = add_days(arrive, 1);
                    }
                    if let Some(tour) = self.coach_tour(cal, boards, arrive, &mut managers) {
                        stays.push(tour);
                    }
                }
            }
        }
        stays
    }

    fn coach_tour(
        &mut self,
        cal: &mut Calendar,
        boards: &[BoardOption],
        arrive: NaiveDate,
        managers: &mut [Option<GuestId>],
    ) -> Option<DemoStay> {
        let nights = self.rng.weighted(&[(2usize, 3), (3, 5), (4, 2)]);
        let booked_on = add_days(arrive, -(self.rng.between(90, 240) as i64));
        let day = self.day_index(arrive)?;
        if booked_on > self.today || day + nights > self.days {
            return None;
        }
        let free = |cal: &Calendar, codes: &[&str]| -> Vec<usize> {
            cal.bookable()
                .into_iter()
                .filter(|&r| {
                    codes.contains(&cal.rooms[r].room_type.code.as_str())
                        && cal.free_run(r, day, nights) == nights
                })
                .collect()
        };
        let pairs = self.rng.between(8, 14) as usize;
        let mut rooms: Vec<usize> = free(cal, &["DBL", "TWN"]).into_iter().take(pairs).collect();
        // Solo travellers, and the driver, in singles.
        let singles: Vec<usize> = free(cal, &["SGL"])
            .into_iter()
            .take(self.rng.between(1, 3) as usize)
            .collect();
        if rooms.len() < 6 || singles.is_empty() {
            return None;
        }
        let cancelled = arrive > self.today && self.rng.chance(6);
        rooms.extend(&singles);
        if !cancelled {
            for &r in &rooms {
                cal.take(r, day, nights);
            }
        }

        let op = self.rng.below(OPERATORS.len() as u64) as usize;
        let (operator, domain) = OPERATORS[op];
        let manager = match managers[op] {
            Some(id) => id,
            None => {
                let mut lead = self.organiser(Some(format!("tours@{domain}.example.com")));
                lead.notes = format!("Tour manager for {operator}");
                let id = lead.id;
                self.guests.push(lead);
                managers[op] = Some(id);
                id
            }
        };
        let stay = DateRange {
            start: arrive,
            end: add_days(arrive, nights as i64),
        };
        let guests = (rooms.len() - singles.len()) * 2 + singles.len() - 1;
        let mut tour = self.stay(
            cal,
            &rooms,
            stay,
            booked_on,
            boards,
            BookingSource::Agent,
            manager,
            format!(
                "{operator} tour: {guests} guests plus driver. Coach parks on Quay Street. \
                 Dinner 7pm, one long table. Rooming list a week before."
            ),
        );
        let count = tour.booking.rooms.len();
        let driver = count - 1;
        for (i, room) in tour.booking.rooms.iter_mut().enumerate() {
            let single = i >= count - singles.len();
            room.board_option_id = Some(boards[2].id);
            room.cars = 0;
            room.children = 0;
            room.adults = if single { 1 } else { 2 };
            // The group rate covers room and dinner; the driver stays free.
            let per_night = if i == driver {
                0
            } else if single {
                75
            } else {
                118
            };
            room.price_override = Some(Money::from_major(per_night * nights as i64));
        }
        if cancelled {
            tour.booking.status = BookingStatus::Cancelled;
            tour.account = DemoAccount::default();
        } else {
            // Agents settle by bank transfer and always get an invoice; no
            // bar or dinner extras on the group bill (those go on guests' own).
            tour.account.extras.clear();
            tour.account.deposit = None;
        }
        Some(tour)
    }

    // ---- events -------------------------------------------------------------

    fn events(
        &mut self,
        spaces: &[Space],
        types: &[RoomType],
        cal: &mut Calendar,
    ) -> Vec<DemoEvent> {
        let suite = |name: &str| {
            spaces
                .iter()
                .find(|s| s.name == name)
                .map(|s| s.id)
                .unwrap_or_default()
        };
        let (ballroom, lookout) = (suite(BALLROOM), suite(MEETING_ROOM));
        let double = types.iter().find(|t| t.code == "DBL").map(|t| t.id);
        let mut used: HashSet<(SpaceId, NaiveDate)> = HashSet::new();
        let mut events = Vec::new();

        for day in 0..self.days {
            let date = self.date(day);
            let (m, d) = (date.month(), date.day());
            let festive = (m == 12 && d >= 20) || (m == 1 && d <= 2);
            // Office Christmas parties fill late November and December weekends.
            let party_season = (m == 11 && d >= 20) || (m == 12 && d <= 20);
            if party_season
                && matches!(date.weekday(), Weekday::Fri | Weekday::Sat)
                && self.rng.chance(75)
            {
                events.push(self.christmas_party(date, ballroom));
                used.insert((ballroom, date));
                continue;
            }
            match date.weekday() {
                Weekday::Sat if !festive => {
                    let chance = match m {
                        5..=9 => 45,
                        3 | 4 | 10 => 15,
                        _ => 5,
                    };
                    if self.rng.chance(chance) {
                        events.push(self.wedding(date, ballroom, lookout, double, cal));
                        used.insert((ballroom, date));
                    } else if self.rng.chance(12) {
                        events.push(self.birthday(date, ballroom));
                        used.insert((ballroom, date));
                    }
                }
                Weekday::Tue | Weekday::Wed | Weekday::Thu if !festive => {
                    // About a conference and a half a month.
                    if self.rng.chance(10) {
                        events.push(self.conference(date, ballroom, lookout));
                        used.insert((lookout, date));
                    }
                }
                _ => {}
            }
            // The business club dines on the first Thursday of the month.
            if date.weekday() == Weekday::Thu && d <= 7 && !used.contains(&(lookout, date)) {
                events.push(self.club_dinner(date, lookout));
            }
            if m == 12 && d == 25 {
                events.push(self.festive(date, ballroom, false));
            }
            if m == 12 && d == 31 {
                events.push(self.festive(date, ballroom, true));
            }
        }
        events
    }

    /// Confirmed if it's past, sometimes still provisional if it's ahead.
    fn suite_status(&mut self, date: NaiveDate) -> SuiteBookingStatus {
        if date > self.today && self.rng.chance(25) {
            SuiteBookingStatus::Provisional
        } else {
            SuiteBookingStatus::Confirmed
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn suite_booking(
        &mut self,
        suite: SpaceId,
        event: &Event,
        organiser: GuestId,
        title: &str,
        (start, end): (NaiveDateTime, NaiveDateTime),
        layout: Layout,
        attendees: u16,
        pricing: SuitePricing,
        status: SuiteBookingStatus,
    ) -> SuiteBooking {
        SuiteBooking {
            id: SuiteBookingId(self.rng.uuid()),
            suite_id: suite,
            event_id: Some(event.id),
            guest_id: organiser,
            title: title.into(),
            start,
            end,
            layout,
            attendees,
            setup_minutes: 60,
            cleardown_minutes: 30,
            pricing,
            status,
            notes: String::new(),
        }
    }

    fn sitting(
        name: &str,
        date: NaiveDate,
        hour: u32,
        adults: u16,
        children: u16,
        dietary: &str,
    ) -> CateringSitting {
        CateringSitting {
            name: name.into(),
            date,
            time: NaiveTime::from_hms_opt(hour, 0, 0).unwrap_or_default(),
            adults,
            children,
            dietary: dietary.into(),
        }
    }

    /// The account for an event: a deposit when booked, the rest paid by
    /// bank transfer a week after, and an invoice.
    fn event_account(
        &mut self,
        event: &Event,
        deposit: i64,
        booked_on: NaiveDate,
        extras: Vec<Charge>,
    ) -> DemoAccount {
        let mut account = DemoAccount {
            extras,
            ..DemoAccount::default()
        };
        if deposit > 0 && booked_on <= self.today {
            account.deposit = Some(Payment {
                id: PaymentId(self.rng.uuid()),
                date: booked_on,
                amount: Money::from_major(deposit),
                method: PaymentMethod::BankTransfer,
                is_deposit: true,
                reference: format!("DEP {}", event.name.to_uppercase().replace(' ', "")),
            });
        }
        let paid_on = add_days(event.end_date, 7);
        if paid_on <= self.today {
            account.settle = Some((paid_on, PaymentMethod::BankTransfer));
            account.invoice_on = Some(add_days(event.end_date, 1));
        }
        account
    }

    fn wedding(
        &mut self,
        date: NaiveDate,
        ballroom: SpaceId,
        lookout: SpaceId,
        double: Option<RoomTypeId>,
        cal: &mut Calendar,
    ) -> DemoEvent {
        let one = *self.rng.pick(&LAST_NAMES);
        let mut two = *self.rng.pick(&LAST_NAMES);
        while two == one {
            two = *self.rng.pick(&LAST_NAMES);
        }
        let mut organiser = self.organiser(None);
        organiser.last_name = one.into();
        let adults = self.rng.between(50, 105) as u16;
        let children = self.rng.between(0, 12) as u16;
        let evening = (adults + self.rng.between(20, 60) as u16).min(180);
        let mut event = Event {
            id: EventId(self.rng.uuid()),
            name: format!("{one}–{two} wedding"),
            kind: EventKind::Wedding,
            organiser_id: organiser.id,
            start_date: date,
            end_date: date,
            notes: (*self.rng.pick(&[
                "Band brings their own PA. Cake delivered Friday afternoon.",
                "Bride's mother is the contact on the day.",
                "Photographer wants the terrace at 4pm for group shots.",
                "Two guests use wheelchairs: tables near the terrace doors.",
            ]))
            .into(),
            running_order: "12:30 Guests arrive, Lookout Room\n13:00 Ceremony\n\
                            14:00 Drinks on the terrace\n15:00 Wedding breakfast, Harbour Ballroom\n\
                            17:00 Speeches\n19:30 Evening guests and buffet\n00:00 Carriages"
                .into(),
            sittings: vec![
                Self::sitting(
                    "Wedding breakfast",
                    date,
                    15,
                    adults,
                    children,
                    "6 vegetarian\n2 vegan\n1 coeliac",
                ),
                Self::sitting("Evening buffet", date, 20, evening, children, "Label all allergens"),
            ],
            room_block: None,
        };
        // Weddings more than five weeks away hold a block of doubles for
        // guests; keep that many doubles free so the hotel can hold them.
        if date > add_days(self.today, 35)
            && let Some(day) = self.day_index(date)
        {
            let want = self.rng.between(4, 7) as usize;
            let rooms: Vec<usize> = cal
                .bookable()
                .into_iter()
                .filter(|&r| cal.rooms[r].room_type.code == "DBL" && cal.free_run(r, day, 1) == 1)
                .take(want)
                .collect();
            for &r in &rooms {
                cal.take(r, day, 1);
            }
            if !rooms.is_empty() {
                event.room_block = Some(RoomBlock {
                    room_type_id: double,
                    rooms: rooms.len() as u16,
                    stay: DateRange {
                        start: date,
                        end: add_days(date, 1),
                    },
                    rate: Money::from_major(85),
                    release_date: add_days(date, -28),
                });
            }
        }
        let status = self.suite_status(date);
        let suites = vec![
            self.suite_booking(
                lookout,
                &event,
                organiser.id,
                "Ceremony",
                (at(date, 12, 30), at(date, 14, 0)),
                Layout::Theatre,
                (adults + children).min(40),
                SuitePricing::Hourly,
                status,
            ),
            self.suite_booking(
                ballroom,
                &event,
                organiser.id,
                "Wedding breakfast and evening party",
                (at(date, 15, 0), at(date, 23, 59)),
                Layout::Banquet,
                (adults + children).min(120),
                SuitePricing::PerHead {
                    price: Money::from_major(58),
                },
                status,
            ),
        ];
        let booked_on = add_days(date, -(self.rng.between(200, 450) as i64));
        let extras = if date < self.today {
            vec![
                self.charge(
                    date,
                    ChargeCategory::Drink,
                    "Drinks reception: prosecco and soft drinks",
                    adults,
                    750,
                ),
                self.charge(
                    date,
                    ChargeCategory::Drink,
                    "Wine with the wedding breakfast",
                    adults / 3,
                    2_400,
                ),
            ]
        } else {
            Vec::new()
        };
        let account = self.event_account(&event, 1_000, booked_on, extras);
        DemoEvent {
            event,
            organiser,
            suite_bookings: suites,
            account,
        }
    }

    fn birthday(&mut self, date: NaiveDate, ballroom: SpaceId) -> DemoEvent {
        let organiser = self.organiser(None);
        let age = *self.rng.pick(&[40, 50, 60, 70, 18, 21]);
        let guests = self.rng.between(50, 150) as u16;
        let event = Event {
            id: EventId(self.rng.uuid()),
            name: format!("{} {}th birthday", organiser.first_name, age),
            kind: EventKind::Party,
            organiser_id: organiser.id,
            start_date: date,
            end_date: date,
            notes: "DJ from 8pm. Cake brought in by the family.".into(),
            running_order: "19:00 Doors\n20:00 Buffet\n20:30 DJ\n23:30 Last orders".into(),
            sittings: vec![Self::sitting(
                "Buffet",
                date,
                20,
                guests,
                0,
                "Vegetarian options",
            )],
            room_block: None,
        };
        let status = self.suite_status(date);
        let suites = vec![self.suite_booking(
            ballroom,
            &event,
            organiser.id,
            "Birthday party",
            (at(date, 19, 0), at(date, 23, 59)),
            Layout::Reception,
            guests,
            SuitePricing::PerHead {
                price: Money::from_major(22),
            },
            status,
        )];
        let booked_on = add_days(date, -(self.rng.between(30, 120) as i64));
        let account = self.event_account(&event, 200, booked_on, Vec::new());
        DemoEvent {
            event,
            organiser,
            suite_bookings: suites,
            account,
        }
    }

    fn christmas_party(&mut self, date: NaiveDate, ballroom: SpaceId) -> DemoEvent {
        let company = *self.rng.pick(&COMPANIES);
        let slug = company.to_lowercase().replace(' ', "");
        let organiser = self.organiser(Some(format!("social@{slug}.example.com")));
        let guests = self.rng.between(40, 110) as u16;
        let event = Event {
            id: EventId(self.rng.uuid()),
            name: format!("{company} Christmas party"),
            kind: EventKind::Party,
            organiser_id: organiser.id,
            start_date: date,
            end_date: date,
            notes: "Crackers on the tables. Taxis booked for midnight.".into(),
            running_order: "19:00 Fizz\n19:45 Three-course dinner\n21:30 Disco\n00:00 Carriages"
                .into(),
            sittings: vec![Self::sitting(
                "Christmas dinner",
                date,
                19,
                guests,
                0,
                "8 vegetarian\n2 vegan\n1 nut allergy",
            )],
            room_block: None,
        };
        let status = self.suite_status(date);
        let suites = vec![self.suite_booking(
            ballroom,
            &event,
            organiser.id,
            "Christmas party",
            (at(date, 19, 0), at(date, 23, 59)),
            Layout::Banquet,
            guests.min(120),
            SuitePricing::PerHead {
                price: Money::from_major(45),
            },
            status,
        )];
        let booked_on = add_days(date, -(self.rng.between(60, 200) as i64));
        let account = self.event_account(&event, 250, booked_on, Vec::new());
        DemoEvent {
            event,
            organiser,
            suite_bookings: suites,
            account,
        }
    }

    fn conference(&mut self, date: NaiveDate, ballroom: SpaceId, lookout: SpaceId) -> DemoEvent {
        let company = *self.rng.pick(&COMPANIES);
        let slug = company.to_lowercase().replace(' ', "");
        let organiser = self.organiser(Some(format!("events@{slug}.example.com")));
        let delegates = self.rng.between(40, 140) as u16;
        let kind = *self
            .rng
            .pick(&["conference", "away day", "annual meeting", "training day"]);
        let event = Event {
            id: EventId(self.rng.uuid()),
            name: format!("{company} {kind}"),
            kind: EventKind::Conference,
            organiser_id: organiser.id,
            start_date: date,
            end_date: date,
            notes: format!("Invoice to {company} Ltd. Projector and two lapel mics."),
            running_order: "08:30 Registration and coffee\n09:00 Welcome\n\
                            12:30 Lunch\n13:30 Breakouts (Lookout Room)\n17:00 Close"
                .into(),
            sittings: vec![Self::sitting(
                "Buffet lunch",
                date,
                12,
                delegates,
                0,
                "10 vegetarian\n3 halal",
            )],
            room_block: None,
        };
        // Now and then a provisional booking ahead falls through.
        let status = if date > self.today && self.rng.chance(10) {
            SuiteBookingStatus::Cancelled
        } else {
            self.suite_status(date)
        };
        let suites = vec![
            self.suite_booking(
                ballroom,
                &event,
                organiser.id,
                "Main sessions",
                (at(date, 8, 30), at(date, 17, 0)),
                Layout::Theatre,
                delegates,
                SuitePricing::FullDay,
                status,
            ),
            self.suite_booking(
                lookout,
                &event,
                organiser.id,
                "Breakouts",
                (at(date, 13, 30), at(date, 17, 0)),
                Layout::Boardroom,
                20,
                SuitePricing::HalfDay,
                status,
            ),
        ];
        let booked_on = add_days(date, -(self.rng.between(30, 150) as i64));
        let extras = if date < self.today {
            vec![self.charge(
                date,
                ChargeCategory::Food,
                "Delegate lunch",
                delegates,
                1_650,
            )]
        } else {
            Vec::new()
        };
        let account = self.event_account(&event, 0, booked_on, extras);
        DemoEvent {
            event,
            organiser,
            suite_bookings: suites,
            account,
        }
    }

    fn club_dinner(&mut self, date: NaiveDate, lookout: SpaceId) -> DemoEvent {
        let organiser = self.organiser(Some("secretary@kelmouth-business-club.example.org".into()));
        let diners = self.rng.between(20, 32) as u16;
        let event = Event {
            id: EventId(self.rng.uuid()),
            name: "Kelmouth Business Club dinner".into(),
            kind: EventKind::Dinner,
            organiser_id: organiser.id,
            start_date: date,
            end_date: date,
            notes: "Guest speaker needs a lectern.".into(),
            running_order: "19:00 Drinks\n19:30 Dinner\n21:00 Speaker".into(),
            sittings: vec![Self::sitting("Dinner", date, 19, diners, 0, "2 vegetarian")],
            room_block: None,
        };
        let status = self.suite_status(date);
        let suites = vec![self.suite_booking(
            lookout,
            &event,
            organiser.id,
            "Dinner",
            (at(date, 19, 0), at(date, 23, 0)),
            Layout::Banquet,
            diners,
            SuitePricing::PerHead {
                price: Money::from_major(36),
            },
            status,
        )];
        let mut account = DemoAccount::default();
        if date < self.today {
            account.extras.push(self.charge(
                date,
                ChargeCategory::Drink,
                "Wine",
                diners / 3,
                2_200,
            ));
            account.settle = Some((date, PaymentMethod::Card));
        }
        DemoEvent {
            event,
            organiser,
            suite_bookings: suites,
            account,
        }
    }

    fn festive(&mut self, date: NaiveDate, ballroom: SpaceId, new_year: bool) -> DemoEvent {
        let organiser = self.organiser(Some("events@lanterns-hotel.example.com".into()));
        let diners = self.rng.between(90, 118) as u16;
        let (name, hours, price, sitting) = if new_year {
            (
                format!("New Year's Eve dinner dance {}", date.year()),
                (at(date, 19, 0), at(add_days(date, 1), 1, 0)),
                95,
                ("Dinner", 20),
            )
        } else {
            (
                format!("Christmas Day lunch {}", date.year()),
                (at(date, 12, 0), at(date, 16, 0)),
                75,
                ("Christmas lunch", 13),
            )
        };
        let event = Event {
            id: EventId(self.rng.uuid()),
            name,
            kind: EventKind::Dinner,
            organiser_id: organiser.id,
            start_date: date,
            end_date: hours.1.date(),
            notes: "Tickets sold at reception; residents first.".into(),
            running_order: if new_year {
                "19:00 Fizz\n20:00 Dinner\n22:00 Band\n00:00 Midnight on the terrace".into()
            } else {
                "12:00 Mulled wine\n13:00 Lunch\n15:00 The King's speech".into()
            },
            sittings: vec![Self::sitting(
                sitting.0,
                date,
                sitting.1,
                diners,
                diners / 8,
                "",
            )],
            room_block: None,
        };
        let suites = vec![self.suite_booking(
            ballroom,
            &event,
            organiser.id,
            &event.name.clone(),
            hours,
            Layout::Banquet,
            (diners + diners / 8).min(120),
            SuitePricing::PerHead {
                price: Money::from_major(price),
            },
            SuiteBookingStatus::Confirmed,
        )];
        DemoEvent {
            event,
            organiser,
            suite_bookings: suites,
            account: DemoAccount::default(),
        }
    }

    // ---- reports ------------------------------------------------------------

    /// What a manager might have built: the figures for a Monday meeting.
    fn reports(&mut self) -> (Vec<SavedReport>, Vec<Dashboard>) {
        use ChartKind::{Bar, Line, Number, Table};
        let specs = [
            (
                "Occupancy this month",
                Measure::Occupancy,
                Split::None,
                Period::ThisMonth,
                Number,
            ),
            (
                "Average room rate this month",
                Measure::Adr,
                Split::None,
                Period::ThisMonth,
                Number,
            ),
            (
                "Occupancy by month",
                Measure::Occupancy,
                Split::Month,
                Period::Last12Months,
                Line,
            ),
            (
                "Revenue by type, this year",
                Measure::TotalRevenue,
                Split::RevenueType,
                Period::ThisYear,
                Bar,
            ),
            (
                "Where bookings come from",
                Measure::BookingsMade,
                Split::Source,
                Period::Last12Months,
                Bar,
            ),
            (
                "Arrivals in the next 30 days",
                Measure::Arrivals,
                Split::Day,
                Period::Next30Days,
                Bar,
            ),
            (
                "Room revenue by room type, last month",
                Measure::RoomRevenue,
                Split::RoomType,
                Period::LastMonth,
                Bar,
            ),
            (
                "Payments by method, last month",
                Measure::PaymentsTaken,
                Split::PaymentMethod,
                Period::LastMonth,
                Table,
            ),
        ];
        let reports: Vec<SavedReport> = specs
            .iter()
            .map(|&(name, measure, split, period, chart)| SavedReport {
                id: SavedReportId(self.rng.uuid()),
                spec: ReportSpec {
                    name: name.into(),
                    measure,
                    split,
                    period,
                    chart,
                },
            })
            .collect();
        // The two figures side by side, then the charts; time charts full width.
        let items = [
            (0, false),
            (1, false),
            (2, true),
            (3, false),
            (4, false),
            (5, true),
        ]
        .iter()
        .map(|&(i, wide)| DashboardItem {
            report_id: reports[i].id,
            wide,
        })
        .collect();
        let dashboard = Dashboard {
            id: DashboardId(self.rng.uuid()),
            name: "Monday meeting".into(),
            items,
        };
        (reports, vec![dashboard])
    }

    // ---- this morning -------------------------------------------------------

    /// By mid-morning housekeeping has done some of the rooms: some stay-overs
    /// are made up and some departed rooms are clean or already inspected.
    fn housekeeping(&mut self, stays: &[DemoStay]) -> Vec<(SpaceId, HousekeepingStatus)> {
        let yesterday = add_days(self.today, -1);
        let mut marks = Vec::new();
        for s in stays {
            if !s.booking.status.holds_rooms() {
                continue;
            }
            for room in &s.booking.rooms {
                let slept_here = room.stay.start <= yesterday && yesterday < room.stay.end;
                if !slept_here {
                    continue;
                }
                let leaving = room.stay.end == self.today;
                let status = if leaving && s.booking.status == BookingStatus::CheckedOut {
                    self.rng.weighted(&[
                        (Some(HousekeepingStatus::Clean), 35),
                        (Some(HousekeepingStatus::Inspected), 15),
                        (None, 50),
                    ])
                } else if !leaving {
                    self.rng
                        .weighted(&[(Some(HousekeepingStatus::Clean), 35), (None, 65)])
                } else {
                    None
                };
                if let Some(status) = status {
                    marks.push((room.space_id, status));
                }
            }
        }
        marks
    }
}

// ---------------------------------------------------------------------------
// The room calendar
// ---------------------------------------------------------------------------

struct CalRoom {
    id: SpaceId,
    room_type: RoomType,
    pets: bool,
    in_service: bool,
}

/// Which bedroom is taken on which night, so no two stays ever share a room.
struct Calendar {
    rooms: Vec<CalRoom>,
    taken: Vec<Vec<bool>>,
}

impl Calendar {
    fn new(g: &Gen, spaces: &[Space], types: &[RoomType]) -> Self {
        let rooms: Vec<CalRoom> = spaces
            .iter()
            .filter(|s| s.kind == SpaceKind::Room)
            .filter_map(|s| {
                let room_type = types.iter().find(|t| Some(t.id) == s.room_type_id)?;
                Some(CalRoom {
                    id: s.id,
                    room_type: room_type.clone(),
                    pets: s.features.contains(&RoomFeature::PetsAllowed),
                    in_service: !s.out_of_service,
                })
            })
            .collect();
        let taken = vec![vec![false; g.days]; rooms.len()];
        Self { rooms, taken }
    }

    fn bookable(&self) -> Vec<usize> {
        (0..self.rooms.len())
            .filter(|&r| self.rooms[r].in_service)
            .collect()
    }

    fn taken(&self, room: usize, day: usize) -> bool {
        self.taken[room][day]
    }

    /// How many nights from `day` the room is free, up to `wanted`.
    fn free_run(&self, room: usize, day: usize, wanted: usize) -> usize {
        self.taken[room][day..]
            .iter()
            .take(wanted)
            .take_while(|t| !**t)
            .count()
    }

    fn take(&mut self, room: usize, day: usize, nights: usize) {
        for t in self.taken[room][day..].iter_mut().take(nights) {
            *t = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 11, 22).unwrap()
    }

    fn hotel() -> DemoHotel {
        demo_hotel(DEMO_SEED, today())
    }

    #[test]
    fn same_seed_and_day_give_the_same_hotel() {
        let a = serde_json::to_string(&hotel()).unwrap();
        let b = serde_json::to_string(&hotel()).unwrap();
        assert_eq!(a, b);
        let other = serde_json::to_string(&demo_hotel(DEMO_SEED + 1, today())).unwrap();
        assert_ne!(a, other);
    }

    #[test]
    fn the_building_has_forty_rooms_and_two_suites() {
        let h = hotel();
        assert_eq!(h.settings.hotel_name, "The Lanterns Hotel");
        assert_eq!(h.floors.len(), 3);
        assert_eq!(h.room_types.len(), 6);
        let rooms = h
            .spaces
            .iter()
            .filter(|s| s.kind == SpaceKind::Room)
            .count();
        let suites = h
            .spaces
            .iter()
            .filter(|s| s.kind == SpaceKind::Suite)
            .count();
        assert_eq!((rooms, suites), (40, 2));
        assert!(
            h.spaces
                .iter()
                .filter(|s| s.kind == SpaceKind::Room)
                .all(|s| s.features.contains(&RoomFeature::Wifi))
        );
        assert_eq!(h.spaces.iter().filter(|s| s.out_of_service).count(), 1);
        let floor = &h.floors[0];
        for s in &h.spaces {
            crate::validate_space(s, floor, &h.spaces).unwrap();
        }
    }

    #[test]
    fn no_room_is_ever_booked_twice_on_a_night() {
        let h = hotel();
        let mut nights: HashSet<(SpaceId, NaiveDate)> = HashSet::new();
        for s in &h.stays {
            if !s.booking.status.holds_rooms() {
                continue;
            }
            for r in &s.booking.rooms {
                for night in r.stay.nights() {
                    assert!(
                        nights.insert((r.space_id, night)),
                        "double booking on {night}"
                    );
                }
            }
        }
        let out = h.spaces.iter().find(|s| s.out_of_service).unwrap().id;
        assert!(
            !nights.iter().any(|(id, _)| *id == out),
            "nobody in the room being refitted"
        );
    }

    #[test]
    fn bookings_cover_history_and_future_with_every_status_and_source() {
        let h = hotel();
        let t = today();
        assert!(
            h.stays.len() > 3_000,
            "a busy hotel: {} stays",
            h.stays.len()
        );
        let first = h.stays.iter().map(|s| arrival(&s.booking)).min().unwrap();
        let last = h.stays.iter().map(|s| arrival(&s.booking)).max().unwrap();
        assert!(first <= add_days(t, -500));
        assert!(last >= add_days(t, 120));
        let statuses: HashSet<_> = h.stays.iter().map(|s| s.booking.status).collect();
        let sources: HashSet<_> = h.stays.iter().map(|s| s.booking.source.as_str()).collect();
        assert_eq!(statuses.len(), 6, "{statuses:?}");
        assert_eq!(sources.len(), 6, "{sources:?}");
        assert!(
            h.stays.iter().all(|s| s.booked_on <= t),
            "nothing booked in the future"
        );
        // Today: guests in the house and people arriving.
        assert!(
            h.stays
                .iter()
                .any(|s| s.booking.status == BookingStatus::CheckedIn)
        );
        assert!(
            h.stays
                .iter()
                .any(|s| arrival(&s.booking) == t && s.booking.status == BookingStatus::Confirmed)
        );
        // Some multi-room bookings, parking on about a third.
        assert!(h.stays.iter().any(|s| s.booking.rooms.len() > 1));
        let rooms: Vec<&BookingRoom> = h.stays.iter().flat_map(|s| &s.booking.rooms).collect();
        let with_cars = rooms.iter().filter(|r| r.cars > 0).count() * 100 / rooms.len();
        assert!((25..=45).contains(&with_cars), "{with_cars}% park");
    }

    #[test]
    fn summers_are_busier_than_januaries() {
        let h = hotel();
        let mut by_month: HashMap<(i32, u32), u32> = HashMap::new();
        for s in &h.stays {
            if !s.booking.status.holds_rooms() {
                continue;
            }
            for r in &s.booking.rooms {
                for night in r.stay.nights() {
                    *by_month.entry((night.year(), night.month())).or_default() += 1;
                }
            }
        }
        let august = by_month[&(2026, 8)];
        let january = by_month[&(2026, 1)];
        assert!(august > january * 2, "August {august} vs January {january}");
    }

    #[test]
    fn some_guests_come_back() {
        let h = hotel();
        let mut count: HashMap<GuestId, u32> = HashMap::new();
        for s in &h.stays {
            *count.entry(s.booking.guest_id).or_default() += 1;
        }
        let returning = count.values().filter(|&&n| n > 1).count();
        assert!(returning > 200, "{returning} returning guests");
        let ids: HashSet<GuestId> = h.guests.iter().map(|g| g.id).collect();
        assert!(h.stays.iter().all(|s| ids.contains(&s.booking.guest_id)));
        // Everyone has a different email, apart from the one duplicate with none.
        let with_email: Vec<&Guest> = h.guests.iter().filter(|g| !g.email.is_empty()).collect();
        let emails: HashSet<&str> = with_email.iter().map(|g| g.email.as_str()).collect();
        assert_eq!(emails.len(), with_email.len(), "every email is different");
        assert_eq!(with_email.len() + 1, h.guests.len());
        for g in with_email {
            assert!(
                g.email.contains("@") && g.email.contains("example."),
                "{}",
                g.email
            );
            assert!(g.phone.starts_with("07700 900") || g.phone.starts_with("01632 960"));
        }
    }

    #[test]
    fn events_include_weddings_conferences_and_the_festive_season() {
        let h = hotel();
        let kinds: HashSet<_> = h.events.iter().map(|e| e.event.kind.as_str()).collect();
        assert!(kinds.contains("wedding"));
        assert!(kinds.contains("conference"));
        assert!(kinds.contains("dinner"));
        assert!(
            h.events
                .iter()
                .any(|e| e.event.name.starts_with("Christmas Day"))
        );
        assert!(
            h.events.iter().any(|e| e.event.room_block.is_some()),
            "a future wedding holds rooms"
        );
        for e in &h.events {
            crate::validate_event(&e.event).unwrap();
            for b in &e.suite_bookings {
                let suite = h.spaces.iter().find(|s| s.id == b.suite_id).unwrap();
                crate::validate_suite_booking(b, suite, &[]).unwrap();
            }
        }
        // No suite is double-booked.
        let all: Vec<SuiteBooking> = h
            .events
            .iter()
            .flat_map(|e| e.suite_bookings.clone())
            .collect();
        for b in &all {
            let suite = h.spaces.iter().find(|s| s.id == b.suite_id).unwrap();
            crate::validate_suite_booking(b, suite, &all).unwrap();
        }
    }

    #[test]
    fn coach_tours_come_through_operators_most_of_the_year() {
        let h = hotel();
        let tours: Vec<&DemoStay> = h
            .stays
            .iter()
            .filter(|s| s.booking.notes.contains(" tour: "))
            .collect();
        assert!(tours.len() >= 15, "{} tours", tours.len());
        let months: HashSet<u32> = tours.iter().map(|t| arrival(&t.booking).month()).collect();
        assert!(months.len() >= 7, "{months:?}");
        let operators: HashSet<GuestId> = tours.iter().map(|t| t.booking.guest_id).collect();
        assert!(
            operators.len() >= 3 && operators.len() <= 4,
            "the same tour managers return"
        );
        for t in &tours {
            assert_eq!(t.booking.source, BookingSource::Agent);
            assert!(t.booking.rooms.len() >= 7);
            assert!(t.booking.rooms.iter().all(|r| r.price_override.is_some()));
            assert!(
                t.booking
                    .rooms
                    .iter()
                    .any(|r| r.price_override == Some(Money::ZERO)),
                "the driver stays free"
            );
        }
    }

    #[test]
    fn bills_have_extras_deposits_and_every_payment_method() {
        let h = hotel();
        let methods: HashSet<_> = h
            .stays
            .iter()
            .filter_map(|s| s.account.settle.map(|(_, m)| m.as_str()))
            .collect();
        assert_eq!(methods.len(), 6, "{methods:?}");
        assert!(h.stays.iter().any(|s| s.account.deposit.is_some()));
        assert!(h.stays.iter().any(|s| s.account.invoice_on.is_some()));
        assert!(h.stays.iter().any(|s| {
            s.account
                .extras
                .iter()
                .any(|c| c.category == ChargeCategory::Drink)
        }));
        for s in &h.stays {
            for c in &s.account.extras {
                crate::validate_charge(c).unwrap();
                assert!(c.date <= today());
            }
            if s.account.settle.is_some() {
                assert_eq!(s.booking.status, BookingStatus::CheckedOut);
            }
        }
    }

    #[test]
    fn housekeeping_has_done_some_rooms_this_morning() {
        let h = hotel();
        assert!(!h.housekeeping.is_empty());
        let rooms: HashSet<SpaceId> = h.housekeeping.iter().map(|(id, _)| *id).collect();
        assert_eq!(rooms.len(), h.housekeeping.len(), "each room once");
    }

    #[test]
    fn saved_reports_and_the_dashboard_are_valid() {
        let h = hotel();
        assert!(h.saved_reports.len() >= 6);
        for r in &h.saved_reports {
            crate::validate_report_spec(&r.spec).unwrap();
        }
        for d in &h.dashboards {
            crate::validate_dashboard(d).unwrap();
            assert!(
                d.items
                    .iter()
                    .all(|i| h.saved_reports.iter().any(|r| r.id == i.report_id))
            );
        }
    }

    #[test]
    fn a_regular_has_a_duplicate_record_to_merge() {
        let h = hotel();
        let named = |first: &str| {
            h.guests
                .iter()
                .find(|g| g.first_name == first && g.last_name == "Trevithick")
        };
        let margaret = named("Margaret").expect("Margaret Trevithick");
        let maggie = named("Maggie").expect("Maggie Trevithick");
        let stays = |id: GuestId| h.stays.iter().filter(|s| s.booking.guest_id == id).count();
        assert!(stays(margaret.id) >= 1 && stays(maggie.id) == 1);
        assert!(maggie.email.is_empty());
    }

    #[test]
    fn rng_is_stable() {
        let mut r = DemoRng::new(1);
        let first: Vec<u64> = (0..3).map(|_| r.next_u64()).collect();
        let mut again = DemoRng::new(1);
        assert_eq!(first, (0..3).map(|_| again.next_u64()).collect::<Vec<_>>());
        assert!((0..1000).all(|_| r.below(7) < 7));
        assert_eq!(r.below(0), 0);
    }
}
