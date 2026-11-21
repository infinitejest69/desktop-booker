//! Guests, bookings and the rules for taking them.

use std::collections::HashMap;

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use crate::dates::DateRange;
use crate::ids::{BoardOptionId, BookingId, BookingRoomId, GuestId, SpaceId};
use crate::money::Money;
use crate::property::{RoomType, Space, SpaceKind};
use crate::validation::{ValidationError, ValidationResult};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct Guest {
    pub id: GuestId,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub phone: String,
    pub address: String,
    pub notes: String,
}

impl Guest {
    pub fn display_name(&self) -> String {
        format!("{} {}", self.first_name.trim(), self.last_name.trim())
            .trim()
            .to_string()
    }
}

pub fn validate_guest(guest: &Guest) -> ValidationResult {
    if guest.first_name.trim().is_empty() && guest.last_name.trim().is_empty() {
        return Err(ValidationError::GuestNameRequired);
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub enum BookingStatus {
    Provisional,
    Confirmed,
    CheckedIn,
    CheckedOut,
    Cancelled,
    NoShow,
}

impl BookingStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Provisional => "provisional",
            Self::Confirmed => "confirmed",
            Self::CheckedIn => "checked_in",
            Self::CheckedOut => "checked_out",
            Self::Cancelled => "cancelled",
            Self::NoShow => "no_show",
        }
    }

    /// Cancelled and no-show bookings release their rooms.
    pub const fn holds_rooms(self) -> bool {
        !matches!(self, Self::Cancelled | Self::NoShow)
    }

    /// Allowed moves, including "undo" steps for mistakes at the desk.
    pub fn can_change_to(self, to: BookingStatus) -> bool {
        use BookingStatus::*;
        self == to
            || matches!(
                (self, to),
                (Provisional, Confirmed | CheckedIn | Cancelled)
                    | (Confirmed, Provisional | CheckedIn | Cancelled | NoShow)
                    | (CheckedIn, Confirmed | CheckedOut)
                    | (CheckedOut, CheckedIn)
                    | (Cancelled | NoShow, Provisional | Confirmed)
            )
    }
}

impl std::str::FromStr for BookingStatus {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, String> {
        use BookingStatus::*;
        [
            Provisional,
            Confirmed,
            CheckedIn,
            CheckedOut,
            Cancelled,
            NoShow,
        ]
        .into_iter()
        .find(|status| status.as_str() == s)
        .ok_or_else(|| format!("unknown booking status {s:?}"))
    }
}

/// How the booking reached the hotel (for reports later).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub enum BookingSource {
    Phone,
    Email,
    WalkIn,
    Website,
    Agent,
    Other,
}

impl BookingSource {
    pub const ALL: [BookingSource; 6] = [
        Self::Phone,
        Self::Email,
        Self::WalkIn,
        Self::Website,
        Self::Agent,
        Self::Other,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Phone => "phone",
            Self::Email => "email",
            Self::WalkIn => "walk_in",
            Self::Website => "website",
            Self::Agent => "agent",
            Self::Other => "other",
        }
    }
}

impl std::str::FromStr for BookingSource {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, String> {
        Self::ALL
            .into_iter()
            .find(|source| source.as_str() == s)
            .ok_or_else(|| format!("unknown booking source {s:?}"))
    }
}

/// One room within a booking. Rooms in the same booking can have different dates.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct BookingRoom {
    pub id: BookingRoomId,
    pub space_id: SpaceId,
    pub stay: DateRange,
    pub adults: u8,
    pub children: u8,
    pub board_option_id: Option<BoardOptionId>,
    /// Replaces the calculated total for this room's whole stay.
    pub price_override: Option<Money>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct Booking {
    pub id: BookingId,
    /// e.g. "2026-0001". Assigned by the store when first saved.
    pub reference: String,
    pub guest_id: GuestId,
    pub status: BookingStatus,
    pub source: BookingSource,
    pub notes: String,
    pub rooms: Vec<BookingRoom>,
}

/// A room held by a booking for a stay, as the availability check sees it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Occupancy {
    pub booking_id: BookingId,
    pub space_id: SpaceId,
    pub stay: DateRange,
}

/// "2026-0001": year, then a number padded to at least four digits.
pub fn format_reference(year: i32, number: u32) -> String {
    format!("{year}-{number:04}")
}

/// First night two stays share (only called when they overlap).
fn first_shared_night(a: &DateRange, b: &DateRange) -> NaiveDate {
    a.start.max(b.start)
}

/// Checks a booking against the rooms, room types and everything already booked.
///
/// `occupancy` may include this booking's own saved rooms; they're ignored, so
/// editing a booking never clashes with itself.
pub fn validate_booking(
    booking: &Booking,
    spaces: &[Space],
    room_types: &[RoomType],
    occupancy: &[Occupancy],
) -> ValidationResult {
    if booking.rooms.is_empty() {
        return Err(ValidationError::NoRooms);
    }
    // A HashMap gives quick lookups by id instead of searching the list each time.
    let spaces_by_id: HashMap<SpaceId, &Space> = spaces.iter().map(|s| (s.id, s)).collect();

    for (i, room) in booking.rooms.iter().enumerate() {
        room.stay.validate()?;
        let space = spaces_by_id
            .get(&room.space_id)
            .ok_or(ValidationError::Empty { field: "Room" })?;
        if space.kind != SpaceKind::Room {
            return Err(ValidationError::NotABedroom {
                space: space.name.clone(),
            });
        }
        if room.adults == 0 {
            return Err(ValidationError::NoAdults);
        }
        let max = space
            .room_type_id
            .and_then(|id| room_types.iter().find(|t| t.id == id))
            .map_or(u8::MAX, |t| t.max_occupancy);
        if u16::from(room.adults) + u16::from(room.children) > u16::from(max) {
            return Err(ValidationError::TooManyGuests {
                room: space.name.clone(),
                max,
            });
        }
        // The same room twice in one booking, on overlapping nights.
        if booking.rooms[..i]
            .iter()
            .any(|other| other.space_id == room.space_id && other.stay.overlaps(&room.stay))
        {
            return Err(ValidationError::RoomBookedTwice {
                room: space.name.clone(),
            });
        }
    }

    if !booking.status.holds_rooms() {
        return Ok(());
    }
    for room in &booking.rooms {
        let space = spaces_by_id[&room.space_id];
        if space.out_of_service {
            return Err(ValidationError::OutOfService {
                room: space.name.clone(),
            });
        }
        if let Some(clash) = occupancy.iter().find(|o| {
            o.booking_id != booking.id && o.space_id == room.space_id && o.stay.overlaps(&room.stay)
        }) {
            return Err(ValidationError::RoomUnavailable {
                room: space.name.clone(),
                night: first_shared_night(&clash.stay, &room.stay)
                    .format("%-d %b %Y")
                    .to_string(),
            });
        }
    }
    Ok(())
}

/// Bedrooms that are in service and free for the whole stay.
pub fn free_rooms<'a>(
    stay: &DateRange,
    spaces: &'a [Space],
    occupancy: &[Occupancy],
    ignore_booking: Option<BookingId>,
) -> Vec<&'a Space> {
    spaces
        .iter()
        .filter(|s| s.kind == SpaceKind::Room && !s.out_of_service)
        .filter(|s| {
            !occupancy.iter().any(|o| {
                Some(o.booking_id) != ignore_booking && o.space_id == s.id && o.stay.overlaps(stay)
            })
        })
        .collect()
}

/// One booked room as the tariff chart draws it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct ChartEntry {
    pub booking_id: BookingId,
    pub booking_room_id: BookingRoomId,
    pub reference: String,
    pub guest_name: String,
    pub status: BookingStatus,
    pub space_id: SpaceId,
    pub stay: DateRange,
    pub adults: u8,
    pub children: u8,
}

/// The price of one booked room: the nights as priced when booked, and the
/// total actually charged (the override, if staff set one).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct RoomPricing {
    pub booking_room_id: BookingRoomId,
    pub nights: Vec<crate::rates::NightPrice>,
    pub calculated: Money,
    pub total: Money,
}

/// Everything the booking page shows.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct BookingDetail {
    pub booking: Booking,
    pub guest: Guest,
    pub pricing: Vec<RoomPricing>,
    pub total: Money,
}

/// A row in booking search results.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct BookingListItem {
    pub id: BookingId,
    pub reference: String,
    pub guest_name: String,
    pub status: BookingStatus,
    pub arrival: NaiveDate,
    pub departure: NaiveDate,
    pub rooms: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grid::GridRect;
    use crate::ids::{FloorId, RoomTypeId};
    use crate::property::SuiteCapacities;
    use proptest::prelude::*;

    fn d(s: &str) -> NaiveDate {
        s.parse().unwrap()
    }

    fn stay(a: &str, b: &str) -> DateRange {
        DateRange::new(d(a), d(b)).unwrap()
    }

    struct Hotel {
        spaces: Vec<Space>,
        types: Vec<RoomType>,
    }

    fn hotel(rooms: usize) -> Hotel {
        let rt = RoomType {
            id: RoomTypeId::new(),
            name: "Double".into(),
            code: "DBL".into(),
            default_occupancy: 2,
            max_occupancy: 3,
            description: String::new(),
            base_rate: Money::from_major(90),
            extra_adult_rate: Money::ZERO,
            child_rate: Money::ZERO,
        };
        let floor = FloorId::new();
        let mut spaces: Vec<Space> = (0..rooms)
            .map(|i| Space {
                id: SpaceId::new(),
                floor_id: floor,
                kind: SpaceKind::Room,
                name: format!("{}", 101 + i),
                room_type_id: Some(rt.id),
                suite_capacities: SuiteCapacities::default(),
                rect: GridRect::new(u16::try_from(i).unwrap() * 3, 0, 3, 3),
                out_of_service: false,
                notes: String::new(),
            })
            .collect();
        spaces.push(Space {
            id: SpaceId::new(),
            floor_id: floor,
            kind: SpaceKind::Suite,
            name: "Garden Suite".into(),
            room_type_id: None,
            suite_capacities: SuiteCapacities::default(),
            rect: GridRect::new(0, 5, 6, 3),
            out_of_service: false,
            notes: String::new(),
        });
        Hotel {
            spaces,
            types: vec![rt],
        }
    }

    fn booking(rooms: Vec<BookingRoom>) -> Booking {
        Booking {
            id: BookingId::new(),
            reference: String::new(),
            guest_id: GuestId::new(),
            status: BookingStatus::Confirmed,
            source: BookingSource::Phone,
            notes: String::new(),
            rooms,
        }
    }

    fn room(space: &Space, s: DateRange, adults: u8, children: u8) -> BookingRoom {
        BookingRoom {
            id: BookingRoomId::new(),
            space_id: space.id,
            stay: s,
            adults,
            children,
            board_option_id: None,
            price_override: None,
        }
    }

    fn occupancy_of(b: &Booking) -> Vec<Occupancy> {
        b.rooms
            .iter()
            .map(|r| Occupancy {
                booking_id: b.id,
                space_id: r.space_id,
                stay: r.stay,
            })
            .collect()
    }

    #[test]
    fn references_are_year_and_padded_number() {
        assert_eq!(format_reference(2026, 1), "2026-0001");
        assert_eq!(format_reference(2026, 12345), "2026-12345");
    }

    #[test]
    fn status_round_trips_and_transitions() {
        use BookingStatus::*;
        for s in [
            Provisional,
            Confirmed,
            CheckedIn,
            CheckedOut,
            Cancelled,
            NoShow,
        ] {
            assert_eq!(s.as_str().parse::<BookingStatus>(), Ok(s));
        }
        assert!(Confirmed.can_change_to(CheckedIn));
        assert!(CheckedIn.can_change_to(CheckedOut));
        assert!(!Provisional.can_change_to(CheckedOut));
        assert!(!CheckedOut.can_change_to(Cancelled));
        assert!(!Cancelled.holds_rooms());
    }

    #[test]
    fn a_valid_booking_passes() {
        let h = hotel(2);
        let b = booking(vec![room(
            &h.spaces[0],
            stay("2026-05-10", "2026-05-12"),
            2,
            1,
        )]);
        assert_eq!(validate_booking(&b, &h.spaces, &h.types, &[]), Ok(()));
    }

    #[test]
    fn overlapping_booking_is_rejected_with_the_first_clashing_night() {
        let h = hotel(1);
        let existing = booking(vec![room(
            &h.spaces[0],
            stay("2026-05-10", "2026-05-13"),
            2,
            0,
        )]);
        let b = booking(vec![room(
            &h.spaces[0],
            stay("2026-05-12", "2026-05-15"),
            2,
            0,
        )]);
        assert_eq!(
            validate_booking(&b, &h.spaces, &h.types, &occupancy_of(&existing)),
            Err(ValidationError::RoomUnavailable {
                room: "101".into(),
                night: "12 May 2026".into()
            })
        );
    }

    #[test]
    fn same_day_turnover_is_allowed() {
        let h = hotel(1);
        let existing = booking(vec![room(
            &h.spaces[0],
            stay("2026-05-10", "2026-05-12"),
            2,
            0,
        )]);
        let b = booking(vec![room(
            &h.spaces[0],
            stay("2026-05-12", "2026-05-14"),
            2,
            0,
        )]);
        assert_eq!(
            validate_booking(&b, &h.spaces, &h.types, &occupancy_of(&existing)),
            Ok(())
        );
    }

    #[test]
    fn editing_a_booking_does_not_clash_with_itself() {
        let h = hotel(1);
        let mut b = booking(vec![room(
            &h.spaces[0],
            stay("2026-05-10", "2026-05-12"),
            2,
            0,
        )]);
        let saved = occupancy_of(&b);
        b.rooms[0].stay = stay("2026-05-11", "2026-05-14");
        assert_eq!(validate_booking(&b, &h.spaces, &h.types, &saved), Ok(()));
    }

    #[test]
    fn cancelled_bookings_skip_the_clash_check() {
        let h = hotel(1);
        let existing = booking(vec![room(
            &h.spaces[0],
            stay("2026-05-10", "2026-05-12"),
            2,
            0,
        )]);
        let mut b = booking(vec![room(
            &h.spaces[0],
            stay("2026-05-10", "2026-05-12"),
            2,
            0,
        )]);
        b.status = BookingStatus::Cancelled;
        assert_eq!(
            validate_booking(&b, &h.spaces, &h.types, &occupancy_of(&existing)),
            Ok(())
        );
    }

    #[test]
    fn guests_rooms_and_suites_rules() {
        let h = hotel(1);
        let s = stay("2026-05-10", "2026-05-12");
        assert_eq!(
            validate_booking(&booking(vec![]), &h.spaces, &h.types, &[]),
            Err(ValidationError::NoRooms)
        );
        assert_eq!(
            validate_booking(
                &booking(vec![room(&h.spaces[0], s, 0, 1)]),
                &h.spaces,
                &h.types,
                &[]
            ),
            Err(ValidationError::NoAdults)
        );
        assert_eq!(
            validate_booking(
                &booking(vec![room(&h.spaces[0], s, 2, 2)]),
                &h.spaces,
                &h.types,
                &[]
            ),
            Err(ValidationError::TooManyGuests {
                room: "101".into(),
                max: 3
            })
        );
        assert_eq!(
            validate_booking(
                &booking(vec![room(&h.spaces[1], s, 2, 0)]),
                &h.spaces,
                &h.types,
                &[]
            ),
            Err(ValidationError::NotABedroom {
                space: "Garden Suite".into()
            })
        );
        let twice = booking(vec![
            room(&h.spaces[0], s, 2, 0),
            room(&h.spaces[0], stay("2026-05-11", "2026-05-13"), 1, 0),
        ]);
        assert_eq!(
            validate_booking(&twice, &h.spaces, &h.types, &[]),
            Err(ValidationError::RoomBookedTwice { room: "101".into() })
        );
    }

    #[test]
    fn out_of_service_rooms_cannot_be_booked() {
        let mut h = hotel(1);
        h.spaces[0].out_of_service = true;
        let b = booking(vec![room(
            &h.spaces[0],
            stay("2026-05-10", "2026-05-12"),
            2,
            0,
        )]);
        assert_eq!(
            validate_booking(&b, &h.spaces, &h.types, &[]),
            Err(ValidationError::OutOfService { room: "101".into() })
        );
    }

    #[test]
    fn free_rooms_excludes_booked_suites_and_out_of_service() {
        let mut h = hotel(3);
        h.spaces[2].out_of_service = true;
        let taken = booking(vec![room(
            &h.spaces[0],
            stay("2026-05-10", "2026-05-12"),
            2,
            0,
        )]);
        let occ = occupancy_of(&taken);
        let free: Vec<_> = free_rooms(&stay("2026-05-11", "2026-05-13"), &h.spaces, &occ, None)
            .into_iter()
            .map(|s| s.name.as_str())
            .collect();
        assert_eq!(free, vec!["102"]);
        // Ignoring the booking being edited frees its room again.
        assert_eq!(
            free_rooms(
                &stay("2026-05-11", "2026-05-13"),
                &h.spaces,
                &occ,
                Some(taken.id)
            )
            .len(),
            2
        );
    }

    proptest! {
        /// Whatever bookings are attempted, in any order, the accepted ones
        /// never share a room-night.
        #[test]
        fn accepted_bookings_never_double_book(
            requests in prop::collection::vec((0usize..3, 0i64..30, 1i64..6), 1..40)
        ) {
            let h = hotel(3);
            let base = d("2026-06-01");
            let mut occupancy: Vec<Occupancy> = Vec::new();
            for (room_ix, offset, nights) in requests {
                let start = base + chrono::Days::new(offset.unsigned_abs());
                let end = start + chrono::Days::new(nights.unsigned_abs());
                let b = booking(vec![room(&h.spaces[room_ix], DateRange::new(start, end).unwrap(), 1, 0)]);
                if validate_booking(&b, &h.spaces, &h.types, &occupancy).is_ok() {
                    occupancy.extend(occupancy_of(&b));
                }
            }
            let mut seen = std::collections::HashSet::new();
            for o in &occupancy {
                for night in o.stay.nights() {
                    prop_assert!(seen.insert((o.space_id, night)), "double-booked {:?} on {}", o.space_id, night);
                }
            }
        }
    }
}
