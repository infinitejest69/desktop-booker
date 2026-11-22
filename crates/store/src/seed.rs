//! Demo data so there is something to look at during development.

use booker_core::{
    Adjustment, BoardOption, BoardOptionId, Booking, BookingId, BookingRoom, BookingRoomId,
    BookingSource, BookingStatus, CateringSitting, DateRange, Event, EventId, EventKind, Floor,
    FloorId, GridRect, Guest, GuestId, Layout, Money, RateRule, RateRuleId, RoomBlock, RoomFeature,
    RoomType, RoomTypeId, Space, SpaceId, SpaceKind, SuiteBooking, SuiteBookingId,
    SuiteBookingStatus, SuiteCapacities, SuitePricing, SuiteRates, ValidationError,
};
use chrono::{Datelike, Days, Local, NaiveDate, NaiveDateTime, NaiveTime, Weekday};

use crate::StoreError;
use crate::repo::{
    BookingRepo, EventRepo, FloorRepo, InvoiceRepo, RateRepo, RoomTypeRepo, SpaceRepo, SuiteRepo,
};
use crate::{SqliteStore, StoreResult};

const COLS: u16 = 24;
const ROWS: u16 = 10;
/// Rooms are 3×4 cells: a row of 8 along the top (y=0) and bottom (y=6),
/// with a corridor (y=4..6) between them.
const ROOM_W: u16 = 3;
const ROOM_H: u16 = 4;

impl SqliteStore {
    /// Fills an empty database with a 3-floor demo hotel (40 rooms, 2 suites).
    /// Returns `false` and does nothing if any floors already exist.
    pub async fn seed_demo(&self) -> StoreResult<bool> {
        if !self.list_floors().await?.is_empty() {
            return Ok(false);
        }

        let types = [
            ("Single", "SGL", 1, 1, 65),
            ("Double", "DBL", 2, 3, 95),
            ("Twin", "TWN", 2, 2, 95),
            ("Family", "FAM", 3, 4, 140),
        ]
        .map(
            |(name, code, default_occupancy, max_occupancy, rate)| RoomType {
                id: RoomTypeId::new(),
                name: name.into(),
                code: code.into(),
                default_occupancy,
                max_occupancy,
                description: String::new(),
                base_rate: Money::from_major(rate),
                extra_adult_rate: Money::from_major(25),
                child_rate: Money::from_major(15),
            },
        );
        for rt in &types {
            self.save_room_type(rt).await?;
        }

        let room = |floor: &Floor, number: String, slot: u16, top: bool, type_ix: usize| Space {
            id: SpaceId::new(),
            floor_id: floor.id,
            kind: SpaceKind::Room,
            name: number,
            room_type_id: Some(types[type_ix % types.len()].id),
            suite_capacities: SuiteCapacities::default(),
            rect: GridRect::new(
                slot * ROOM_W,
                if top { 0 } else { ROWS - ROOM_H },
                ROOM_W,
                ROOM_H,
            ),
            out_of_service: false,
            features: Vec::new(),
            notes: String::new(),
        };

        for (level, name) in [(0, "Ground floor"), (1, "First floor"), (2, "Second floor")] {
            let floor = Floor {
                id: FloorId::new(),
                name: name.into(),
                level,
                grid_cols: COLS,
                grid_rows: ROWS,
            };
            self.save_floor(&floor).await?;

            // Ground floor: 8 rooms on top, two function suites below.
            // Upper floors: 16 rooms each, numbered 101–116, 201–216.
            let slots_per_side = COLS / ROOM_W;
            let mut number: u16 = 1;
            let sides: &[bool] = if level == 0 { &[true] } else { &[true, false] };
            for &top in sides {
                for slot in 0..slots_per_side {
                    let label = if level == 0 {
                        format!("G{number:02}")
                    } else {
                        format!("{level}{number:02}")
                    };
                    let mut space = room(&floor, label, slot, top, usize::from(number));
                    space.features = demo_features(level, number);
                    self.save_space(&space).await?;
                    number += 1;
                }
            }
            if level == 0 {
                for (name, x, w, caps) in [
                    (
                        "Garden Suite",
                        0,
                        14,
                        SuiteCapacities {
                            theatre: Some(150),
                            banquet: Some(110),
                            cabaret: Some(80),
                            reception: Some(200),
                            ..Default::default()
                        },
                    ),
                    (
                        "Oak Room",
                        14,
                        10,
                        SuiteCapacities {
                            theatre: Some(40),
                            boardroom: Some(20),
                            banquet: Some(30),
                            ..Default::default()
                        },
                    ),
                ] {
                    self.save_space(&Space {
                        id: SpaceId::new(),
                        floor_id: floor.id,
                        kind: SpaceKind::Suite,
                        name: name.into(),
                        room_type_id: None,
                        suite_capacities: caps,
                        rect: GridRect::new(x, ROWS - ROOM_H, w, ROOM_H),
                        out_of_service: false,
                        features: Vec::new(),
                        notes: String::new(),
                    })
                    .await?;
                }
            }
        }
        self.seed_rates_and_bookings().await?;
        self.seed_demo_events().await?;
        self.seed_demo_invoicing().await?;
        Ok(true)
    }

    /// Adds UK VAT rates, invoice settings and a few bar charges on in-house
    /// guests' bills. Does nothing (returns `false`) if tax rates exist.
    pub async fn seed_demo_invoicing(&self) -> StoreResult<bool> {
        if !self.list_tax_rates().await?.is_empty() {
            return Ok(false);
        }
        let rate = |name: &str, basis_points| booker_core::TaxRate {
            id: booker_core::TaxRateId::new(),
            name: name.into(),
            basis_points,
        };
        let standard = rate("Standard VAT", 2000);
        for r in [&standard, &rate("Reduced VAT", 500), &rate("Zero rated", 0)] {
            self.save_tax_rate(r).await?;
        }
        let all = Some(standard.id);
        self.save_invoice_settings(&booker_core::InvoiceSettings {
            prices_include_tax: true,
            prefix: "INV-".into(),
            footer: "VAT no. GB 123 4567 89\nBank transfer: The Grange Ltd, sort code 12-34-56, \
                     account 12345678\nThank you for staying with us."
                .into(),
            accommodation_rate: all,
            board_rate: all,
            suite_rate: all,
            food_rate: all,
            drink_rate: all,
            other_rate: all,
        })
        .await?;

        // A bar tab for a few guests who are in the house tonight.
        let today = Local::now().date_naive();
        let in_house: Vec<_> = self
            .chart(today, today.succ_opt().unwrap_or(today))
            .await?
            .into_iter()
            .filter(|e| e.status == BookingStatus::CheckedIn)
            .take(4)
            .collect();
        for (i, e) in in_house.iter().enumerate() {
            let owner = booker_core::FolioOwner::Booking(e.booking_id);
            for (what, category, pence) in [
                (
                    "Bar",
                    booker_core::ChargeCategory::Drink,
                    1_450 + 300 * i as i64,
                ),
                ("Dinner", booker_core::ChargeCategory::Food, 5_200),
            ] {
                self.add_charge(
                    owner,
                    &booker_core::Charge {
                        id: booker_core::ChargeId::new(),
                        date: today,
                        category,
                        description: what.into(),
                        quantity: 1,
                        unit_price: Money(pence),
                        tax_basis_points: 2000,
                    },
                )
                .await?;
            }
        }
        Ok(true)
    }

    /// Adds demo suite rates, a wedding, a conference and a club dinner to
    /// the demo hotel. Returns `false` and does nothing if any event exists
    /// or the demo suites aren't there.
    pub async fn seed_demo_events(&self) -> StoreResult<bool> {
        if !self.list_events(NaiveDate::MIN).await?.is_empty() {
            return Ok(false);
        }
        let spaces = self.list_spaces(None).await?;
        let suite = |name: &str| {
            spaces
                .iter()
                .find(|s| s.kind == SpaceKind::Suite && s.name == name)
                .map(|s| s.id)
        };
        let (Some(garden), Some(oak)) = (suite("Garden Suite"), suite("Oak Room")) else {
            return Ok(false);
        };
        for (id, hourly, min_hours, half, full) in
            [(garden, 120, 3, 450, 800), (oak, 45, 2, 150, 280)]
        {
            self.save_suite_rates(
                id,
                &SuiteRates {
                    hourly: Some(Money::from_major(hourly)),
                    min_hours,
                    half_day: Some(Money::from_major(half)),
                    full_day: Some(Money::from_major(full)),
                    setup_minutes: 60,
                    cleardown_minutes: 30,
                },
            )
            .await?;
        }

        let today = Local::now().date_naive();
        let at = |day: NaiveDate, h: u32, m: u32| -> NaiveDateTime {
            day.and_time(NaiveTime::from_hms_opt(h, m, 0).unwrap_or_default())
        };
        let day_in = |days: u64| today.checked_add_days(Days::new(days)).unwrap_or(today);
        // The wedding is on a Saturday about five weeks away.
        let mut wedding_day = day_in(35);
        while wedding_day.weekday() != Weekday::Sat {
            wedding_day = wedding_day.succ_opt().unwrap_or(wedding_day);
        }
        let mut conference_day = day_in(10);
        while matches!(conference_day.weekday(), Weekday::Sat | Weekday::Sun) {
            conference_day = conference_day.succ_opt().unwrap_or(conference_day);
        }
        let dinner_day = day_in(1);
        let double = self
            .list_room_types()
            .await?
            .into_iter()
            .find(|t| t.code == "DBL")
            .map(|t| t.id);

        let contact = |first: &str, last: &str, email: &str| Guest {
            id: GuestId::new(),
            first_name: first.into(),
            last_name: last.into(),
            email: email.into(),
            phone: "07700 900123".into(),
            address: String::new(),
            notes: String::new(),
        };
        let sitting =
            |name: &str, day: NaiveDate, h: u32, adults: u16, children: u16, dietary: &str| {
                CateringSitting {
                    name: name.into(),
                    date: day,
                    time: NaiveTime::from_hms_opt(h, 0, 0).unwrap_or_default(),
                    adults,
                    children,
                    dietary: dietary.into(),
                }
            };

        // A wedding with a block of double rooms for guests.
        let ffion = contact("Ffion", "Hughes", "ffion.hughes@example.com");
        let mut wedding = Event {
            id: EventId::new(),
            name: "Hughes–Price wedding".into(),
            kind: EventKind::Wedding,
            organiser_id: ffion.id,
            start_date: wedding_day,
            end_date: wedding_day,
            notes: "Bride's mother (Carys) is the contact on the day. Band brings own PA.".into(),
            running_order: "12:30 Guests arrive, Oak Room\n13:00 Ceremony\n\
                            14:00 Drinks on the lawn\n15:00 Wedding breakfast, Garden Suite\n\
                            17:00 Speeches\n19:30 Evening guests and buffet\n00:00 Carriages"
                .into(),
            sittings: vec![
                sitting(
                    "Wedding breakfast",
                    wedding_day,
                    15,
                    86,
                    8,
                    "6 vegetarian\n2 vegan\n1 coeliac\n1 nut allergy (table 4)",
                ),
                sitting(
                    "Evening buffet",
                    wedding_day,
                    20,
                    120,
                    10,
                    "Label all allergens",
                ),
            ],
            room_block: DateRange::new(wedding_day, wedding_day.succ_opt().unwrap_or(wedding_day))
                .ok()
                .map(|stay| RoomBlock {
                    room_type_id: double,
                    rooms: 8,
                    stay,
                    rate: Money::from_major(85),
                    release_date: wedding_day
                        .checked_sub_days(Days::new(21))
                        .unwrap_or(wedding_day),
                }),
        };
        // The demo hotel is busy: hold as many of the eight as are free.
        loop {
            match self.save_event(&wedding, &ffion, today).await {
                Ok(_) => break,
                Err(StoreError::Validation(ValidationError::NotEnoughRooms { free, .. })) => {
                    if let Some(b) = wedding.room_block.as_mut() {
                        let taken = b.rooms;
                        b.rooms = free.min(taken.saturating_sub(1));
                    }
                    if wedding.room_block.is_some_and(|b| b.rooms == 0) {
                        wedding.room_block = None;
                    }
                }
                Err(e) => return Err(e),
            }
        }
        let suite_booking = |suite: SpaceId,
                             event: &Event,
                             guest: &Guest,
                             title: &str,
                             start: NaiveDateTime,
                             end: NaiveDateTime,
                             layout: Layout,
                             attendees: u16,
                             pricing: SuitePricing| SuiteBooking {
            id: SuiteBookingId::new(),
            suite_id: suite,
            event_id: Some(event.id),
            guest_id: guest.id,
            title: title.into(),
            start,
            end,
            layout,
            attendees,
            setup_minutes: 60,
            cleardown_minutes: 30,
            pricing,
            status: SuiteBookingStatus::Confirmed,
            notes: String::new(),
        };
        for b in [
            suite_booking(
                oak,
                &wedding,
                &ffion,
                "Ceremony",
                at(wedding_day, 12, 30),
                at(wedding_day, 14, 0),
                Layout::Theatre,
                40,
                SuitePricing::Hourly,
            ),
            suite_booking(
                garden,
                &wedding,
                &ffion,
                "Wedding breakfast and evening party",
                at(wedding_day, 15, 0),
                at(wedding_day, 23, 59),
                Layout::Banquet,
                94,
                SuitePricing::PerHead {
                    price: Money::from_major(55),
                },
            ),
        ] {
            self.save_suite_booking(&b, &ffion).await?;
        }

        // A one-day conference across both suites.
        let priya = contact("Priya", "Shah", "events@coastaltech.example.com");
        let conference = Event {
            id: EventId::new(),
            name: "Coastal Tech Summit".into(),
            kind: EventKind::Conference,
            organiser_id: priya.id,
            start_date: conference_day,
            end_date: conference_day,
            notes: "Invoice to Coastal Tech Ltd. Projector and two lapel mics.".into(),
            running_order: "08:30 Registration and coffee\n09:00 Keynote\n\
                            12:30 Lunch\n13:30 Breakouts (Oak Room)\n17:00 Close"
                .into(),
            sittings: vec![sitting(
                "Buffet lunch",
                conference_day,
                12,
                110,
                0,
                "12 vegetarian\n3 halal",
            )],
            room_block: None,
        };
        self.save_event(&conference, &priya, today).await?;
        for b in [
            suite_booking(
                garden,
                &conference,
                &priya,
                "Main sessions",
                at(conference_day, 8, 30),
                at(conference_day, 17, 0),
                Layout::Theatre,
                110,
                SuitePricing::FullDay,
            ),
            suite_booking(
                oak,
                &conference,
                &priya,
                "Breakouts",
                at(conference_day, 13, 30),
                at(conference_day, 17, 0),
                Layout::Boardroom,
                20,
                SuitePricing::HalfDay,
            ),
        ] {
            self.save_suite_booking(&b, &priya).await?;
        }

        // A club dinner tomorrow evening.
        let gareth = contact("Gareth", "Lewis", "secretary@rotary.example.com");
        let dinner = Event {
            id: EventId::new(),
            name: "Rotary Club dinner".into(),
            kind: EventKind::Dinner,
            organiser_id: gareth.id,
            start_date: dinner_day,
            end_date: dinner_day,
            notes: "Guest speaker needs a lectern.".into(),
            running_order: "19:00 Drinks\n19:30 Dinner\n21:00 Speaker".into(),
            sittings: vec![sitting("Dinner", dinner_day, 19, 28, 0, "2 vegetarian")],
            room_block: None,
        };
        self.save_event(&dinner, &gareth, today).await?;
        let mut club = suite_booking(
            oak,
            &dinner,
            &gareth,
            "Dinner",
            at(dinner_day, 19, 0),
            at(dinner_day, 23, 0),
            Layout::Banquet,
            28,
            SuitePricing::PerHead {
                price: Money::from_major(35),
            },
        );
        club.status = SuiteBookingStatus::Provisional;
        self.save_suite_booking(&club, &gareth).await?;
        Ok(true)
    }

    async fn seed_rates_and_bookings(&self) -> StoreResult<()> {
        let boards = [
            ("RO", "Room only", 0, 0),
            ("BB", "Bed & breakfast", 12, 7),
            ("DBB", "Dinner, bed & breakfast", 40, 20),
        ]
        .map(|(code, name, adult, child)| BoardOption {
            id: BoardOptionId::new(),
            code: code.into(),
            name: name.into(),
            adult_price: Money::from_major(adult),
            child_price: Money::from_major(child),
        });
        for b in &boards {
            self.save_board_option(b).await?;
        }

        let today = Local::now().date_naive();
        let year = today.year();
        let ymd = |y: i32, m: u32, d: u32| NaiveDate::from_ymd_opt(y, m, d).unwrap_or(today);
        let rules = [
            (
                "Weekend",
                ymd(year - 1, 1, 1),
                ymd(year + 1, 12, 31),
                vec![5, 6],
                10,
                Adjustment::AddAmount {
                    amount: Money::from_major(20),
                },
            ),
            (
                "Summer season",
                ymd(year, 7, 1),
                ymd(year, 8, 31),
                vec![],
                5,
                Adjustment::Percent { percent: 15 },
            ),
            (
                "Christmas & New Year",
                ymd(year, 12, 20),
                ymd(year + 1, 1, 2),
                vec![],
                5,
                Adjustment::Percent { percent: 25 },
            ),
            (
                "Winter midweek offer",
                ymd(year + 1, 1, 5),
                ymd(year + 1, 2, 28),
                vec![1, 2, 3, 4],
                20,
                Adjustment::Percent { percent: -10 },
            ),
        ];
        for (name, start, end, weekdays, priority, adjustment) in rules {
            self.save_rate_rule(&RateRule {
                id: RateRuleId::new(),
                name: name.into(),
                room_type_id: None,
                start,
                end,
                weekdays,
                priority,
                adjustment,
                active: true,
            })
            .await?;
        }

        let room_types = self.list_room_types().await?;
        let rooms: Vec<Space> = self
            .list_spaces(None)
            .await?
            .into_iter()
            .filter(|s| s.kind == SpaceKind::Room)
            .collect();
        let mut rng = DemoRng(0x5EED_B00C);

        // ~60 attempted bookings from two weeks ago to six weeks ahead. Any that
        // would clash with one already placed are simply skipped.
        for n in 0..90 {
            let offset = i64::from(rng.below(60)) - 14;
            // Mostly short breaks, sometimes a week. Pick the range first: `rng` can't be
            // borrowed mutably twice in one expression.
            let max_nights = if rng.below(4) == 0 { 7 } else { 3 };
            let nights = 1 + u64::from(rng.below(max_nights));
            let Some(start) = shift(today, offset) else {
                continue;
            };
            let Some(end) = start.checked_add_days(Days::new(nights)) else {
                continue;
            };
            let stay = DateRange { start, end };

            let room_count = if rng.below(10) == 0 { 2 } else { 1 };
            let mut booked_rooms = Vec::new();
            for _ in 0..room_count {
                let space = &rooms[rng.below(rooms.len() as u32) as usize];
                if booked_rooms
                    .iter()
                    .any(|r: &BookingRoom| r.space_id == space.id)
                {
                    continue;
                }
                let max = space_max(space, &room_types);
                let adults = if max == 1 {
                    1
                } else {
                    1 + rng.below(2).min(u32::from(max) - 1) as u8
                };
                let children = if max > adults {
                    rng.below(u32::from(max - adults) + 1) as u8
                } else {
                    0
                };
                booked_rooms.push(BookingRoom {
                    id: BookingRoomId::new(),
                    space_id: space.id,
                    stay,
                    adults,
                    children,
                    // Mostly B&B, some dinner too; room only is no board at all,
                    // as the booking form treats a free board.
                    board_option_id: [Some(1), Some(1), Some(1), Some(2), Some(2), None]
                        [rng.below(6) as usize]
                        .map(|i| boards[i].id),
                    price_override: None,
                    cars: 0,
                });
            }

            let status = if end <= today {
                if rng.below(20) == 0 {
                    BookingStatus::NoShow
                } else {
                    BookingStatus::CheckedOut
                }
            } else if start < today {
                BookingStatus::CheckedIn
            } else if rng.below(15) == 0 {
                BookingStatus::Cancelled
            } else if rng.below(7) == 0 {
                BookingStatus::Provisional
            } else {
                BookingStatus::Confirmed
            };

            let (first, last) = (
                FIRST_NAMES[rng.below(FIRST_NAMES.len() as u32) as usize],
                LAST_NAMES[n % LAST_NAMES.len()],
            );
            let guest = Guest {
                id: GuestId::new(),
                first_name: first.into(),
                last_name: last.into(),
                email: format!(
                    "{}.{}@example.com",
                    first.to_lowercase(),
                    last.to_lowercase()
                ),
                phone: format!("07700 900{:03}", rng.below(1000)),
                address: String::new(),
                notes: String::new(),
            };
            let booking = Booking {
                id: BookingId::new(),
                reference: String::new(),
                guest_id: guest.id,
                status,
                source: BookingSource::ALL[rng.below(6) as usize],
                notes: NOTES[rng.below(NOTES.len() as u32) as usize].into(),
                rooms: booked_rooms,
            };
            // A clash with an earlier demo booking isn't an error here; just skip it.
            let _ = self.save_booking(&booking, &guest).await;
        }
        Ok(())
    }
}

const FIRST_NAMES: [&str; 20] = [
    "Margaret", "Tom", "Aisha", "David", "Sofia", "James", "Priya", "Gareth", "Chloe", "Oliver",
    "Fatima", "Rhys", "Hannah", "Liam", "Mei", "Owen", "Grace", "Arjun", "Ellie", "Dylan",
];

const LAST_NAMES: [&str; 25] = [
    "Hughes", "Price", "Khan", "Lewis", "Rossi", "Evans", "Patel", "Jones", "Morgan", "Davies",
    "Williams", "Thomas", "Roberts", "Walker", "Chen", "Wright", "Taylor", "Murphy", "Novak",
    "Ahmed", "Clarke", "Bennett", "Owen", "Hall", "Fisher",
];

const NOTES: [&str; 8] = [
    "",
    "",
    "",
    "Late arrival, after 9pm",
    "Anniversary: flowers in room",
    "Dog-friendly room please",
    "Ground floor if possible",
    "Gluten-free breakfast",
];

fn space_max(space: &Space, types: &[RoomType]) -> u8 {
    space
        .room_type_id
        .and_then(|id| types.iter().find(|t| t.id == id))
        .map_or(1, |t| t.max_occupancy)
}

fn shift(date: NaiveDate, days: i64) -> Option<NaiveDate> {
    let n = Days::new(days.unsigned_abs());
    if days >= 0 {
        date.checked_add_days(n)
    } else {
        date.checked_sub_days(n)
    }
}

/// A tiny deterministic random generator (xorshift), so the demo hotel is
/// the same every time without pulling in a random-number crate.
struct DemoRng(u32);

impl DemoRng {
    /// A number in `0..n` (n must be > 0).
    fn below(&mut self, n: u32) -> u32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.0 = x;
        x % n
    }
}

/// The demo hotel's room features: wifi everywhere, two accessible rooms and
/// three pet-friendly ones on the ground floor, and one smoking room (216).
fn demo_features(level: i32, number: u16) -> Vec<RoomFeature> {
    let mut features = vec![RoomFeature::Wifi];
    match (level, number) {
        (0, 1..=2) => features.push(RoomFeature::Accessible),
        (0, 3..=5) => features.push(RoomFeature::PetsAllowed),
        (2, 16) => features.push(RoomFeature::Smoking),
        _ => {}
    }
    features
}
