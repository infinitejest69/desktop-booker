//! Day-to-day operations: housekeeping, reports, CSV export and erasing a
//! guest's personal data (GDPR).

use chrono::{Datelike, NaiveDate};
use serde::{Deserialize, Serialize};

use crate::booking::{BookingStatus, Guest};
use crate::dates::DateRange;
use crate::ids::SpaceId;
use crate::money::Money;

// ---------------------------------------------------------------------------
// Housekeeping
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub enum HousekeepingStatus {
    /// Cleaned, waiting to be checked.
    Clean,
    /// Needs cleaning (e.g. after a check-out).
    Dirty,
    /// Cleaned and checked: ready to let.
    #[default]
    Inspected,
}

impl HousekeepingStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Clean => "clean",
            Self::Dirty => "dirty",
            Self::Inspected => "inspected",
        }
    }
}

impl std::str::FromStr for HousekeepingStatus {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, String> {
        [Self::Clean, Self::Dirty, Self::Inspected]
            .into_iter()
            .find(|x| x.as_str() == s)
            .ok_or_else(|| format!("unknown housekeeping status {s:?}"))
    }
}

/// A room's housekeeping state as the board shows it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct RoomHousekeeping {
    pub space_id: SpaceId,
    pub status: HousekeepingStatus,
    pub note: String,
}

/// What a booking status change does to its rooms: a check-out leaves
/// them dirty.
pub fn housekeeping_after(status: BookingStatus) -> Option<HousekeepingStatus> {
    (status == BookingStatus::CheckedOut).then_some(HousekeepingStatus::Dirty)
}

/// A room's housekeeping state today. Someone slept in it last night
/// (still in house, or checked out this morning) means it needs doing,
/// unless housekeeping has marked it since the day began.
pub fn housekeeping_today(
    saved: HousekeepingStatus,
    saved_on: Option<NaiveDate>,
    slept_in_last_night: bool,
    today: NaiveDate,
) -> HousekeepingStatus {
    if slept_in_last_night && saved_on.is_none_or(|d| d < today) {
        HousekeepingStatus::Dirty
    } else {
        saved
    }
}

// ---------------------------------------------------------------------------
// Reports
// ---------------------------------------------------------------------------

/// How full the hotel was on one night (or, from `occupancy_total`, a period).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct DayOccupancy {
    pub date: NaiveDate,
    pub occupied: u32,
    pub rooms: u32,
    /// Tenths of a percent (853 = 85.3%), so it stays an integer.
    pub per_mille: u32,
}

fn per_mille(part: u32, whole: u32) -> u32 {
    if whole == 0 {
        return 0;
    }
    // Round to the nearest tenth of a percent.
    let value = (u64::from(part) * 1000 + u64::from(whole) / 2) / u64::from(whole);
    u32::try_from(value).unwrap_or(u32::MAX)
}

/// Occupancy for each night of `period`. `nights` lists one entry per
/// room sold per night (from `room_nights`); `rooms` is how many bedrooms
/// are in service.
pub fn occupancy_by_day(
    period: DateRange,
    rooms: u32,
    nights: &[(NaiveDate, SpaceId)],
) -> Vec<DayOccupancy> {
    period
        .nights()
        .map(|date| {
            let sold = nights.iter().filter(|(n, _)| *n == date).count();
            let occupied = u32::try_from(sold).unwrap_or(u32::MAX);
            DayOccupancy {
                date,
                occupied,
                rooms,
                per_mille: per_mille(occupied, rooms),
            }
        })
        .collect()
}

/// Room-nights sold over room-nights available, for a whole period.
pub fn occupancy_total(days: &[DayOccupancy]) -> DayOccupancy {
    let occupied = days.iter().map(|d| d.occupied).sum();
    let rooms = days.iter().map(|d| d.rooms).sum();
    DayOccupancy {
        date: days.first().map_or(NaiveDate::MIN, |d| d.date),
        occupied,
        rooms,
        per_mille: per_mille(occupied, rooms),
    }
}

/// The two standard hotel measures for a period.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct RoomKpis {
    pub rooms_sold: u32,
    pub rooms_available: u32,
    pub room_revenue: Money,
    /// Average daily rate: room revenue per room-night sold.
    pub adr: Money,
    /// Revenue per available room: room revenue per room-night available.
    pub revpar: Money,
}

fn per(total: Money, count: u32) -> Money {
    if count == 0 {
        return Money::ZERO;
    }
    // Round to the nearest penny.
    let c = i64::from(count);
    Money((total.0 * 2 + c) / (2 * c))
}

/// ADR and RevPAR from a period's occupancy and its room revenue (rooms
/// only, not board, suites or extras).
pub fn room_kpis(days: &[DayOccupancy], room_revenue: Money) -> RoomKpis {
    let total = occupancy_total(days);
    RoomKpis {
        rooms_sold: total.occupied,
        rooms_available: total.rooms,
        room_revenue,
        adr: per(room_revenue, total.occupied),
        revpar: per(room_revenue, total.rooms),
    }
}

/// Revenue for one month, by where it came from. Gross amounts, as charged.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct MonthRevenue {
    pub year: i32,
    pub month: u32,
    pub rooms: Money,
    pub board: Money,
    pub suites: Money,
    pub extras: Money,
    pub total: Money,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RevenueSource {
    Rooms,
    Board,
    Suites,
    Extras,
}

/// Adds up dated amounts into one row per month, oldest first. Months
/// with nothing are left out.
pub fn revenue_by_month(items: &[(NaiveDate, RevenueSource, Money)]) -> Vec<MonthRevenue> {
    let mut rows: Vec<MonthRevenue> = Vec::new();
    for (date, source, amount) in items {
        let (year, month) = (date.year(), date.month());
        let i = match rows.iter().position(|r| r.year == year && r.month == month) {
            Some(i) => i,
            None => {
                rows.push(MonthRevenue {
                    year,
                    month,
                    ..MonthRevenue::default()
                });
                rows.len() - 1
            }
        };
        let row = &mut rows[i];
        match source {
            RevenueSource::Rooms => row.rooms += *amount,
            RevenueSource::Board => row.board += *amount,
            RevenueSource::Suites => row.suites += *amount,
            RevenueSource::Extras => row.extras += *amount,
        }
        row.total += *amount;
    }
    rows.sort_by_key(|r| (r.year, r.month));
    rows
}

// ---------------------------------------------------------------------------
// CSV
// ---------------------------------------------------------------------------

/// A CSV file's text: a header row then the rows, quoting fields that
/// contain commas, quotes or line breaks (RFC 4180).
pub fn to_csv(headers: &[&str], rows: &[Vec<String>]) -> String {
    fn field(s: &str) -> String {
        if s.contains([',', '"', '\n', '\r']) {
            format!("\"{}\"", s.replace('"', "\"\""))
        } else {
            s.to_string()
        }
    }
    fn line<S: AsRef<str>>(cells: &[S]) -> String {
        let fields: Vec<String> = cells.iter().map(|c| field(c.as_ref())).collect();
        fields.join(",") + "\r\n"
    }
    std::iter::once(line(headers))
        .chain(rows.iter().map(|r| line(r)))
        .collect()
}

/// Pence as plain pounds for spreadsheets: 12345 → "123.45".
pub fn csv_money(m: Money) -> String {
    let sign = if m.0 < 0 { "-" } else { "" };
    format!("{sign}{}.{:02}", m.0.abs() / 100, m.0.abs() % 100)
}

// ---------------------------------------------------------------------------
// Guests
// ---------------------------------------------------------------------------

/// A guest's record with their stays and what they've spent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct GuestProfile {
    pub guest: Guest,
    /// Newest first.
    pub stays: Vec<crate::booking::BookingListItem>,
    /// Bookings (not cancelled or no-show) plus extras charged to them.
    pub total_spend: Money,
}

// ---------------------------------------------------------------------------
// GDPR
// ---------------------------------------------------------------------------

/// Two records for the same person combined: `keep`'s details win, its
/// blanks are filled from `duplicate`, and both sets of notes are kept.
pub fn merged_guest(keep: &Guest, duplicate: &Guest) -> Guest {
    let pick = |a: &str, b: &str| {
        if a.trim().is_empty() {
            b.trim().to_string()
        } else {
            a.trim().to_string()
        }
    };
    let notes = match (keep.notes.trim(), duplicate.notes.trim()) {
        (a, "") => a.to_string(),
        ("", b) => b.to_string(),
        (a, b) if a == b => a.to_string(),
        (a, b) => format!("{a}\n{b}"),
    };
    Guest {
        id: keep.id,
        first_name: pick(&keep.first_name, &duplicate.first_name),
        last_name: pick(&keep.last_name, &duplicate.last_name),
        email: pick(&keep.email, &duplicate.email),
        phone: pick(&keep.phone, &duplicate.phone),
        address: pick(&keep.address, &duplicate.address),
        notes,
    }
}

/// The guest with every personal detail removed. Bookings and invoices keep
/// pointing at the record (so totals and reports still add up) but it no
/// longer identifies anyone.
pub fn anonymised(guest: &Guest) -> Guest {
    Guest {
        id: guest.id,
        first_name: String::new(),
        last_name: "Erased guest".into(),
        email: String::new(),
        phone: String::new(),
        address: String::new(),
        notes: String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::GuestId;

    fn d(s: &str) -> NaiveDate {
        s.parse().unwrap()
    }

    #[test]
    fn checking_out_leaves_the_room_dirty() {
        assert_eq!(
            housekeeping_after(BookingStatus::CheckedOut),
            Some(HousekeepingStatus::Dirty)
        );
        assert_eq!(housekeeping_after(BookingStatus::CheckedIn), None);
        assert_eq!("inspected".parse(), Ok(HousekeepingStatus::Inspected));
    }

    #[test]
    fn rooms_slept_in_are_dirty_each_morning_until_marked() {
        use HousekeepingStatus::*;
        let today = d("2026-06-02");
        // Slept in, marked ready yesterday: needs doing again today.
        assert_eq!(
            housekeeping_today(Inspected, Some(d("2026-06-01")), true, today),
            Dirty
        );
        // Slept in but already cleaned this morning: stays clean.
        assert_eq!(housekeeping_today(Clean, Some(today), true, today), Clean);
        // Never marked and slept in: dirty.
        assert_eq!(housekeeping_today(Inspected, None, true, today), Dirty);
        // Empty last night: whatever was last set.
        assert_eq!(
            housekeeping_today(Inspected, Some(d("2026-05-20")), false, today),
            Inspected
        );
    }

    #[test]
    fn occupancy_matches_a_hand_count() {
        // 3 rooms over 4 nights: 2, 3, 0 and 1 sold = 6 of 12 = 50%.
        let (a, b, c) = (SpaceId::new(), SpaceId::new(), SpaceId::new());
        let nights = [
            (d("2026-06-01"), a),
            (d("2026-06-01"), b),
            (d("2026-06-02"), a),
            (d("2026-06-02"), b),
            (d("2026-06-02"), c),
            (d("2026-06-04"), c),
            // Outside the period: ignored.
            (d("2026-06-05"), a),
        ];
        let days = occupancy_by_day(
            DateRange::new(d("2026-06-01"), d("2026-06-05")).unwrap(),
            3,
            &nights,
        );
        let per_day: Vec<_> = days.iter().map(|x| (x.occupied, x.per_mille)).collect();
        assert_eq!(per_day, vec![(2, 667), (3, 1000), (0, 0), (1, 333)]);
        let total = occupancy_total(&days);
        assert_eq!((total.occupied, total.rooms, total.per_mille), (6, 12, 500));
        // No rooms at all is 0%, not a division by zero.
        assert_eq!(per_mille(0, 0), 0);
    }

    #[test]
    fn adr_and_revpar_divide_room_revenue_by_sold_and_available() {
        // 6 of 12 room-nights sold for £570: ADR £95, RevPAR £47.50.
        let (a, b, c) = (SpaceId::new(), SpaceId::new(), SpaceId::new());
        let nights = [
            (d("2026-06-01"), a),
            (d("2026-06-01"), b),
            (d("2026-06-02"), a),
            (d("2026-06-02"), b),
            (d("2026-06-02"), c),
            (d("2026-06-04"), c),
        ];
        let days = occupancy_by_day(
            DateRange::new(d("2026-06-01"), d("2026-06-05")).unwrap(),
            3,
            &nights,
        );
        let k = room_kpis(&days, Money(57_000));
        assert_eq!((k.rooms_sold, k.rooms_available), (6, 12));
        assert_eq!(k.adr, Money(9_500));
        assert_eq!(k.revpar, Money(4_750));
        assert_eq!(room_kpis(&[], Money(100)).adr, Money::ZERO);
    }

    #[test]
    fn revenue_is_grouped_by_month_and_source() {
        let rows = revenue_by_month(&[
            (d("2026-07-02"), RevenueSource::Suites, Money(30_000)),
            (d("2026-06-30"), RevenueSource::Rooms, Money(9_500)),
            (d("2026-06-01"), RevenueSource::Rooms, Money(9_500)),
            (d("2026-06-01"), RevenueSource::Board, Money(2_400)),
            (d("2026-06-15"), RevenueSource::Extras, Money(1_250)),
        ]);
        assert_eq!(rows.len(), 2);
        assert_eq!((rows[0].year, rows[0].month), (2026, 6));
        assert_eq!(rows[0].rooms, Money(19_000));
        assert_eq!(rows[0].board, Money(2_400));
        assert_eq!(rows[0].extras, Money(1_250));
        assert_eq!(rows[0].total, Money(22_650));
        assert_eq!(rows[1].suites, Money(30_000));
    }

    #[test]
    fn csv_quotes_awkward_fields() {
        let csv = to_csv(
            &["Name", "Notes", "Spend"],
            &[vec![
                "Hughes, Jo".into(),
                "Said \"hi\"".into(),
                csv_money(Money(-1_205)),
            ]],
        );
        assert_eq!(
            csv,
            "Name,Notes,Spend\r\n\"Hughes, Jo\",\"Said \"\"hi\"\"\",-12.05\r\n"
        );
    }

    #[test]
    fn merging_fills_blanks_and_keeps_both_notes() {
        let keep = Guest {
            id: GuestId::new(),
            first_name: "Jo".into(),
            last_name: "Hughes".into(),
            email: String::new(),
            phone: "07700 900123".into(),
            address: String::new(),
            notes: "Ground floor".into(),
        };
        let dup = Guest {
            id: GuestId::new(),
            first_name: "Joanne".into(),
            last_name: "Hughes".into(),
            email: "jo@example.com".into(),
            phone: "01234 567890".into(),
            address: "1 High St".into(),
            notes: "Vegetarian".into(),
        };
        let m = merged_guest(&keep, &dup);
        assert_eq!(m.id, keep.id);
        assert_eq!(m.first_name, "Jo");
        assert_eq!(m.email, "jo@example.com");
        assert_eq!(m.phone, "07700 900123");
        assert_eq!(m.address, "1 High St");
        assert_eq!(m.notes, "Ground floor\nVegetarian");
    }

    #[test]
    fn erasing_a_guest_keeps_only_the_id() {
        let g = Guest {
            id: GuestId::new(),
            first_name: "Jo".into(),
            last_name: "Hughes".into(),
            email: "jo@example.com".into(),
            phone: "07700 900123".into(),
            address: "1 High St".into(),
            notes: "Allergic to feathers".into(),
        };
        let e = anonymised(&g);
        assert_eq!(e.id, g.id);
        assert_eq!(e.display_name(), "Erased guest");
        assert!(e.email.is_empty() && e.phone.is_empty() && e.address.is_empty());
        assert!(e.notes.is_empty());
    }
}
