//! Backup naming, rotation and "is it overdue?" rules. The copying itself
//! lives in the store (SQLite `VACUUM INTO`, safe while the app runs).

use chrono::{Datelike, Duration, NaiveDateTime};
use serde::{Deserialize, Serialize};

/// Where backups go and how many to keep.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct BackupSettings {
    /// A folder on a USB stick, OneDrive, Dropbox or NAS. Empty means the
    /// app's own data folder (which doesn't protect against a dead disk).
    pub folder: String,
    /// One backup per day for this many days…
    pub keep_daily: u16,
    /// …and one per month for this many months.
    pub keep_monthly: u16,
}

impl Default for BackupSettings {
    fn default() -> Self {
        Self {
            folder: String::new(),
            keep_daily: 14,
            keep_monthly: 12,
        }
    }
}

/// One backup file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct BackupInfo {
    pub name: String,
    pub taken_at: NaiveDateTime,
    pub size_bytes: u64,
}

/// What the backups page and the dashboard warning show.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct BackupStatus {
    pub settings: BackupSettings,
    /// The folder actually used.
    pub folder: String,
    /// False if the folder can't be reached (USB stick unplugged…).
    pub folder_ok: bool,
    pub last: Option<NaiveDateTime>,
    pub overdue: bool,
    /// Newest first.
    pub backups: Vec<BackupInfo>,
}

const PREFIX: &str = "booker-";
const STAMP: &str = "%Y%m%d-%H%M%S";

/// e.g. "booker-20261121-183005.db".
pub fn backup_file_name(at: NaiveDateTime) -> String {
    format!("{PREFIX}{}.db", at.format(STAMP))
}

/// When a backup was taken, from its file name; `None` for other files.
/// Also reads the older "booker-20261121-183005.123.db" names.
pub fn parse_backup_name(name: &str) -> Option<NaiveDateTime> {
    let stem = name.strip_prefix(PREFIX)?.strip_suffix(".db")?;
    let stamp = stem.get(..15)?;
    NaiveDateTime::parse_from_str(stamp, STAMP).ok()
}

/// A backup is overdue if there's none, or the newest is over 26 hours old
/// (a day plus a little slack for the app being opened later than usual).
pub fn backup_overdue(last: Option<NaiveDateTime>, now: NaiveDateTime) -> bool {
    last.is_none_or(|t| now - t > Duration::hours(26))
}

/// Which backups to delete: keep the newest backup of each of the most
/// recent `keep_daily` days and of each of the most recent `keep_monthly`
/// months; everything else goes. The newest backup is always kept.
pub fn backups_to_delete(backups: &[BackupInfo], settings: &BackupSettings) -> Vec<String> {
    let mut newest_first: Vec<&BackupInfo> = backups.iter().collect();
    newest_first.sort_by_key(|b| std::cmp::Reverse(b.taken_at));

    let mut keep: Vec<&str> = Vec::new();
    let mut days_seen: Vec<chrono::NaiveDate> = Vec::new();
    let mut months_seen: Vec<(i32, u32)> = Vec::new();
    for b in &newest_first {
        let day = b.taken_at.date();
        let month = (day.year(), day.month());
        let mut kept = false;
        if !days_seen.contains(&day) && days_seen.len() < usize::from(settings.keep_daily) {
            days_seen.push(day);
            kept = true;
        }
        if !months_seen.contains(&month) && months_seen.len() < usize::from(settings.keep_monthly) {
            months_seen.push(month);
            kept = true;
        }
        if kept {
            keep.push(&b.name);
        }
    }
    if let Some(newest) = newest_first.first() {
        keep.push(&newest.name);
    }
    newest_first
        .iter()
        .filter(|b| !keep.contains(&b.name.as_str()))
        .map(|b| b.name.clone())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(s: &str) -> NaiveDateTime {
        NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M").unwrap()
    }

    fn info(at: &str) -> BackupInfo {
        BackupInfo {
            name: backup_file_name(t(at)),
            taken_at: t(at),
            size_bytes: 1,
        }
    }

    #[test]
    fn names_round_trip_and_other_files_are_ignored() {
        let at = t("2026-11-21 18:30");
        assert_eq!(backup_file_name(at), "booker-20261121-183000.db");
        assert_eq!(parse_backup_name("booker-20261121-183000.db"), Some(at));
        assert_eq!(parse_backup_name("booker-20261121-183000.123.db"), Some(at));
        assert_eq!(parse_backup_name("notes.txt"), None);
        assert_eq!(parse_backup_name("booker-garbage.db"), None);
    }

    #[test]
    fn a_backup_is_overdue_after_26_hours_or_if_there_is_none() {
        let now = t("2026-11-21 12:00");
        assert!(backup_overdue(None, now));
        assert!(!backup_overdue(Some(t("2026-11-20 11:00")), now));
        assert!(backup_overdue(Some(t("2026-11-20 09:59")), now));
    }

    #[test]
    fn rotation_keeps_daily_then_monthly_backups() {
        let settings = BackupSettings {
            folder: String::new(),
            keep_daily: 2,
            keep_monthly: 2,
        };
        let backups = [
            info("2026-11-21 18:00"),
            info("2026-11-21 09:00"), // second of the day: goes
            info("2026-11-20 18:00"), // second day: kept
            info("2026-11-19 18:00"), // third day, same month: goes
            info("2026-10-31 18:00"), // newest in October: kept as monthly
            info("2026-10-01 18:00"), // goes
            info("2026-09-30 18:00"), // third month: goes
        ];
        let mut gone = backups_to_delete(&backups, &settings);
        gone.sort();
        let mut expected: Vec<String> = [
            "2026-11-21 09:00",
            "2026-11-19 18:00",
            "2026-10-01 18:00",
            "2026-09-30 18:00",
        ]
        .iter()
        .map(|s| backup_file_name(t(s)))
        .collect();
        expected.sort();
        assert_eq!(gone, expected);
    }

    #[test]
    fn the_newest_backup_always_survives() {
        let none = BackupSettings {
            folder: String::new(),
            keep_daily: 0,
            keep_monthly: 0,
        };
        let backups = [info("2026-11-21 18:00"), info("2026-11-20 18:00")];
        assert_eq!(
            backups_to_delete(&backups, &none),
            vec![backup_file_name(t("2026-11-20 18:00"))]
        );
    }
}
