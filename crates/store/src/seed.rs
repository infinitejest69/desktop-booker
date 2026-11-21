//! Demo data so there is something to look at during development.

use booker_core::{
    Adjustment, BoardOption, BoardOptionId, Booking, BookingId, BookingRoom, BookingRoomId,
    BookingSource, BookingStatus, DateRange, Floor, FloorId, GridRect, Guest, GuestId, Money,
    RateRule, RateRuleId, RoomType, RoomTypeId, Space, SpaceId, SpaceKind, SuiteCapacities,
};
use chrono::{Datelike, Days, Local, NaiveDate};

use crate::repo::{BookingRepo, FloorRepo, RateRepo, RoomTypeRepo, SpaceRepo};
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
                    self.save_space(&room(&floor, label, slot, top, usize::from(number)))
                        .await?;
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
                        notes: String::new(),
                    })
                    .await?;
                }
            }
        }
        self.seed_rates_and_bookings().await?;
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
                    board_option_id: Some(boards[[1, 1, 1, 0, 2][rng.below(5) as usize]].id),
                    price_override: None,
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
