//! Money as whole minor units (pence/cents) in an `i64`.
//!
//! Floating point can't represent 0.10 exactly, so money never touches
//! `f64`. Percentages use integer maths with explicit rounding.

use std::iter::Sum;
use std::ops::{Add, AddAssign, Mul, Neg, Sub};

use serde::{Deserialize, Serialize};

#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(transparent)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct Money(pub i64);

impl Money {
    pub const ZERO: Money = Money(0);

    /// `Money::from_major(90)` = £90.00.
    pub const fn from_major(units: i64) -> Self {
        Money(units * 100)
    }

    /// Applies a percentage change, e.g. `+10` or `-15`, rounding half away
    /// from zero to the nearest minor unit.
    pub fn adjust_percent(self, percent: i32) -> Self {
        let scaled = i128::from(self.0) * i128::from(100 + percent);
        let rounded = if scaled >= 0 {
            (scaled + 50) / 100
        } else {
            (scaled - 50) / 100
        };
        // A hotel price times a sensible percentage always fits back into i64.
        Money(i64::try_from(rounded).unwrap_or(i64::MAX))
    }

    /// Never below zero (used after discounts).
    pub fn at_least_zero(self) -> Self {
        Money(self.0.max(0))
    }
}

// Implementing these operator traits lets us write `a + b` and `price * nights`.
impl Add for Money {
    type Output = Money;
    fn add(self, rhs: Money) -> Money {
        Money(self.0 + rhs.0)
    }
}

impl AddAssign for Money {
    fn add_assign(&mut self, rhs: Money) {
        self.0 += rhs.0;
    }
}

impl Sub for Money {
    type Output = Money;
    fn sub(self, rhs: Money) -> Money {
        Money(self.0 - rhs.0)
    }
}

impl Neg for Money {
    type Output = Money;
    fn neg(self) -> Money {
        Money(-self.0)
    }
}

impl Mul<i64> for Money {
    type Output = Money;
    fn mul(self, rhs: i64) -> Money {
        Money(self.0 * rhs)
    }
}

/// Lets an iterator of `Money` be `.sum()`med.
impl Sum for Money {
    fn sum<I: Iterator<Item = Money>>(iter: I) -> Money {
        iter.fold(Money::ZERO, Add::add)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arithmetic_and_sum() {
        let a = Money::from_major(90);
        assert_eq!(a + Money(50), Money(9050));
        assert_eq!(a * 3, Money(27000));
        assert_eq!(
            vec![Money(1), Money(2), Money(3)]
                .into_iter()
                .sum::<Money>(),
            Money(6)
        );
        assert_eq!((Money(5) - Money(9)).at_least_zero(), Money::ZERO);
    }

    #[test]
    fn percent_rounds_half_away_from_zero() {
        assert_eq!(Money(9000).adjust_percent(10), Money(9900));
        assert_eq!(Money(9999).adjust_percent(-15), Money(8499)); // 8499.15 -> 8499
        assert_eq!(Money(1).adjust_percent(50), Money(2)); // 1.5 -> 2
        assert_eq!(Money(-1).adjust_percent(50), Money(-2));
        assert_eq!(Money(12345).adjust_percent(-100), Money::ZERO);
    }

    #[test]
    fn serialises_as_a_plain_number() {
        assert_eq!(serde_json::to_string(&Money(9050)).unwrap(), "9050");
    }
}
