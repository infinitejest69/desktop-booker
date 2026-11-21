//! Raw database rows and their conversion into domain types.
//!
//! SQLite stores IDs as text and numbers as 64-bit integers. These private
//! `*Row` structs mirror the columns exactly (`sqlx::FromRow` fills them),
//! and `TryFrom` turns them into `booker_core` types, failing cleanly with
//! `StoreError::Corrupt` instead of panicking if the data is odd.

use booker_core::{
    Floor, FloorId, GridRect, PropertySettings, RoomType, RoomTypeId, Space, SpaceId, SpaceKind,
    SuiteCapacities,
};
use uuid::Uuid;

use crate::{StoreError, StoreResult};

fn uuid(s: &str) -> StoreResult<Uuid> {
    Uuid::parse_str(s).map_err(|e| StoreError::Corrupt(format!("bad id {s:?}: {e}")))
}

/// Converts an `i64` column into a smaller integer type, e.g. `u16`.
/// The generic `T: TryFrom<i64>` means "any type that can be fallibly made from an i64".
fn int<T: TryFrom<i64>>(value: i64, column: &'static str) -> StoreResult<T> {
    T::try_from(value).map_err(|_| StoreError::Corrupt(format!("{column} out of range: {value}")))
}

fn opt_int<T: TryFrom<i64>>(value: Option<i64>, column: &'static str) -> StoreResult<Option<T>> {
    // `transpose` flips Option<Result<T>> into Result<Option<T>> so `?` works.
    value.map(|v| int(v, column)).transpose()
}

#[derive(sqlx::FromRow)]
pub(crate) struct SettingsRow {
    hotel_name: String,
    address: String,
    phone: String,
    email: String,
    currency: String,
    timezone: String,
}

impl From<SettingsRow> for PropertySettings {
    fn from(r: SettingsRow) -> Self {
        Self {
            hotel_name: r.hotel_name,
            address: r.address,
            phone: r.phone,
            email: r.email,
            currency: r.currency,
            timezone: r.timezone,
        }
    }
}

#[derive(sqlx::FromRow)]
pub(crate) struct FloorRow {
    id: String,
    name: String,
    level: i64,
    grid_cols: i64,
    grid_rows: i64,
}

impl TryFrom<FloorRow> for Floor {
    type Error = StoreError;
    fn try_from(r: FloorRow) -> StoreResult<Self> {
        Ok(Self {
            id: FloorId(uuid(&r.id)?),
            name: r.name,
            level: int(r.level, "floors.level")?,
            grid_cols: int(r.grid_cols, "floors.grid_cols")?,
            grid_rows: int(r.grid_rows, "floors.grid_rows")?,
        })
    }
}

#[derive(sqlx::FromRow)]
pub(crate) struct RoomTypeRow {
    id: String,
    name: String,
    code: String,
    default_occupancy: i64,
    max_occupancy: i64,
    description: String,
}

impl TryFrom<RoomTypeRow> for RoomType {
    type Error = StoreError;
    fn try_from(r: RoomTypeRow) -> StoreResult<Self> {
        Ok(Self {
            id: RoomTypeId(uuid(&r.id)?),
            name: r.name,
            code: r.code,
            default_occupancy: int(r.default_occupancy, "room_types.default_occupancy")?,
            max_occupancy: int(r.max_occupancy, "room_types.max_occupancy")?,
            description: r.description,
        })
    }
}

#[derive(sqlx::FromRow)]
pub(crate) struct SpaceRow {
    id: String,
    floor_id: String,
    kind: String,
    name: String,
    room_type_id: Option<String>,
    cap_theatre: Option<i64>,
    cap_boardroom: Option<i64>,
    cap_banquet: Option<i64>,
    cap_cabaret: Option<i64>,
    cap_reception: Option<i64>,
    grid_x: i64,
    grid_y: i64,
    grid_w: i64,
    grid_h: i64,
    out_of_service: bool,
    notes: String,
}

impl TryFrom<SpaceRow> for Space {
    type Error = StoreError;
    fn try_from(r: SpaceRow) -> StoreResult<Self> {
        let kind = match r.kind.as_str() {
            "room" => SpaceKind::Room,
            "suite" => SpaceKind::Suite,
            other => return Err(StoreError::Corrupt(format!("unknown space kind {other:?}"))),
        };
        Ok(Self {
            id: SpaceId(uuid(&r.id)?),
            floor_id: FloorId(uuid(&r.floor_id)?),
            kind,
            name: r.name,
            room_type_id: r
                .room_type_id
                .as_deref()
                .map(uuid)
                .transpose()?
                .map(RoomTypeId),
            suite_capacities: SuiteCapacities {
                theatre: opt_int(r.cap_theatre, "spaces.cap_theatre")?,
                boardroom: opt_int(r.cap_boardroom, "spaces.cap_boardroom")?,
                banquet: opt_int(r.cap_banquet, "spaces.cap_banquet")?,
                cabaret: opt_int(r.cap_cabaret, "spaces.cap_cabaret")?,
                reception: opt_int(r.cap_reception, "spaces.cap_reception")?,
            },
            rect: GridRect {
                x: int(r.grid_x, "spaces.grid_x")?,
                y: int(r.grid_y, "spaces.grid_y")?,
                w: int(r.grid_w, "spaces.grid_w")?,
                h: int(r.grid_h, "spaces.grid_h")?,
            },
            out_of_service: r.out_of_service,
            notes: r.notes,
        })
    }
}

/// Converts a list of rows, stopping at the first bad one.
pub(crate) fn convert_all<R, T>(rows: Vec<R>) -> StoreResult<Vec<T>>
where
    T: TryFrom<R, Error = StoreError>,
{
    rows.into_iter().map(T::try_from).collect()
}

pub(crate) fn kind_str(kind: SpaceKind) -> &'static str {
    match kind {
        SpaceKind::Room => "room",
        SpaceKind::Suite => "suite",
    }
}
