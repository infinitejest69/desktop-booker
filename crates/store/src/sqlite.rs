//! SQLite implementation of the repository traits.

use std::path::Path;
use std::str::FromStr;

use booker_core::{
    Floor, FloorId, PropertySettings, RoomType, RoomTypeId, Space, SpaceId, validate_floor,
    validate_floor_delete, validate_room_type, validate_room_type_delete, validate_settings,
    validate_space,
};
use sqlx::SqliteConnection;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePool, SqlitePoolOptions};

use crate::repo::{FloorRepo, RoomTypeRepo, SettingsRepo, SpaceRepo};
use crate::rows::{FloorRow, RoomTypeRow, SettingsRow, SpaceRow, convert_all, kind_str};
use crate::{StoreError, StoreResult};

/// Migrations are compiled into the binary from `crates/store/migrations/`.
static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

/// The app's database. Cheap to clone: the pool inside is reference-counted,
/// so clones share the same connections.
#[derive(Clone)]
pub struct SqliteStore {
    pub(crate) pool: SqlitePool,
}

impl SqliteStore {
    /// Opens (creating if needed) the database file at `path` and applies migrations.
    pub async fn open(path: &Path) -> StoreResult<Self> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)
                .map_err(|e| StoreError::Corrupt(format!("can't create {}: {e}", dir.display())))?;
        }
        let options = SqliteConnectOptions::new()
            .filename(path)
            .create_if_missing(true)
            .foreign_keys(true)
            // WAL = readers don't block the writer; safer if the app crashes mid-write.
            .journal_mode(SqliteJournalMode::Wal)
            .busy_timeout(std::time::Duration::from_secs(5));
        let pool = SqlitePoolOptions::new()
            .max_connections(4)
            .connect_with(options)
            .await?;
        Self::migrated(pool).await
    }

    /// A fresh, empty database in memory. Used by tests.
    pub async fn open_in_memory() -> StoreResult<Self> {
        let options = SqliteConnectOptions::from_str("sqlite::memory:")?.foreign_keys(true);
        // Each in-memory connection is its own separate database, so keep exactly
        // one connection, and never close it while the store is alive.
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .idle_timeout(None)
            .max_lifetime(None)
            .connect_with(options)
            .await?;
        Self::migrated(pool).await
    }

    async fn migrated(pool: SqlitePool) -> StoreResult<Self> {
        MIGRATOR.run(&pool).await?;
        Ok(Self { pool })
    }
}

// ---------------------------------------------------------------------------
// Loaders shared by list methods and by validation inside transactions.
// They take `&mut SqliteConnection` so they work on a plain pooled connection
// *or* on an open transaction (`&mut *tx`).
// ---------------------------------------------------------------------------

async fn load_floors(conn: &mut SqliteConnection) -> StoreResult<Vec<Floor>> {
    let rows: Vec<FloorRow> = sqlx::query_as(
        "SELECT id, name, level, grid_cols, grid_rows FROM floors
         WHERE deleted_at IS NULL ORDER BY level, name",
    )
    .fetch_all(conn)
    .await?;
    convert_all(rows)
}

async fn load_floor(conn: &mut SqliteConnection, id: FloorId) -> StoreResult<Floor> {
    let row: Option<FloorRow> = sqlx::query_as(
        "SELECT id, name, level, grid_cols, grid_rows FROM floors
         WHERE id = ? AND deleted_at IS NULL",
    )
    .bind(id.to_string())
    .fetch_optional(conn)
    .await?;
    row.ok_or(StoreError::NotFound { what: "Floor" })?
        .try_into()
}

pub(crate) async fn load_room_types(conn: &mut SqliteConnection) -> StoreResult<Vec<RoomType>> {
    let rows: Vec<RoomTypeRow> = sqlx::query_as(
        "SELECT id, name, code, default_occupancy, max_occupancy, description,
                base_rate, extra_adult_rate, child_rate
         FROM room_types WHERE deleted_at IS NULL ORDER BY name",
    )
    .fetch_all(conn)
    .await?;
    convert_all(rows)
}

/// The SELECT shared by space queries. A macro (not a `const`) so `concat!`
/// can join it with the rest of the query at compile time. sqlx 0.9 only
/// accepts SQL known at compile time, which guards against SQL injection.
macro_rules! select_spaces {
    () => {
        "SELECT id, floor_id, kind, name, room_type_id,
                cap_theatre, cap_boardroom, cap_banquet, cap_cabaret, cap_reception,
                grid_x, grid_y, grid_w, grid_h, out_of_service, notes
         FROM spaces WHERE deleted_at IS NULL"
    };
}

pub(crate) async fn load_spaces(
    conn: &mut SqliteConnection,
    floor: Option<FloorId>,
) -> StoreResult<Vec<Space>> {
    let rows: Vec<SpaceRow> = match floor {
        Some(floor_id) => {
            sqlx::query_as(concat!(
                select_spaces!(),
                " AND floor_id = ? ORDER BY kind, name"
            ))
            .bind(floor_id.to_string())
            .fetch_all(conn)
            .await?
        }
        None => {
            sqlx::query_as(concat!(select_spaces!(), " ORDER BY kind, name"))
                .fetch_all(conn)
                .await?
        }
    };
    convert_all(rows)
}

// ---------------------------------------------------------------------------
// Repository implementations.
// Every save follows the same shape:
//   begin transaction → load what the rule needs → validate (booker-core)
//   → write → commit.
// If any step fails, the transaction is dropped and rolls back automatically.
// ---------------------------------------------------------------------------

impl SettingsRepo for SqliteStore {
    async fn get_settings(&self) -> StoreResult<PropertySettings> {
        let row: Option<SettingsRow> = sqlx::query_as(
            "SELECT hotel_name, address, phone, email, currency, timezone,
                    check_in_time, check_out_time
             FROM property_settings WHERE id = 1",
        )
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(Into::into).unwrap_or_default())
    }

    async fn save_settings(&self, settings: &PropertySettings) -> StoreResult<()> {
        validate_settings(settings)?;
        sqlx::query(
            "INSERT INTO property_settings
                (id, hotel_name, address, phone, email, currency, timezone, check_in_time, check_out_time)
             VALUES (1, ?, ?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT (id) DO UPDATE SET
                hotel_name = excluded.hotel_name, address = excluded.address,
                phone = excluded.phone, email = excluded.email,
                currency = excluded.currency, timezone = excluded.timezone,
                check_in_time = excluded.check_in_time, check_out_time = excluded.check_out_time,
                updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')",
        )
        .bind(settings.hotel_name.trim())
        .bind(settings.address.trim())
        .bind(settings.phone.trim())
        .bind(settings.email.trim())
        .bind(settings.currency.trim())
        .bind(settings.timezone.trim())
        .bind(settings.check_in_time.trim())
        .bind(settings.check_out_time.trim())
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}

impl FloorRepo for SqliteStore {
    async fn list_floors(&self) -> StoreResult<Vec<Floor>> {
        let mut conn = self.pool.acquire().await?;
        load_floors(&mut conn).await
    }

    async fn save_floor(&self, floor: &Floor) -> StoreResult<()> {
        let mut tx = self.pool.begin().await?;
        let others = load_floors(&mut tx).await?;
        let spaces = load_spaces(&mut tx, Some(floor.id)).await?;
        validate_floor(floor, &others, &spaces)?;
        sqlx::query(
            "INSERT INTO floors (id, name, level, grid_cols, grid_rows) VALUES (?, ?, ?, ?, ?)
             ON CONFLICT (id) DO UPDATE SET
                name = excluded.name, level = excluded.level,
                grid_cols = excluded.grid_cols, grid_rows = excluded.grid_rows,
                updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')",
        )
        .bind(floor.id.to_string())
        .bind(floor.name.trim())
        .bind(floor.level)
        .bind(i64::from(floor.grid_cols))
        .bind(i64::from(floor.grid_rows))
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(())
    }

    async fn delete_floor(&self, id: FloorId) -> StoreResult<()> {
        let mut tx = self.pool.begin().await?;
        let floor = load_floor(&mut tx, id).await?;
        let spaces = load_spaces(&mut tx, Some(id)).await?;
        validate_floor_delete(&floor, &spaces)?;
        sqlx::query(
            "UPDATE floors SET deleted_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                               updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE id = ?",
        )
        .bind(id.to_string())
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(())
    }
}

impl RoomTypeRepo for SqliteStore {
    async fn list_room_types(&self) -> StoreResult<Vec<RoomType>> {
        let mut conn = self.pool.acquire().await?;
        load_room_types(&mut conn).await
    }

    async fn save_room_type(&self, room_type: &RoomType) -> StoreResult<()> {
        let mut tx = self.pool.begin().await?;
        let others = load_room_types(&mut tx).await?;
        validate_room_type(room_type, &others)?;
        sqlx::query(
            "INSERT INTO room_types (id, name, code, default_occupancy, max_occupancy, description,
                                     base_rate, extra_adult_rate, child_rate)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT (id) DO UPDATE SET
                name = excluded.name, code = excluded.code,
                default_occupancy = excluded.default_occupancy,
                max_occupancy = excluded.max_occupancy, description = excluded.description,
                base_rate = excluded.base_rate, extra_adult_rate = excluded.extra_adult_rate,
                child_rate = excluded.child_rate,
                updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')",
        )
        .bind(room_type.id.to_string())
        .bind(room_type.name.trim())
        .bind(room_type.code.trim().to_uppercase())
        .bind(i64::from(room_type.default_occupancy))
        .bind(i64::from(room_type.max_occupancy))
        .bind(room_type.description.trim())
        .bind(room_type.base_rate.0)
        .bind(room_type.extra_adult_rate.0)
        .bind(room_type.child_rate.0)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(())
    }

    async fn delete_room_type(&self, id: RoomTypeId) -> StoreResult<()> {
        let mut tx = self.pool.begin().await?;
        let room_type = load_room_types(&mut tx)
            .await?
            .into_iter()
            .find(|rt| rt.id == id)
            .ok_or(StoreError::NotFound { what: "Room type" })?;
        let spaces = load_spaces(&mut tx, None).await?;
        validate_room_type_delete(&room_type, &spaces)?;
        sqlx::query(
            "UPDATE room_types SET deleted_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                                   updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE id = ?",
        )
        .bind(id.to_string())
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(())
    }
}

impl SpaceRepo for SqliteStore {
    async fn list_spaces(&self, floor: Option<FloorId>) -> StoreResult<Vec<Space>> {
        let mut conn = self.pool.acquire().await?;
        load_spaces(&mut conn, floor).await
    }

    async fn save_space(&self, space: &Space) -> StoreResult<()> {
        let mut tx = self.pool.begin().await?;
        let floor = load_floor(&mut tx, space.floor_id).await?;
        let all = load_spaces(&mut tx, None).await?;
        validate_space(space, &floor, &all)?;
        if let Some(rt) = space.room_type_id {
            let exists = load_room_types(&mut tx).await?.iter().any(|t| t.id == rt);
            if !exists {
                return Err(StoreError::NotFound { what: "Room type" });
            }
        }
        let caps = &space.suite_capacities;
        let cap = |c: Option<u16>| c.map(i64::from);
        sqlx::query(
            "INSERT INTO spaces (id, floor_id, kind, name, room_type_id,
                cap_theatre, cap_boardroom, cap_banquet, cap_cabaret, cap_reception,
                grid_x, grid_y, grid_w, grid_h, out_of_service, notes)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT (id) DO UPDATE SET
                floor_id = excluded.floor_id, kind = excluded.kind, name = excluded.name,
                room_type_id = excluded.room_type_id,
                cap_theatre = excluded.cap_theatre, cap_boardroom = excluded.cap_boardroom,
                cap_banquet = excluded.cap_banquet, cap_cabaret = excluded.cap_cabaret,
                cap_reception = excluded.cap_reception,
                grid_x = excluded.grid_x, grid_y = excluded.grid_y,
                grid_w = excluded.grid_w, grid_h = excluded.grid_h,
                out_of_service = excluded.out_of_service, notes = excluded.notes,
                updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')",
        )
        .bind(space.id.to_string())
        .bind(space.floor_id.to_string())
        .bind(kind_str(space.kind))
        .bind(space.name.trim())
        .bind(space.room_type_id.map(|id| id.to_string()))
        .bind(cap(caps.theatre))
        .bind(cap(caps.boardroom))
        .bind(cap(caps.banquet))
        .bind(cap(caps.cabaret))
        .bind(cap(caps.reception))
        .bind(i64::from(space.rect.x))
        .bind(i64::from(space.rect.y))
        .bind(i64::from(space.rect.w))
        .bind(i64::from(space.rect.h))
        .bind(space.out_of_service)
        .bind(space.notes.trim())
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(())
    }

    async fn delete_space(&self, id: SpaceId) -> StoreResult<()> {
        let result = sqlx::query(
            "UPDATE spaces SET deleted_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                               updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE id = ? AND deleted_at IS NULL",
        )
        .bind(id.to_string())
        .execute(&self.pool)
        .await?;
        if result.rows_affected() == 0 {
            return Err(StoreError::NotFound {
                what: "Room or suite",
            });
        }
        Ok(())
    }
}

impl SqliteStore {
    /// Writes a consistent copy of the database to `dir` as
    /// `booker-YYYYMMDD-HHMMSS.db`, then deletes all but the newest `keep`
    /// backups there. Safe while the app is running (`VACUUM INTO`).
    pub async fn backup_with_rotation(
        &self,
        dir: &Path,
        keep: usize,
    ) -> StoreResult<std::path::PathBuf> {
        std::fs::create_dir_all(dir)
            .map_err(|e| StoreError::Corrupt(format!("can't create {}: {e}", dir.display())))?;
        let stamp = chrono::Local::now().format("%Y%m%d-%H%M%S%.3f");
        let path = dir.join(format!("booker-{stamp}.db"));
        sqlx::query("VACUUM INTO ?")
            .bind(path.to_string_lossy().into_owned())
            .execute(&self.pool)
            .await?;

        let mut backups: Vec<std::path::PathBuf> = std::fs::read_dir(dir)
            .map_err(|e| StoreError::Corrupt(format!("can't read {}: {e}", dir.display())))?
            .filter_map(|entry| entry.ok().map(|e| e.path()))
            .filter(|p| {
                p.file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.starts_with("booker-") && n.ends_with(".db"))
            })
            .collect();
        // Names sort by time because the timestamp is zero-padded, newest last.
        backups.sort();
        let excess = backups.len().saturating_sub(keep);
        for old in backups.into_iter().take(excess) {
            let _ = std::fs::remove_file(old);
        }
        Ok(path)
    }
}
