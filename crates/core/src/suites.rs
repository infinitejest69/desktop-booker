//! Function suite hire: rate cards, pricing and the no-overlap rule.
//!
//! A suite booking covers `[start, end)`. Around it the suite also needs
//! `setup_minutes` before and `cleardown_minutes` after, so two bookings clash
//! if those *padded* times overlap, even when the bookings themselves don't.

use chrono::{Duration, NaiveDateTime};
use serde::{Deserialize, Serialize};

use crate::ids::{EventId, GuestId, SpaceId, SuiteBookingId};
use crate::money::Money;
use crate::property::{Space, SpaceKind, SuiteCapacities};
use crate::validation::{ValidationError, ValidationResult};

/// A suite's prices and default changeover time. Missing prices mean that
/// way of pricing isn't offered for the suite.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct SuiteRates {
    pub hourly: Option<Money>,
    /// Hourly hire is charged for at least this many hours.
    pub min_hours: u8,
    pub half_day: Option<Money>,
    pub full_day: Option<Money>,
    pub setup_minutes: u16,
    pub cleardown_minutes: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub enum Layout {
    Theatre,
    Boardroom,
    Banquet,
    Cabaret,
    Reception,
}

impl Layout {
    pub const ALL: [Layout; 5] = [
        Self::Theatre,
        Self::Boardroom,
        Self::Banquet,
        Self::Cabaret,
        Self::Reception,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Theatre => "theatre",
            Self::Boardroom => "boardroom",
            Self::Banquet => "banquet",
            Self::Cabaret => "cabaret",
            Self::Reception => "reception",
        }
    }

    /// How many this layout seats in a suite, if the suite offers it.
    pub fn capacity(self, caps: &SuiteCapacities) -> Option<u16> {
        match self {
            Self::Theatre => caps.theatre,
            Self::Boardroom => caps.boardroom,
            Self::Banquet => caps.banquet,
            Self::Cabaret => caps.cabaret,
            Self::Reception => caps.reception,
        }
    }
}

impl std::str::FromStr for Layout {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, String> {
        Self::ALL
            .into_iter()
            .find(|l| l.as_str() == s)
            .ok_or_else(|| format!("unknown layout {s:?}"))
    }
}

/// How this booking is charged. An enum whose variants carry their own data:
/// only per-head and hand-set prices need an amount on the booking.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub enum SuitePricing {
    Hourly,
    HalfDay,
    FullDay,
    PerHead { price: Money },
    Manual { price: Money },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub enum SuiteBookingStatus {
    Provisional,
    Confirmed,
    Cancelled,
}

impl SuiteBookingStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Provisional => "provisional",
            Self::Confirmed => "confirmed",
            Self::Cancelled => "cancelled",
        }
    }
}

impl std::str::FromStr for SuiteBookingStatus {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, String> {
        [Self::Provisional, Self::Confirmed, Self::Cancelled]
            .into_iter()
            .find(|x| x.as_str() == s)
            .ok_or_else(|| format!("unknown suite booking status {s:?}"))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct SuiteBooking {
    pub id: SuiteBookingId,
    pub suite_id: SpaceId,
    pub event_id: Option<EventId>,
    /// Who booked it (the organiser).
    pub guest_id: GuestId,
    /// What it's for, e.g. "Wedding breakfast".
    pub title: String,
    pub start: NaiveDateTime,
    pub end: NaiveDateTime,
    pub layout: Layout,
    pub attendees: u16,
    pub setup_minutes: u16,
    pub cleardown_minutes: u16,
    pub pricing: SuitePricing,
    pub status: SuiteBookingStatus,
    pub notes: String,
}

impl SuiteBooking {
    /// The time the suite is out of use, including setup and clear-down.
    pub fn blocked(&self) -> (NaiveDateTime, NaiveDateTime) {
        (
            self.start - Duration::minutes(i64::from(self.setup_minutes)),
            self.end + Duration::minutes(i64::from(self.cleardown_minutes)),
        )
    }

    fn clashes_with(&self, other: &SuiteBooking) -> bool {
        let (a0, a1) = self.blocked();
        let (b0, b1) = other.blocked();
        a0 < b1 && b0 < a1
    }
}

/// A suite booking with what the timeline and event page show beside it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct SuiteBookingEntry {
    pub booking: SuiteBooking,
    /// Fixed when the booking was saved.
    pub price: Money,
    pub suite_name: String,
    pub guest_name: String,
    pub event_name: Option<String>,
}

/// Prices can't be negative and buffers stay under a day.
pub fn validate_suite_rates(rates: &SuiteRates) -> ValidationResult {
    let prices = [rates.hourly, rates.half_day, rates.full_day];
    if prices.iter().flatten().any(|m| m.0 < 0) {
        return Err(ValidationError::NegativeAmount {
            field: "Suite price",
        });
    }
    if rates.setup_minutes >= 24 * 60 || rates.cleardown_minutes >= 24 * 60 {
        return Err(ValidationError::InvalidBuffer);
    }
    Ok(())
}

/// The price of a suite booking, or an error if the suite doesn't offer the
/// chosen kind of pricing.
pub fn price_suite_booking(
    booking: &SuiteBooking,
    rates: &SuiteRates,
) -> Result<Money, ValidationError> {
    let missing = |what: &'static str| ValidationError::PriceNotOffered { what };
    Ok(match booking.pricing {
        SuitePricing::Hourly => {
            let rate = rates.hourly.ok_or(missing("hourly"))?;
            let minutes = (booking.end - booking.start).num_minutes().max(0);
            // Round up to whole hours, then apply the minimum.
            let hours = ((minutes + 59) / 60).max(i64::from(rates.min_hours));
            rate * hours
        }
        SuitePricing::HalfDay => rates.half_day.ok_or(missing("half-day"))?,
        SuitePricing::FullDay => rates.full_day.ok_or(missing("full-day"))?,
        SuitePricing::PerHead { price } => price * i64::from(booking.attendees),
        SuitePricing::Manual { price } => price,
    })
}

/// Checks a suite booking against the suite and its other bookings.
/// `others` may include this booking's saved copy; it's ignored.
pub fn validate_suite_booking(
    booking: &SuiteBooking,
    suite: &Space,
    others: &[SuiteBooking],
) -> ValidationResult {
    if suite.kind != SpaceKind::Suite {
        return Err(ValidationError::NotASuite {
            space: suite.name.clone(),
        });
    }
    if booking.title.trim().is_empty() {
        return Err(ValidationError::Empty {
            field: "What it's for",
        });
    }
    if booking.end <= booking.start {
        return Err(ValidationError::EndBeforeStart);
    }
    if booking.status == SuiteBookingStatus::Cancelled {
        return Ok(());
    }
    if suite.out_of_service {
        return Err(ValidationError::OutOfService {
            room: suite.name.clone(),
        });
    }
    let capacity = booking.layout.capacity(&suite.suite_capacities).ok_or(
        ValidationError::LayoutNotOffered {
            layout: booking.layout.as_str().into(),
        },
    )?;
    if booking.attendees == 0 || booking.attendees > capacity {
        return Err(ValidationError::TooManyAttendees { max: capacity });
    }
    if let Some(clash) = others.iter().find(|o| {
        o.id != booking.id
            && o.suite_id == booking.suite_id
            && o.status != SuiteBookingStatus::Cancelled
            && booking.clashes_with(o)
    }) {
        return Err(ValidationError::SuiteUnavailable {
            suite: suite.name.clone(),
            other: clash.title.clone(),
            at: clash.start.format("%-d %b %H:%M").to_string(),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grid::GridRect;
    use crate::ids::FloorId;

    fn t(s: &str) -> NaiveDateTime {
        NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M").unwrap()
    }

    fn suite() -> Space {
        Space {
            id: SpaceId::new(),
            floor_id: FloorId::new(),
            kind: SpaceKind::Suite,
            name: "Garden Suite".into(),
            room_type_id: None,
            suite_capacities: SuiteCapacities {
                banquet: Some(80),
                theatre: Some(120),
                ..Default::default()
            },
            rect: GridRect::new(0, 0, 6, 4),
            out_of_service: false,
            notes: String::new(),
        }
    }

    fn booking(suite: &Space, start: &str, end: &str) -> SuiteBooking {
        SuiteBooking {
            id: SuiteBookingId::new(),
            suite_id: suite.id,
            event_id: None,
            guest_id: GuestId::new(),
            title: "Dinner".into(),
            start: t(start),
            end: t(end),
            layout: Layout::Banquet,
            attendees: 60,
            setup_minutes: 60,
            cleardown_minutes: 30,
            pricing: SuitePricing::Hourly,
            status: SuiteBookingStatus::Confirmed,
            notes: String::new(),
        }
    }

    fn rates() -> SuiteRates {
        SuiteRates {
            hourly: Some(Money::from_major(50)),
            min_hours: 3,
            half_day: Some(Money::from_major(250)),
            full_day: None,
            setup_minutes: 60,
            cleardown_minutes: 30,
        }
    }

    #[test]
    fn a_valid_booking_passes() {
        let s = suite();
        assert_eq!(
            validate_suite_booking(
                &booking(&s, "2026-06-01 18:00", "2026-06-01 23:00"),
                &s,
                &[]
            ),
            Ok(())
        );
    }

    #[test]
    fn bookings_that_collide_only_through_buffers_are_rejected() {
        let s = suite();
        // Lunch ends 15:00, then 30 min clear-down; dinner needs 60 min setup from 16:00.
        let lunch = booking(&s, "2026-06-01 12:00", "2026-06-01 15:00");
        let tea = booking(&s, "2026-06-01 16:00", "2026-06-01 18:00");
        assert!(matches!(
            validate_suite_booking(&tea, &s, std::slice::from_ref(&lunch)),
            Err(ValidationError::SuiteUnavailable { .. })
        ));
        // 16:30 start: setup from 15:30, right after clear-down ends. Fine.
        let later = booking(&s, "2026-06-01 16:30", "2026-06-01 18:00");
        assert_eq!(validate_suite_booking(&later, &s, &[lunch]), Ok(()));
    }

    #[test]
    fn cancelled_bookings_and_other_suites_do_not_clash() {
        let s = suite();
        let mut a = booking(&s, "2026-06-01 12:00", "2026-06-01 15:00");
        let b = booking(&s, "2026-06-01 13:00", "2026-06-01 14:00");
        a.status = SuiteBookingStatus::Cancelled;
        assert_eq!(
            validate_suite_booking(&b, &s, std::slice::from_ref(&a)),
            Ok(())
        );
        a.status = SuiteBookingStatus::Confirmed;
        a.suite_id = SpaceId::new();
        assert_eq!(validate_suite_booking(&b, &s, &[a]), Ok(()));
    }

    #[test]
    fn capacity_depends_on_the_layout() {
        let s = suite();
        let mut b = booking(&s, "2026-06-01 18:00", "2026-06-01 23:00");
        b.attendees = 100;
        assert_eq!(
            validate_suite_booking(&b, &s, &[]),
            Err(ValidationError::TooManyAttendees { max: 80 })
        );
        b.layout = Layout::Theatre;
        assert_eq!(validate_suite_booking(&b, &s, &[]), Ok(()));
        b.layout = Layout::Boardroom;
        assert!(matches!(
            validate_suite_booking(&b, &s, &[]),
            Err(ValidationError::LayoutNotOffered { .. })
        ));
    }

    #[test]
    fn times_and_kind_are_checked() {
        let s = suite();
        let b = booking(&s, "2026-06-01 18:00", "2026-06-01 18:00");
        assert_eq!(
            validate_suite_booking(&b, &s, &[]),
            Err(ValidationError::EndBeforeStart)
        );
        let mut room = suite();
        room.kind = SpaceKind::Room;
        let b = booking(&room, "2026-06-01 18:00", "2026-06-01 19:00");
        assert!(matches!(
            validate_suite_booking(&b, &room, &[]),
            Err(ValidationError::NotASuite { .. })
        ));
    }

    #[test]
    fn hourly_rounds_up_and_has_a_minimum() {
        let s = suite();
        let short = booking(&s, "2026-06-01 10:00", "2026-06-01 11:00");
        assert_eq!(
            price_suite_booking(&short, &rates()),
            Ok(Money::from_major(150))
        );
        let long = booking(&s, "2026-06-01 10:00", "2026-06-01 14:10");
        assert_eq!(
            price_suite_booking(&long, &rates()),
            Ok(Money::from_major(250))
        );
    }

    #[test]
    fn other_pricing_kinds() {
        let s = suite();
        let mut b = booking(&s, "2026-06-01 09:00", "2026-06-01 13:00");
        b.pricing = SuitePricing::HalfDay;
        assert_eq!(
            price_suite_booking(&b, &rates()),
            Ok(Money::from_major(250))
        );
        b.pricing = SuitePricing::FullDay;
        assert_eq!(
            price_suite_booking(&b, &rates()),
            Err(ValidationError::PriceNotOffered { what: "full-day" })
        );
        b.pricing = SuitePricing::PerHead {
            price: Money::from_major(45),
        };
        assert_eq!(
            price_suite_booking(&b, &rates()),
            Ok(Money::from_major(45 * 60))
        );
        b.pricing = SuitePricing::Manual {
            price: Money::from_major(999),
        };
        assert_eq!(
            price_suite_booking(&b, &rates()),
            Ok(Money::from_major(999))
        );
    }

    #[test]
    fn rate_cards_reject_negative_prices_and_day_long_buffers() {
        let ok = SuiteRates {
            hourly: Some(Money::from_major(50)),
            setup_minutes: 60,
            ..SuiteRates::default()
        };
        assert_eq!(validate_suite_rates(&ok), Ok(()));
        let neg = SuiteRates {
            full_day: Some(Money(-1)),
            ..ok
        };
        assert!(validate_suite_rates(&neg).is_err());
        let long = SuiteRates {
            cleardown_minutes: 24 * 60,
            ..ok
        };
        assert_eq!(
            validate_suite_rates(&long),
            Err(ValidationError::InvalidBuffer)
        );
    }
}
