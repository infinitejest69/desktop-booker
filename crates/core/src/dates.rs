//! Stays as half-open date ranges: `[start, end)`.
//!
//! A stay arriving on the 10th and leaving on the 12th covers the *nights* of
//! the 10th and 11th. The departure day is free for the next guest, so a
//! same-day turnover never counts as an overlap.

use chrono::{Datelike, Days, NaiveDate};
use serde::{Deserialize, Serialize};

use crate::validation::ValidationError;

/// Longest stay we accept in one booked room (a sanity limit, not a policy).
pub const MAX_NIGHTS: u32 = 365;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct DateRange {
    /// Arrival date (first night).
    pub start: NaiveDate,
    /// Departure date (not a night of the stay).
    pub end: NaiveDate,
}

impl DateRange {
    /// Builds a range, rejecting zero-length, backwards or absurdly long stays.
    pub fn new(start: NaiveDate, end: NaiveDate) -> Result<Self, ValidationError> {
        let range = DateRange { start, end };
        range.validate()?;
        Ok(range)
    }

    pub fn validate(&self) -> Result<(), ValidationError> {
        let n = self.nights_count();
        if n <= 0 || n > i64::from(MAX_NIGHTS) {
            return Err(ValidationError::InvalidStay);
        }
        Ok(())
    }

    /// Number of nights (≤ 0 for an invalid range).
    pub fn nights_count(&self) -> i64 {
        (self.end - self.start).num_days()
    }

    /// Each night of the stay, in order. `impl Iterator` means "some iterator
    /// type" without naming it; callers can loop over it or `.collect()` it.
    pub fn nights(&self) -> impl Iterator<Item = NaiveDate> + '_ {
        self.start.iter_days().take_while(move |d| *d < self.end)
    }

    pub fn contains_night(&self, night: NaiveDate) -> bool {
        self.start <= night && night < self.end
    }

    /// True if the two stays share at least one night.
    pub fn overlaps(&self, other: &DateRange) -> bool {
        self.start < other.end && other.start < self.end
    }

    /// The range shifted by whole days (used when moving a booking on the chart).
    pub fn shifted(&self, days: i64) -> Option<DateRange> {
        let shift = |d: NaiveDate| {
            let n = Days::new(days.unsigned_abs());
            if days >= 0 {
                d.checked_add_days(n)
            } else {
                d.checked_sub_days(n)
            }
        };
        Some(DateRange {
            start: shift(self.start)?,
            end: shift(self.end)?,
        })
    }
}

/// ISO weekday number: Monday = 1 … Sunday = 7.
pub fn iso_weekday(date: NaiveDate) -> u8 {
    // `number_from_monday` is 1..=7, so the cast can't truncate.
    date.weekday().number_from_monday() as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(s: &str) -> NaiveDate {
        s.parse().unwrap()
    }

    fn r(a: &str, b: &str) -> DateRange {
        DateRange::new(d(a), d(b)).unwrap()
    }

    #[test]
    fn rejects_empty_backwards_and_huge_stays() {
        assert_eq!(
            DateRange::new(d("2026-05-10"), d("2026-05-10")),
            Err(ValidationError::InvalidStay)
        );
        assert_eq!(
            DateRange::new(d("2026-05-10"), d("2026-05-09")),
            Err(ValidationError::InvalidStay)
        );
        assert!(DateRange::new(d("2026-01-01"), d("2027-01-02")).is_err());
    }

    #[test]
    fn nights_are_arrival_up_to_but_not_including_departure() {
        let stay = r("2026-12-30", "2027-01-02");
        let nights: Vec<_> = stay.nights().collect();
        assert_eq!(
            nights,
            vec![d("2026-12-30"), d("2026-12-31"), d("2027-01-01")]
        );
        assert_eq!(stay.nights_count(), 3);
        assert!(stay.contains_night(d("2027-01-01")));
        assert!(!stay.contains_night(d("2027-01-02")));
    }

    #[test]
    fn same_day_turnover_is_not_an_overlap() {
        let first = r("2026-05-10", "2026-05-12");
        let next = r("2026-05-12", "2026-05-14");
        assert!(!first.overlaps(&next));
        assert!(!next.overlaps(&first));
        assert!(first.overlaps(&r("2026-05-11", "2026-05-13")));
        assert!(first.overlaps(&r("2026-05-01", "2026-06-01")));
    }

    #[test]
    fn shifting_moves_both_ends() {
        assert_eq!(
            r("2026-05-10", "2026-05-12").shifted(-3),
            Some(r("2026-05-07", "2026-05-09"))
        );
    }

    #[test]
    fn iso_weekdays() {
        assert_eq!(iso_weekday(d("2026-11-23")), 1); // Monday
        assert_eq!(iso_weekday(d("2026-11-29")), 7); // Sunday
    }
}
