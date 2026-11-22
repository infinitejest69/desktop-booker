//! Events (weddings, dinners, conferences) and what they gather together:
//! catering sittings with numbers and dietary needs, and a block of bedrooms
//! held for guests until a release date.

use chrono::{NaiveDate, NaiveTime};
use serde::{Deserialize, Serialize};

use crate::booking::{BookingStatus, Guest, Occupancy, free_rooms};
use crate::dates::DateRange;
use crate::ids::{BookingId, EventId, GuestId, RoomTypeId, SpaceId};
use crate::money::Money;
use crate::property::Space;
use crate::suites::SuiteBookingEntry;
use crate::validation::{ValidationError, ValidationResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub enum EventKind {
    Wedding,
    Conference,
    Dinner,
    Party,
    Other,
}

impl EventKind {
    pub const ALL: [EventKind; 5] = [
        Self::Wedding,
        Self::Conference,
        Self::Dinner,
        Self::Party,
        Self::Other,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Wedding => "wedding",
            Self::Conference => "conference",
            Self::Dinner => "dinner",
            Self::Party => "party",
            Self::Other => "other",
        }
    }
}

impl std::str::FromStr for EventKind {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, String> {
        Self::ALL
            .into_iter()
            .find(|k| k.as_str() == s)
            .ok_or_else(|| format!("unknown event kind {s:?}"))
    }
}

/// One serving: e.g. "Wedding breakfast", 14:00, 80 adults, 6 children.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct CateringSitting {
    pub name: String,
    pub date: NaiveDate,
    pub time: NaiveTime,
    pub adults: u16,
    pub children: u16,
    /// Free text, one need per line, e.g. "3 vegetarian".
    pub dietary: String,
}

/// Bedrooms held for event guests at a group rate until `release_date`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct RoomBlock {
    pub room_type_id: Option<RoomTypeId>,
    pub rooms: u16,
    pub stay: DateRange,
    /// Group rate per room per night.
    pub rate: Money,
    /// Rooms still unclaimed on this date go back on general sale.
    pub release_date: NaiveDate,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct Event {
    pub id: EventId,
    pub name: String,
    pub kind: EventKind,
    /// The organiser (bride, company contact…).
    pub organiser_id: GuestId,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub notes: String,
    pub running_order: String,
    pub sittings: Vec<CateringSitting>,
    pub room_block: Option<RoomBlock>,
}

/// Total numbers across all sittings, for the kitchen.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct CateringTotals {
    pub sittings: u16,
    pub adults: u32,
    pub children: u32,
}

pub fn catering_totals(sittings: &[CateringSitting]) -> CateringTotals {
    CateringTotals {
        sittings: u16::try_from(sittings.len()).unwrap_or(u16::MAX),
        adults: sittings.iter().map(|s| u32::from(s.adults)).sum(),
        children: sittings.iter().map(|s| u32::from(s.children)).sum(),
    }
}

pub fn validate_event(event: &Event) -> ValidationResult {
    if event.name.trim().is_empty() {
        return Err(ValidationError::Empty {
            field: "Event name",
        });
    }
    if event.end_date < event.start_date {
        return Err(ValidationError::InvalidRuleDates);
    }
    for s in &event.sittings {
        if s.name.trim().is_empty() {
            return Err(ValidationError::Empty {
                field: "Sitting name",
            });
        }
        if s.adults == 0 && s.children == 0 {
            return Err(ValidationError::Empty {
                field: "Number of diners",
            });
        }
    }
    if let Some(block) = &event.room_block {
        block.stay.validate()?;
        if block.rooms == 0 {
            return Err(ValidationError::Empty {
                field: "Rooms to hold",
            });
        }
        if block.rate.0 < 0 {
            return Err(ValidationError::NegativeAmount {
                field: "Group rate",
            });
        }
        if block.release_date > block.stay.start {
            return Err(ValidationError::ReleaseAfterArrival);
        }
    }
    Ok(())
}

/// Whether held-but-unclaimed block rooms should be released on `today`.
pub fn block_due_for_release(block: &RoomBlock, today: NaiveDate) -> bool {
    today >= block.release_date
}

// ---------------------------------------------------------------------------
// Room blocks
//
// A block is made of ordinary bedroom bookings linked to the event. While a
// room is only *held* its booking is provisional; confirming it (usually after
// changing the guest to the person staying) means it has been taken up.
// ---------------------------------------------------------------------------

/// A bedroom booking linked to an event, as the block planner sees it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LinkedRoom {
    pub booking_id: BookingId,
    pub status: BookingStatus,
    pub stay: DateRange,
    pub price_override: Option<Money>,
}

impl LinkedRoom {
    fn is_held(&self) -> bool {
        self.status == BookingStatus::Provisional
    }
}

/// What to change so the event's bookings match its block.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BlockPlan {
    /// Held rooms to cancel (block removed, shrunk, moved or released).
    pub release: Vec<BookingId>,
    /// Held rooms whose group price needs updating.
    pub reprice: Vec<BookingId>,
    /// How many more rooms to hold.
    pub add: u16,
}

/// The group price for one block room's whole stay.
pub fn block_room_price(block: &RoomBlock) -> Money {
    block.rate * block.stay.nights_count()
}

/// Works out how to bring an event's linked bookings in line with its block.
///
/// Every live linked booking counts towards the block, taken up or not.
/// Only held (provisional) rooms are ever released or repriced, so a guest's
/// confirmed room is never touched. On or after the release date nothing new
/// is held and the remaining held rooms are released.
pub fn plan_room_block(
    block: Option<&RoomBlock>,
    linked: &[LinkedRoom],
    today: NaiveDate,
) -> BlockPlan {
    let live: Vec<&LinkedRoom> = linked.iter().filter(|r| r.status.holds_rooms()).collect();
    let held = || live.iter().filter(|r| r.is_held());
    let Some(block) = block.filter(|b| !block_due_for_release(b, today)) else {
        return BlockPlan {
            release: held().map(|r| r.booking_id).collect(),
            ..BlockPlan::default()
        };
    };

    // Held rooms for other nights no longer belong to the block.
    let mut release: Vec<BookingId> = held()
        .filter(|r| r.stay != block.stay)
        .map(|r| r.booking_id)
        .collect();
    let mut keep: Vec<&LinkedRoom> = held().filter(|r| r.stay == block.stay).copied().collect();
    let taken_up = live.iter().filter(|r| !r.is_held()).count();
    let counted = taken_up + keep.len();
    let wanted = usize::from(block.rooms);
    if counted > wanted {
        // Give back the most recently added held rooms first.
        let surplus = (counted - wanted).min(keep.len());
        release.extend(
            keep.split_off(keep.len() - surplus)
                .iter()
                .map(|r| r.booking_id),
        );
    }
    let price = block_room_price(block);
    BlockPlan {
        reprice: keep
            .iter()
            .filter(|r| r.price_override != Some(price))
            .map(|r| r.booking_id)
            .collect(),
        add: u16::try_from(wanted.saturating_sub(counted)).unwrap_or(u16::MAX),
        release,
    }
}

/// Chooses `count` free bedrooms of the block's room type (any type if none),
/// in the order given (callers pass them sorted by floor and name).
pub fn pick_block_rooms(
    block: &RoomBlock,
    spaces: &[Space],
    occupancy: &[Occupancy],
    count: u16,
) -> Result<Vec<SpaceId>, ValidationError> {
    let free: Vec<SpaceId> = free_rooms(&block.stay, spaces, occupancy, None)
        .into_iter()
        .filter(|s| block.room_type_id.is_none_or(|t| s.room_type_id == Some(t)))
        .map(|s| s.id)
        .collect();
    if free.len() < usize::from(count) {
        return Err(ValidationError::NotEnoughRooms {
            wanted: count,
            free: u16::try_from(free.len()).unwrap_or(u16::MAX),
        });
    }
    Ok(free.into_iter().take(usize::from(count)).collect())
}

// ---------------------------------------------------------------------------
// What the event pages show
// ---------------------------------------------------------------------------

/// A bedroom booking linked to an event, for the event page.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct EventRoom {
    pub booking_id: BookingId,
    pub reference: String,
    pub guest_name: String,
    pub status: BookingStatus,
    pub rooms: Vec<String>,
    pub stay: DateRange,
    pub total: Money,
}

/// Everything the event page shows.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct EventDetail {
    pub event: Event,
    pub organiser: Guest,
    pub suites: Vec<SuiteBookingEntry>,
    pub rooms: Vec<EventRoom>,
    /// Rooms still held (provisional) for the block.
    pub held_rooms: u16,
    pub catering: CateringTotals,
    pub suites_value: Money,
    pub rooms_value: Money,
    pub total: Money,
}

impl EventDetail {
    /// Adds up the value of live suite bookings and bedroom bookings.
    pub fn new(
        event: Event,
        organiser: Guest,
        suites: Vec<SuiteBookingEntry>,
        rooms: Vec<EventRoom>,
    ) -> Self {
        let suites_value: Money = suites
            .iter()
            .filter(|s| s.booking.status != crate::suites::SuiteBookingStatus::Cancelled)
            .map(|s| s.price)
            .sum();
        let rooms_value: Money = rooms
            .iter()
            .filter(|r| r.status.holds_rooms())
            .map(|r| r.total)
            .sum();
        let held_rooms = rooms
            .iter()
            .filter(|r| r.status == BookingStatus::Provisional)
            .count();
        Self {
            catering: catering_totals(&event.sittings),
            held_rooms: u16::try_from(held_rooms).unwrap_or(u16::MAX),
            suites_value,
            rooms_value,
            total: suites_value + rooms_value,
            event,
            organiser,
            suites,
            rooms,
        }
    }
}

/// A row in the events list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct EventListItem {
    pub id: EventId,
    pub name: String,
    pub kind: EventKind,
    pub organiser_name: String,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub suite_bookings: u16,
    pub held_rooms: u16,
    /// Set while the event has a room block.
    pub release_date: Option<NaiveDate>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(s: &str) -> NaiveDate {
        s.parse().unwrap()
    }

    fn sitting(name: &str, adults: u16, children: u16) -> CateringSitting {
        CateringSitting {
            name: name.into(),
            date: d("2026-06-20"),
            time: NaiveTime::from_hms_opt(14, 0, 0).unwrap(),
            adults,
            children,
            dietary: String::new(),
        }
    }

    fn wedding() -> Event {
        Event {
            id: EventId::new(),
            name: "Hughes–Price wedding".into(),
            kind: EventKind::Wedding,
            organiser_id: GuestId::new(),
            start_date: d("2026-06-20"),
            end_date: d("2026-06-21"),
            notes: String::new(),
            running_order: String::new(),
            sittings: vec![
                sitting("Wedding breakfast", 80, 6),
                sitting("Evening buffet", 120, 10),
            ],
            room_block: Some(RoomBlock {
                room_type_id: None,
                rooms: 10,
                stay: DateRange::new(d("2026-06-20"), d("2026-06-21")).unwrap(),
                rate: Money::from_major(85),
                release_date: d("2026-05-20"),
            }),
        }
    }

    #[test]
    fn a_valid_wedding_passes_and_totals_its_numbers() {
        let w = wedding();
        assert_eq!(validate_event(&w), Ok(()));
        assert_eq!(
            catering_totals(&w.sittings),
            CateringTotals {
                sittings: 2,
                adults: 200,
                children: 16
            }
        );
    }

    #[test]
    fn names_dates_and_numbers_are_checked() {
        let mut w = wedding();
        w.end_date = d("2026-06-19");
        assert_eq!(validate_event(&w), Err(ValidationError::InvalidRuleDates));
        let mut w = wedding();
        w.sittings.push(sitting("Breakfast", 0, 0));
        assert_eq!(
            validate_event(&w),
            Err(ValidationError::Empty {
                field: "Number of diners"
            })
        );
    }

    #[test]
    fn a_block_must_release_by_arrival() {
        let mut w = wedding();
        if let Some(b) = w.room_block.as_mut() {
            b.release_date = d("2026-06-21");
        }
        assert_eq!(
            validate_event(&w),
            Err(ValidationError::ReleaseAfterArrival)
        );
    }

    #[test]
    fn release_happens_on_or_after_the_release_date() {
        let block = wedding().room_block.unwrap();
        assert!(!block_due_for_release(&block, d("2026-05-19")));
        assert!(block_due_for_release(&block, d("2026-05-20")));
        assert!(block_due_for_release(&block, d("2026-06-01")));
    }

    fn linked(status: BookingStatus, stay: DateRange, price: Option<Money>) -> LinkedRoom {
        LinkedRoom {
            booking_id: BookingId::new(),
            status,
            stay,
            price_override: price,
        }
    }

    fn block_of(rooms: u16) -> RoomBlock {
        RoomBlock {
            rooms,
            ..wedding().room_block.unwrap()
        }
    }

    #[test]
    fn a_new_block_holds_every_room() {
        let plan = plan_room_block(Some(&block_of(10)), &[], d("2026-05-01"));
        assert_eq!(
            plan,
            BlockPlan {
                add: 10,
                ..BlockPlan::default()
            }
        );
    }

    #[test]
    fn taken_up_rooms_count_and_surplus_held_rooms_go_back() {
        let b = block_of(3);
        let price = Some(block_room_price(&b));
        let rooms = [
            linked(BookingStatus::Confirmed, b.stay, price),
            linked(BookingStatus::Provisional, b.stay, price),
            linked(BookingStatus::Provisional, b.stay, price),
            linked(BookingStatus::Provisional, b.stay, price),
            linked(BookingStatus::Cancelled, b.stay, price),
        ];
        let plan = plan_room_block(Some(&b), &rooms, d("2026-05-01"));
        // 1 taken up + 3 held = 4 for a block of 3: the last held one goes.
        assert_eq!(plan.release, vec![rooms[3].booking_id]);
        assert_eq!(plan.add, 0);
        assert!(plan.reprice.is_empty());
    }

    #[test]
    fn a_new_rate_reprices_and_new_dates_rehold() {
        let b = block_of(2);
        let old_stay = DateRange::new(d("2026-06-19"), d("2026-06-20")).unwrap();
        let rooms = [
            linked(
                BookingStatus::Provisional,
                b.stay,
                Some(Money::from_major(1)),
            ),
            linked(BookingStatus::Provisional, old_stay, None),
        ];
        let plan = plan_room_block(Some(&b), &rooms, d("2026-05-01"));
        assert_eq!(plan.reprice, vec![rooms[0].booking_id]);
        assert_eq!(plan.release, vec![rooms[1].booking_id]);
        assert_eq!(plan.add, 1);
        assert_eq!(block_room_price(&b), Money::from_major(85));
    }

    #[test]
    fn removing_or_releasing_the_block_frees_only_held_rooms() {
        let b = block_of(2);
        let rooms = [
            linked(BookingStatus::Provisional, b.stay, None),
            linked(BookingStatus::CheckedIn, b.stay, None),
        ];
        let removed = plan_room_block(None, &rooms, d("2026-05-01"));
        assert_eq!(removed.release, vec![rooms[0].booking_id]);
        let released = plan_room_block(Some(&b), &rooms, d("2026-05-20"));
        assert_eq!(released.release, vec![rooms[0].booking_id]);
        assert_eq!(released.add, 0);
    }

    #[test]
    fn block_rooms_are_free_rooms_of_the_chosen_type() {
        use crate::grid::GridRect;
        use crate::ids::FloorId;
        use crate::property::{SpaceKind, SuiteCapacities};
        let (double, twin) = (RoomTypeId::new(), RoomTypeId::new());
        let room = |name: &str, t: RoomTypeId| Space {
            id: SpaceId::new(),
            floor_id: FloorId::new(),
            kind: SpaceKind::Room,
            name: name.into(),
            room_type_id: Some(t),
            suite_capacities: SuiteCapacities::default(),
            rect: GridRect::new(0, 0, 1, 1),
            out_of_service: false,
            features: Vec::new(),
            notes: String::new(),
        };
        let spaces = [
            room("1", double),
            room("2", twin),
            room("3", double),
            room("4", double),
        ];
        let mut b = block_of(2);
        b.room_type_id = Some(double);
        let busy = [Occupancy {
            booking_id: BookingId::new(),
            space_id: spaces[0].id,
            stay: b.stay,
        }];
        assert_eq!(
            pick_block_rooms(&b, &spaces, &busy, 2),
            Ok(vec![spaces[2].id, spaces[3].id])
        );
        assert_eq!(
            pick_block_rooms(&b, &spaces, &busy, 3),
            Err(ValidationError::NotEnoughRooms { wanted: 3, free: 2 })
        );
    }
}
