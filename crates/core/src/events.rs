//! Events (weddings, dinners, conferences) and what they gather together:
//! catering sittings with numbers and dietary needs, and a block of bedrooms
//! held for guests until a release date.

use chrono::{NaiveDate, NaiveTime};
use serde::{Deserialize, Serialize};

use crate::dates::DateRange;
use crate::ids::{EventId, GuestId, RoomTypeId};
use crate::money::Money;
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
}
