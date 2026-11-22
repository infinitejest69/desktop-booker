//! Room pricing: base rate per room type, rate rules on top, board and
//! occupancy supplements, giving a price for every night of a stay.
//!
//! The nightly room price is worked out like this:
//!
//! 1. Start from the room type's `base_rate`.
//! 2. Apply every active rule that covers that night (date range, weekday and
//!    room type all match), lowest `priority` first. A `SetPrice` replaces the
//!    price; `AddAmount` and `Percent` adjust whatever it is at that point.
//! 3. Add supplements: extra adults above the usual occupancy, and children.
//! 4. Add board (e.g. B&B) per adult and per child.

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use crate::dates::{DateRange, iso_weekday};
use crate::ids::{BoardOptionId, RateRuleId, RoomTypeId};
use crate::money::Money;
use crate::property::RoomType;
use crate::validation::{ValidationError, ValidationResult};

/// How a rule changes the nightly room price.
///
/// `#[serde(tag = "kind")]` makes each variant serialise as an object with a
/// `kind` field, e.g. `{ "kind": "percent", "percent": 10 }`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub enum Adjustment {
    /// Use exactly this price.
    SetPrice { amount: Money },
    /// Add (or, if negative, subtract) an amount.
    AddAmount { amount: Money },
    /// Change by a percentage, e.g. 10 or -15.
    Percent { percent: i32 },
}

impl Adjustment {
    fn apply(self, price: Money) -> Money {
        match self {
            Adjustment::SetPrice { amount } => amount,
            Adjustment::AddAmount { amount } => price + amount,
            Adjustment::Percent { percent } => price.adjust_percent(percent),
        }
        .at_least_zero()
    }
}

/// A seasonal, weekend or special-offer price change.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct RateRule {
    pub id: RateRuleId,
    pub name: String,
    /// `None` = every room type.
    pub room_type_id: Option<RoomTypeId>,
    /// First night the rule covers.
    pub start: NaiveDate,
    /// Last night the rule covers (inclusive, unlike a stay's departure date).
    pub end: NaiveDate,
    /// ISO weekdays (1 = Monday … 7 = Sunday). Empty = every day.
    pub weekdays: Vec<u8>,
    /// Lower numbers apply first.
    pub priority: i32,
    pub adjustment: Adjustment,
    pub active: bool,
}

impl RateRule {
    pub fn applies_to(&self, room_type: RoomTypeId, night: NaiveDate) -> bool {
        self.active
            && self.start <= night
            && night <= self.end
            && self.room_type_id.is_none_or(|id| id == room_type)
            && (self.weekdays.is_empty() || self.weekdays.contains(&iso_weekday(night)))
    }
}

/// A meal plan, e.g. Room only, Bed & breakfast, Dinner, bed & breakfast.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct BoardOption {
    pub id: BoardOptionId,
    /// Short code, e.g. "BB".
    pub code: String,
    pub name: String,
    /// Per adult per night.
    pub adult_price: Money,
    /// Per child per night.
    pub child_price: Money,
}

/// The price of one night, split so staff can see where it comes from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct NightPrice {
    pub night: NaiveDate,
    pub room: Money,
    pub supplements: Money,
    pub board: Money,
    /// Cars × the parking price.
    pub parking: Money,
    pub total: Money,
    /// Names of the rate rules that changed the room price.
    pub rules: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct StayQuote {
    pub nights: Vec<NightPrice>,
    pub total: Money,
}

/// The room price for one night after rate rules (no supplements or board).
pub fn room_rate_for_night(
    room_type: &RoomType,
    rules: &[RateRule],
    night: NaiveDate,
) -> (Money, Vec<String>) {
    let mut matching: Vec<&RateRule> = rules
        .iter()
        .filter(|r| r.applies_to(room_type.id, night))
        .collect();
    // Stable order: priority first, then the more specific (later-starting) rule.
    matching.sort_by_key(|r| (r.priority, r.start));
    let mut price = room_type.base_rate;
    let mut names = Vec::with_capacity(matching.len());
    for rule in matching {
        price = rule.adjustment.apply(price);
        names.push(rule.name.clone());
    }
    (price, names)
}

/// Prices every night of a stay for one room, with parking for `cars` at
/// `parking_price` per car per night.
#[allow(clippy::too_many_arguments)] // each is a separate input to the price
pub fn quote_stay(
    room_type: &RoomType,
    rules: &[RateRule],
    board: Option<&BoardOption>,
    stay: DateRange,
    adults: u8,
    children: u8,
    cars: u8,
    parking_price: Money,
) -> StayQuote {
    let extra_adults = i64::from(adults.saturating_sub(room_type.default_occupancy));
    let children = i64::from(children);
    let supplements = room_type.extra_adult_rate * extra_adults + room_type.child_rate * children;
    let board_price = board.map_or(Money::ZERO, |b| {
        b.adult_price * i64::from(adults) + b.child_price * children
    });
    let parking = parking_price * i64::from(cars);

    let nights: Vec<NightPrice> = stay
        .nights()
        .map(|night| {
            let (room, rules) = room_rate_for_night(room_type, rules, night);
            NightPrice {
                night,
                room,
                supplements,
                board: board_price,
                parking,
                total: room + supplements + board_price + parking,
                rules,
            }
        })
        .collect();
    let total = nights.iter().map(|n| n.total).sum();
    StayQuote { nights, total }
}

pub fn validate_rate_rule(rule: &RateRule) -> ValidationResult {
    if rule.name.trim().is_empty() {
        return Err(ValidationError::Empty { field: "Rule name" });
    }
    if rule.end < rule.start {
        return Err(ValidationError::InvalidRuleDates);
    }
    if rule.weekdays.iter().any(|d| !(1..=7).contains(d)) {
        return Err(ValidationError::InvalidWeekday);
    }
    if let Adjustment::SetPrice { amount } = rule.adjustment
        && amount.0 < 0
    {
        return Err(ValidationError::NegativeAmount { field: "Price" });
    }
    Ok(())
}

pub fn validate_board_option(board: &BoardOption, others: &[BoardOption]) -> ValidationResult {
    if board.name.trim().is_empty() {
        return Err(ValidationError::Empty {
            field: "Board name",
        });
    }
    if board.code.trim().is_empty() {
        return Err(ValidationError::Empty {
            field: "Board code",
        });
    }
    if board.adult_price.0 < 0 || board.child_price.0 < 0 {
        return Err(ValidationError::NegativeAmount {
            field: "Board price",
        });
    }
    if let Some(dup) = others
        .iter()
        .find(|o| o.id != board.id && o.code.trim().eq_ignore_ascii_case(board.code.trim()))
    {
        return Err(ValidationError::DuplicateName {
            name: dup.code.clone(),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(s: &str) -> NaiveDate {
        s.parse().unwrap()
    }

    fn double() -> RoomType {
        RoomType {
            id: RoomTypeId::new(),
            name: "Double".into(),
            code: "DBL".into(),
            default_occupancy: 2,
            max_occupancy: 3,
            description: String::new(),
            base_rate: Money::from_major(90),
            extra_adult_rate: Money::from_major(25),
            child_rate: Money::from_major(15),
        }
    }

    fn rule(name: &str, start: &str, end: &str, adjustment: Adjustment) -> RateRule {
        RateRule {
            id: RateRuleId::new(),
            name: name.into(),
            room_type_id: None,
            start: d(start),
            end: d(end),
            weekdays: vec![],
            priority: 0,
            adjustment,
            active: true,
        }
    }

    fn bb() -> BoardOption {
        BoardOption {
            id: BoardOptionId::new(),
            code: "BB".into(),
            name: "Bed & breakfast".into(),
            adult_price: Money::from_major(12),
            child_price: Money::from_major(6),
        }
    }

    #[test]
    fn base_rate_with_no_rules() {
        let (price, rules) = room_rate_for_night(&double(), &[], d("2026-03-10"));
        assert_eq!(price, Money::from_major(90));
        assert!(rules.is_empty());
    }

    #[test]
    fn season_boundary_prices_each_night_correctly() {
        // Summer starts on the night of 1 July.
        let summer = rule(
            "Summer",
            "2026-07-01",
            "2026-08-31",
            Adjustment::SetPrice {
                amount: Money::from_major(120),
            },
        );
        let q = quote_stay(
            &double(),
            &[summer],
            None,
            DateRange::new(d("2026-06-29"), d("2026-07-02")).unwrap(),
            2,
            0,
            0,
            Money::ZERO,
        );
        let rooms: Vec<_> = q.nights.iter().map(|n| n.room).collect();
        assert_eq!(
            rooms,
            vec![
                Money::from_major(90),
                Money::from_major(90),
                Money::from_major(120)
            ]
        );
        assert_eq!(q.total, Money::from_major(300));
        assert_eq!(q.nights[2].rules, vec!["Summer".to_string()]);
    }

    #[test]
    fn weekend_rule_only_on_its_weekdays() {
        let mut weekend = rule(
            "Weekend",
            "2026-01-01",
            "2026-12-31",
            Adjustment::AddAmount {
                amount: Money::from_major(20),
            },
        );
        weekend.weekdays = vec![5, 6]; // Fri, Sat
        let fri = d("2026-11-27");
        let sun = d("2026-11-29");
        assert_eq!(
            room_rate_for_night(&double(), std::slice::from_ref(&weekend), fri).0,
            Money::from_major(110)
        );
        assert_eq!(
            room_rate_for_night(&double(), &[weekend], sun).0,
            Money::from_major(90)
        );
    }

    #[test]
    fn rules_apply_in_priority_order() {
        let mut set = rule(
            "Summer",
            "2026-07-01",
            "2026-07-31",
            Adjustment::SetPrice {
                amount: Money::from_major(100),
            },
        );
        let mut pct = rule(
            "Offer",
            "2026-07-01",
            "2026-07-31",
            Adjustment::Percent { percent: -10 },
        );
        set.priority = 1;
        pct.priority = 2;
        let night = d("2026-07-10");
        assert_eq!(
            room_rate_for_night(&double(), &[pct.clone(), set.clone()], night).0,
            Money::from_major(90)
        );
        // Swap priorities: the 10% comes off 90 first, then Summer overrides it.
        set.priority = 3;
        assert_eq!(
            room_rate_for_night(&double(), &[pct, set], night).0,
            Money::from_major(100)
        );
    }

    #[test]
    fn rule_for_another_room_type_or_inactive_is_ignored() {
        let mut other = rule(
            "Twin offer",
            "2026-01-01",
            "2026-12-31",
            Adjustment::SetPrice { amount: Money(1) },
        );
        other.room_type_id = Some(RoomTypeId::new());
        let mut off = rule(
            "Paused",
            "2026-01-01",
            "2026-12-31",
            Adjustment::SetPrice { amount: Money(1) },
        );
        off.active = false;
        assert_eq!(
            room_rate_for_night(&double(), &[other, off], d("2026-05-05")).0,
            Money::from_major(90)
        );
    }

    #[test]
    fn discounts_never_go_below_zero() {
        let big = rule(
            "Oops",
            "2026-01-01",
            "2026-12-31",
            Adjustment::AddAmount {
                amount: Money::from_major(-500),
            },
        );
        assert_eq!(
            room_rate_for_night(&double(), &[big], d("2026-05-05")).0,
            Money::ZERO
        );
    }

    #[test]
    fn supplements_and_board_are_per_person_per_night() {
        // 3 adults (1 extra) + 1 child, B&B, 2 nights.
        let q = quote_stay(
            &double(),
            &[],
            Some(&bb()),
            DateRange::new(d("2026-05-01"), d("2026-05-03")).unwrap(),
            3,
            1,
            0,
            Money::ZERO,
        );
        let night = &q.nights[0];
        assert_eq!(night.room, Money::from_major(90));
        assert_eq!(night.supplements, Money::from_major(25 + 15));
        assert_eq!(night.board, Money::from_major(3 * 12 + 6));
        assert_eq!(night.total, Money::from_major(90 + 40 + 42));
        assert_eq!(q.total, Money::from_major(2 * 172));
    }

    #[test]
    fn parking_is_per_car_per_night_and_part_of_the_total() {
        let stay = DateRange::new(d("2026-05-01"), d("2026-05-03")).unwrap();
        let q = quote_stay(&double(), &[], None, stay, 2, 0, 2, Money::from_major(8));
        let night = &q.nights[0];
        assert_eq!(night.parking, Money::from_major(16));
        assert_eq!(night.total, Money::from_major(90 + 16));
        assert_eq!(q.total, Money::from_major(2 * 106));

        let no_car = quote_stay(&double(), &[], None, stay, 2, 0, 0, Money::from_major(8));
        assert_eq!(no_car.nights[0].parking, Money::ZERO);
    }

    #[test]
    fn rule_validation() {
        let mut r = rule(
            "Summer",
            "2026-07-01",
            "2026-06-30",
            Adjustment::Percent { percent: 5 },
        );
        assert_eq!(
            validate_rate_rule(&r),
            Err(ValidationError::InvalidRuleDates)
        );
        r.end = d("2026-07-01"); // one-night rule is fine
        assert_eq!(validate_rate_rule(&r), Ok(()));
        r.weekdays = vec![0];
        assert_eq!(validate_rate_rule(&r), Err(ValidationError::InvalidWeekday));
    }

    #[test]
    fn board_codes_are_unique() {
        let a = bb();
        let mut b = bb();
        b.id = BoardOptionId::new();
        b.code = "bb".into();
        assert!(matches!(
            validate_board_option(&b, std::slice::from_ref(&a)),
            Err(ValidationError::DuplicateName { .. })
        ));
        assert_eq!(validate_board_option(&a, std::slice::from_ref(&a)), Ok(()));
    }
}
