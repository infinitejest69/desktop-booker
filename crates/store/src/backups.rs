//! Backups: settings, taking a copy, rotation and restoring a file.
//!
//! A backup is a complete SQLite file written with `VACUUM INTO`, which is
//! safe while the app is running. Restoring replaces the live database file,
//! so it must happen with the database closed (the app restarts afterwards).

use std::path::Path;

use booker_core::{
    BackupInfo, BackupSettings, backup_file_name, backups_to_delete, parse_backup_name,
};

use crate::audit::{Change, json, record};
use crate::repo::BackupRepo;
use crate::rows::int;
use crate::{SqliteStore, StoreError, StoreResult};

fn io(what: &str, path: &Path, e: std::io::Error) -> StoreError {
    StoreError::Corrupt(format!("can't {what} {}: {e}", path.display()))
}

/// The backups in `dir`, newest first. A missing folder has none.
pub fn list_backups(dir: &Path) -> StoreResult<Vec<BackupInfo>> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Ok(Vec::new());
    };
    let mut out: Vec<BackupInfo> = entries
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            let taken_at = parse_backup_name(&name)?;
            let size_bytes = entry.metadata().map(|m| m.len()).unwrap_or(0);
            Some(BackupInfo {
                name,
                taken_at,
                size_bytes,
            })
        })
        .collect();
    out.sort_by_key(|b| std::cmp::Reverse(b.taken_at));
    Ok(out)
}

/// Puts a backup in place of the database at `db`. Call only with the
/// database closed. The `-wal` and `-shm` side files belong to the old
/// database, so they're removed too.
pub fn restore_file(backup: &Path, db: &Path) -> StoreResult<()> {
    // A real SQLite file starts with this 16-byte header.
    let header = std::fs::read(backup).map_err(|e| io("read", backup, e))?;
    if !header.starts_with(b"SQLite format 3\0") {
        return Err(StoreError::Corrupt(format!(
            "{} isn't a DesktopBooker backup",
            backup.display()
        )));
    }
    for side in ["-wal", "-shm"] {
        let mut p = db.as_os_str().to_owned();
        p.push(side);
        let _ = std::fs::remove_file(std::path::PathBuf::from(p));
    }
    std::fs::copy(backup, db).map_err(|e| io("restore", db, e))?;
    Ok(())
}

impl SqliteStore {
    /// Closes every connection, e.g. before a restore replaces the file.
    pub async fn close(&self) {
        self.pool.close().await;
    }
}

impl BackupRepo for SqliteStore {
    async fn get_backup_settings(&self) -> StoreResult<BackupSettings> {
        let row: Option<(String, i64, i64)> = sqlx::query_as(
            "SELECT folder, keep_daily, keep_monthly FROM backup_settings WHERE id = 1",
        )
        .fetch_optional(&self.pool)
        .await?;
        let Some((folder, daily, monthly)) = row else {
            return Ok(BackupSettings::default());
        };
        Ok(BackupSettings {
            folder,
            keep_daily: int(daily, "backup_settings.keep_daily")?,
            keep_monthly: int(monthly, "backup_settings.keep_monthly")?,
        })
    }

    async fn save_backup_settings(&self, settings: &BackupSettings) -> StoreResult<()> {
        let mut tx = self.pool.begin().await?;
        sqlx::query(
            "INSERT INTO backup_settings (id, folder, keep_daily, keep_monthly) VALUES (1, ?, ?, ?)
             ON CONFLICT (id) DO UPDATE SET folder = excluded.folder,
                keep_daily = excluded.keep_daily, keep_monthly = excluded.keep_monthly,
                updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')",
        )
        .bind(settings.folder.trim())
        .bind(i64::from(settings.keep_daily))
        .bind(i64::from(settings.keep_monthly))
        .execute(&mut *tx)
        .await?;
        record(
            &mut tx,
            Change {
                entity: "backup_settings",
                entity_id: "1".into(),
                action: "updated",
                summary: "Changed the backup settings".into(),
                before: None,
                after: json(settings),
                actor: self.actor.as_ref(),
            },
        )
        .await?;
        tx.commit().await?;
        Ok(())
    }

    async fn backup_now(&self, dir: &Path, settings: &BackupSettings) -> StoreResult<BackupInfo> {
        std::fs::create_dir_all(dir).map_err(|e| io("create", dir, e))?;
        let now = chrono::Local::now().naive_local();
        let name = backup_file_name(now);
        let path = dir.join(&name);
        sqlx::query("VACUUM INTO ?")
            .bind(path.to_string_lossy().into_owned())
            .execute(&self.pool)
            .await?;
        let backups = list_backups(dir)?;
        for old in backups_to_delete(&backups, settings) {
            let _ = std::fs::remove_file(dir.join(old));
        }
        let size_bytes = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
        Ok(BackupInfo {
            taken_at: parse_backup_name(&name).unwrap_or(now),
            name,
            size_bytes,
        })
    }
}
