//! Business rules for the property setup.
//!
//! Each `validate_*` function returns `Ok(())` or the first rule broken.
//! They take everything they need as arguments and do no I/O. The store layer
//! loads the data, calls these, and only saves if they pass.

use serde::Serialize;
use thiserror::Error;

use crate::grid::MAX_GRID_CELLS;
use crate::property::{Floor, PropertySettings, RoomType, Space, SpaceKind};

/// A broken business rule, with a message fit to show to staff.
///
/// `thiserror` generates the `Display` text from each `#[error(...)]`, and
/// `Serialize` lets the UI receive the error as structured data.
#[derive(Debug, Clone, PartialEq, Eq, Error, Serialize)]
#[serde(tag = "code", rename_all = "snake_case")]
pub enum ValidationError {
    #[error("{field} must not be empty")]
    Empty { field: &'static str },
    #[error("Currency must be a 3-letter code such as GBP or EUR")]
    InvalidCurrency,
    #[error("Floor grid must be between 1 and {max} cells wide and tall")]
    GridSize { max: u16 },
    #[error("The grid is too small: \"{space}\" would no longer fit")]
    GridTooSmall { space: String },
    #[error("Name \"{name}\" is already used")]
    DuplicateName { name: String },
    #[error("Default occupancy must be between 1 and the maximum occupancy")]
    InvalidOccupancy,
    #[error("Width and height must be at least 1 cell")]
    EmptyRect,
    #[error("\"{space}\" doesn't fit on this floor's grid")]
    OutOfBounds { space: String },
    #[error("\"{space}\" overlaps \"{other}\"")]
    Overlaps { space: String, other: String },
    #[error("A room must have a room type")]
    RoomTypeRequired,
    #[error("A function suite can't have a room type")]
    SuiteHasRoomType,
    #[error("Floor \"{floor}\" still has {count} room(s)/suite(s); move or delete them first")]
    FloorNotEmpty { floor: String, count: usize },
    #[error("Room type \"{room_type}\" is used by {count} room(s); change them first")]
    RoomTypeInUse { room_type: String, count: usize },
    #[error("Times must be in 24-hour HH:MM format, e.g. 15:00")]
    InvalidTime,
    #[error("{field} can't be negative")]
    NegativeAmount { field: &'static str },
    #[error("The departure date must be after the arrival date (and within a year)")]
    InvalidStay,
    #[error("A rate rule's end date must be on or after its start date")]
    InvalidRuleDates,
    #[error("Weekdays must be numbers 1 (Monday) to 7 (Sunday)")]
    InvalidWeekday,
    #[error("A booking needs at least one room")]
    NoRooms,
    #[error("Each room needs at least one adult")]
    NoAdults,
    #[error("Room {room} sleeps at most {max}")]
    TooManyGuests { room: String, max: u8 },
    #[error("\"{space}\" isn't a bedroom")]
    NotABedroom { space: String },
    #[error("Room {room} is out of service")]
    OutOfService { room: String },
    #[error("Room {room} is in this booking twice for the same nights")]
    RoomBookedTwice { room: String },
    #[error("Room {room} is already booked on the night of {night}")]
    RoomUnavailable { room: String, night: String },
    #[error("A booking can't go from {from} to {to}")]
    InvalidStatusChange { from: String, to: String },
    #[error("The guest needs a first or last name")]
    GuestNameRequired,
    #[error("A PIN is 4 to 8 digits")]
    InvalidPin,
    #[error("The hotel needs at least one active admin")]
    LastAdmin,
    #[error("Auto-lock must be between 0 (never) and 240 minutes")]
    InvalidIdleLock,
    #[error("This suite has no {what} price; choose another way of pricing it")]
    PriceNotOffered { what: &'static str },
    #[error("\"{space}\" isn't a function suite")]
    NotASuite { space: String },
    #[error("The end time must be after the start time")]
    EndBeforeStart,
    #[error("This suite isn't set up for a {layout} layout")]
    LayoutNotOffered { layout: String },
    #[error("This layout seats at most {max}")]
    TooManyAttendees { max: u16 },
    #[error("{suite} is in use then (\"{other}\" at {at}, including setup and clear-down)")]
    SuiteUnavailable {
        suite: String,
        other: String,
        at: String,
    },
    #[error("The release date must be on or before the guests' arrival")]
    ReleaseAfterArrival,
    #[error(
        "Only {free} suitable room(s) are free for those nights; the block needs {wanted} more"
    )]
    NotEnoughRooms { wanted: u16, free: u16 },
    #[error("Setup and clear-down must each be under 24 hours")]
    InvalidBuffer,
    #[error("A tax rate must be between 0% and 100%")]
    InvalidTaxRate,
    #[error("A refund can't be more than has been paid")]
    RefundTooLarge,
    #[error("That doesn't look like an email address")]
    InvalidEmail,
}

pub type ValidationResult = Result<(), ValidationError>;

/// `&'static str` is a string baked into the program, valid for its whole run.
fn require(value: &str, field: &'static str) -> ValidationResult {
    if value.trim().is_empty() {
        Err(ValidationError::Empty { field })
    } else {
        Ok(())
    }
}

/// Names are compared ignoring case and surrounding spaces ("101" == " 101 ").
fn same_name(a: &str, b: &str) -> bool {
    a.trim().eq_ignore_ascii_case(b.trim())
}

pub fn validate_settings(settings: &PropertySettings) -> ValidationResult {
    require(&settings.hotel_name, "Hotel name")?;
    let currency = settings.currency.trim();
    if currency.len() != 3 || !currency.chars().all(|c| c.is_ascii_uppercase()) {
        return Err(ValidationError::InvalidCurrency);
    }
    require(&settings.timezone, "Time zone")?;
    if settings.idle_lock_minutes > 240 {
        return Err(ValidationError::InvalidIdleLock);
    }
    for t in [&settings.check_in_time, &settings.check_out_time] {
        if !is_hh_mm(t) {
            return Err(ValidationError::InvalidTime);
        }
    }
    Ok(())
}

/// "15:00" style 24-hour time.
fn is_hh_mm(s: &str) -> bool {
    let Some((h, m)) = s.split_once(':') else {
        return false;
    };
    let ok = |part: &str, max: u32| part.len() == 2 && part.parse::<u32>().is_ok_and(|n| n <= max);
    ok(h, 23) && ok(m, 59)
}

/// `others` = all existing floors (the floor being saved may be among them).
/// `spaces_on_floor` = spaces currently on this floor, which must still fit.
pub fn validate_floor(
    floor: &Floor,
    others: &[Floor],
    spaces_on_floor: &[Space],
) -> ValidationResult {
    require(&floor.name, "Floor name")?;
    let size_ok = |n: u16| (1..=MAX_GRID_CELLS).contains(&n);
    if !size_ok(floor.grid_cols) || !size_ok(floor.grid_rows) {
        return Err(ValidationError::GridSize {
            max: MAX_GRID_CELLS,
        });
    }
    if let Some(dup) = others
        .iter()
        .find(|f| f.id != floor.id && same_name(&f.name, &floor.name))
    {
        return Err(ValidationError::DuplicateName {
            name: dup.name.clone(),
        });
    }
    if let Some(s) = spaces_on_floor
        .iter()
        .find(|s| !s.rect.fits_within(floor.grid_cols, floor.grid_rows))
    {
        return Err(ValidationError::GridTooSmall {
            space: s.name.clone(),
        });
    }
    Ok(())
}

pub fn validate_room_type(room_type: &RoomType, others: &[RoomType]) -> ValidationResult {
    require(&room_type.name, "Room type name")?;
    require(&room_type.code, "Room type code")?;
    if room_type.default_occupancy == 0 || room_type.default_occupancy > room_type.max_occupancy {
        return Err(ValidationError::InvalidOccupancy);
    }
    for (amount, field) in [
        (room_type.base_rate, "Nightly rate"),
        (room_type.extra_adult_rate, "Extra adult rate"),
        (room_type.child_rate, "Child rate"),
    ] {
        if amount.0 < 0 {
            return Err(ValidationError::NegativeAmount { field });
        }
    }
    for other in others.iter().filter(|o| o.id != room_type.id) {
        if same_name(&other.name, &room_type.name) {
            return Err(ValidationError::DuplicateName {
                name: other.name.clone(),
            });
        }
        if same_name(&other.code, &room_type.code) {
            return Err(ValidationError::DuplicateName {
                name: other.code.clone(),
            });
        }
    }
    Ok(())
}

/// `floor` = the floor the space is on. `all_spaces` = every space in the
/// hotel (names must be unique across floors; overlaps are checked per floor).
pub fn validate_space(space: &Space, floor: &Floor, all_spaces: &[Space]) -> ValidationResult {
    require(&space.name, "Name")?;
    match (space.kind, space.room_type_id) {
        (SpaceKind::Room, None) => return Err(ValidationError::RoomTypeRequired),
        (SpaceKind::Suite, Some(_)) => return Err(ValidationError::SuiteHasRoomType),
        _ => {}
    }
    if space.rect.is_empty() {
        return Err(ValidationError::EmptyRect);
    }
    if !space.rect.fits_within(floor.grid_cols, floor.grid_rows) {
        return Err(ValidationError::OutOfBounds {
            space: space.name.clone(),
        });
    }
    // `filter` skips the space itself, so editing a space doesn't clash with
    // its own saved copy.
    for other in all_spaces.iter().filter(|o| o.id != space.id) {
        if same_name(&other.name, &space.name) {
            return Err(ValidationError::DuplicateName {
                name: other.name.clone(),
            });
        }
        if other.floor_id == space.floor_id && other.rect.overlaps(&space.rect) {
            return Err(ValidationError::Overlaps {
                space: space.name.clone(),
                other: other.name.clone(),
            });
        }
    }
    Ok(())
}

/// A floor can only be deleted once nothing is on it.
pub fn validate_floor_delete(floor: &Floor, spaces_on_floor: &[Space]) -> ValidationResult {
    match spaces_on_floor.len() {
        0 => Ok(()),
        count => Err(ValidationError::FloorNotEmpty {
            floor: floor.name.clone(),
            count,
        }),
    }
}

/// A room type can only be deleted once no room uses it.
pub fn validate_room_type_delete(room_type: &RoomType, all_spaces: &[Space]) -> ValidationResult {
    let count = all_spaces
        .iter()
        .filter(|s| s.room_type_id == Some(room_type.id))
        .count();
    if count == 0 {
        Ok(())
    } else {
        Err(ValidationError::RoomTypeInUse {
            room_type: room_type.name.clone(),
            count,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grid::GridRect;
    use crate::ids::{FloorId, RoomTypeId, SpaceId};
    use crate::property::SuiteCapacities;

    fn floor(cols: u16, rows: u16) -> Floor {
        Floor {
            id: FloorId::new(),
            name: "Ground".into(),
            level: 0,
            grid_cols: cols,
            grid_rows: rows,
        }
    }

    fn room(floor: &Floor, name: &str, rect: GridRect) -> Space {
        Space {
            id: SpaceId::new(),
            floor_id: floor.id,
            kind: SpaceKind::Room,
            name: name.into(),
            room_type_id: Some(RoomTypeId::new()),
            suite_capacities: SuiteCapacities::default(),
            rect,
            out_of_service: false,
            notes: String::new(),
        }
    }

    fn room_type(name: &str, code: &str) -> RoomType {
        RoomType {
            id: RoomTypeId::new(),
            name: name.into(),
            code: code.into(),
            default_occupancy: 2,
            max_occupancy: 3,
            description: String::new(),
            base_rate: crate::Money(9000),
            extra_adult_rate: crate::Money::ZERO,
            child_rate: crate::Money::ZERO,
        }
    }

    // ---- settings ----

    #[test]
    fn default_settings_are_valid() {
        assert_eq!(validate_settings(&PropertySettings::default()), Ok(()));
    }

    #[test]
    fn check_in_and_out_times_are_hh_mm() {
        for bad in ["3pm", "15:0", "24:00", "15:60", ""] {
            let s = PropertySettings {
                check_in_time: bad.into(),
                ..Default::default()
            };
            assert_eq!(
                validate_settings(&s),
                Err(ValidationError::InvalidTime),
                "{bad}"
            );
        }
    }

    #[test]
    fn room_type_prices_cannot_be_negative() {
        let mut rt = room_type("Double", "DBL");
        rt.child_rate = crate::Money(-1);
        assert_eq!(
            validate_room_type(&rt, &[]),
            Err(ValidationError::NegativeAmount {
                field: "Child rate"
            })
        );
    }

    #[test]
    fn settings_need_a_name_and_iso_currency() {
        let mut s = PropertySettings {
            hotel_name: "  ".into(),
            ..Default::default()
        };
        assert_eq!(
            validate_settings(&s),
            Err(ValidationError::Empty {
                field: "Hotel name"
            })
        );
        s.hotel_name = "The Grange".into();
        for bad in ["gbp", "GB", "EURO", "£££"] {
            s.currency = bad.into();
            assert_eq!(
                validate_settings(&s),
                Err(ValidationError::InvalidCurrency),
                "{bad}"
            );
        }
    }

    // ---- floors ----

    #[test]
    fn floor_grid_size_is_bounded() {
        let f = floor(0, 10);
        assert!(matches!(
            validate_floor(&f, &[], &[]),
            Err(ValidationError::GridSize { .. })
        ));
        let f = floor(MAX_GRID_CELLS + 1, 10);
        assert!(matches!(
            validate_floor(&f, &[], &[]),
            Err(ValidationError::GridSize { .. })
        ));
        assert_eq!(validate_floor(&floor(MAX_GRID_CELLS, 1), &[], &[]), Ok(()));
    }

    #[test]
    fn floor_names_are_unique_but_a_floor_can_be_resaved() {
        let f = floor(10, 10);
        let mut other = floor(10, 10);
        other.name = " ground ".into();
        assert!(matches!(
            validate_floor(&f, std::slice::from_ref(&other), &[]),
            Err(ValidationError::DuplicateName { .. })
        ));
        assert_eq!(validate_floor(&f, std::slice::from_ref(&f), &[]), Ok(()));
    }

    #[test]
    fn shrinking_a_floor_cannot_cut_off_a_space() {
        let mut f = floor(10, 10);
        let r = room(&f, "101", GridRect::new(6, 0, 4, 2));
        f.grid_cols = 9;
        assert_eq!(
            validate_floor(&f, &[], &[r]),
            Err(ValidationError::GridTooSmall {
                space: "101".into()
            })
        );
    }

    // ---- room types ----

    #[test]
    fn room_type_occupancy_must_be_sensible() {
        let mut rt = room_type("Double", "DBL");
        rt.default_occupancy = 0;
        assert_eq!(
            validate_room_type(&rt, &[]),
            Err(ValidationError::InvalidOccupancy)
        );
        rt.default_occupancy = 4;
        assert_eq!(
            validate_room_type(&rt, &[]),
            Err(ValidationError::InvalidOccupancy)
        );
    }

    #[test]
    fn room_type_name_and_code_are_unique() {
        let existing = room_type("Double", "DBL");
        let same_name = room_type("double", "DB2");
        let same_code = room_type("King", "dbl");
        assert!(validate_room_type(&same_name, std::slice::from_ref(&existing)).is_err());
        assert!(validate_room_type(&same_code, std::slice::from_ref(&existing)).is_err());
        assert_eq!(
            validate_room_type(&existing, std::slice::from_ref(&existing)),
            Ok(())
        );
    }

    // ---- spaces ----

    #[test]
    fn valid_room_passes() {
        let f = floor(10, 10);
        let r = room(&f, "101", GridRect::new(0, 0, 3, 2));
        assert_eq!(validate_space(&r, &f, &[]), Ok(()));
    }

    #[test]
    fn room_needs_a_type_and_suite_must_not_have_one() {
        let f = floor(10, 10);
        let mut r = room(&f, "101", GridRect::new(0, 0, 3, 2));
        r.room_type_id = None;
        assert_eq!(
            validate_space(&r, &f, &[]),
            Err(ValidationError::RoomTypeRequired)
        );

        let mut s = room(&f, "Garden Suite", GridRect::new(0, 0, 3, 2));
        s.kind = SpaceKind::Suite;
        assert_eq!(
            validate_space(&s, &f, &[]),
            Err(ValidationError::SuiteHasRoomType)
        );
        s.room_type_id = None;
        assert_eq!(validate_space(&s, &f, &[]), Ok(()));
    }

    #[test]
    fn space_must_fit_and_be_non_empty() {
        let f = floor(10, 10);
        let r = room(&f, "101", GridRect::new(8, 0, 3, 2));
        assert_eq!(
            validate_space(&r, &f, &[]),
            Err(ValidationError::OutOfBounds {
                space: "101".into()
            })
        );
        let r = room(&f, "101", GridRect::new(0, 0, 0, 2));
        assert_eq!(validate_space(&r, &f, &[]), Err(ValidationError::EmptyRect));
    }

    #[test]
    fn overlapping_spaces_on_the_same_floor_are_rejected() {
        let f = floor(10, 10);
        let a = room(&f, "101", GridRect::new(0, 0, 3, 2));
        let b = room(&f, "102", GridRect::new(2, 1, 3, 2));
        assert_eq!(
            validate_space(&b, &f, std::slice::from_ref(&a)),
            Err(ValidationError::Overlaps {
                space: "102".into(),
                other: "101".into()
            })
        );
    }

    #[test]
    fn same_position_on_different_floors_is_fine() {
        let ground = floor(10, 10);
        let mut first = floor(10, 10);
        first.name = "First".into();
        let a = room(&ground, "101", GridRect::new(0, 0, 3, 2));
        let b = room(&first, "201", GridRect::new(0, 0, 3, 2));
        assert_eq!(validate_space(&b, &first, &[a]), Ok(()));
    }

    #[test]
    fn names_are_unique_across_all_floors() {
        let ground = floor(10, 10);
        let mut first = floor(10, 10);
        first.name = "First".into();
        let a = room(&ground, "101", GridRect::new(0, 0, 3, 2));
        let b = room(&first, " 101", GridRect::new(5, 5, 3, 2));
        assert_eq!(
            validate_space(&b, &first, &[a]),
            Err(ValidationError::DuplicateName { name: "101".into() })
        );
    }

    #[test]
    fn moving_a_space_does_not_clash_with_itself() {
        let f = floor(10, 10);
        let a = room(&f, "101", GridRect::new(0, 0, 3, 2));
        let mut moved = a.clone();
        moved.rect = GridRect::new(1, 0, 3, 2);
        assert_eq!(validate_space(&moved, &f, &[a]), Ok(()));
    }

    // ---- deletes ----

    #[test]
    fn floor_with_spaces_cannot_be_deleted() {
        let f = floor(10, 10);
        assert_eq!(validate_floor_delete(&f, &[]), Ok(()));
        let r = room(&f, "101", GridRect::new(0, 0, 1, 1));
        assert_eq!(
            validate_floor_delete(&f, &[r]),
            Err(ValidationError::FloorNotEmpty {
                floor: "Ground".into(),
                count: 1
            })
        );
    }

    #[test]
    fn room_type_in_use_cannot_be_deleted() {
        let f = floor(10, 10);
        let rt = room_type("Double", "DBL");
        let mut r = room(&f, "101", GridRect::new(0, 0, 1, 1));
        let unrelated = room(&f, "102", GridRect::new(1, 0, 1, 1));
        assert_eq!(
            validate_room_type_delete(&rt, std::slice::from_ref(&unrelated)),
            Ok(())
        );
        r.room_type_id = Some(rt.id);
        assert_eq!(
            validate_room_type_delete(&rt, &[r, unrelated]),
            Err(ValidationError::RoomTypeInUse {
                room_type: "Double".into(),
                count: 1
            })
        );
    }

    #[test]
    fn errors_have_readable_messages() {
        let e = ValidationError::Overlaps {
            space: "102".into(),
            other: "101".into(),
        };
        assert_eq!(e.to_string(), "\"102\" overlaps \"101\"");
    }
}
