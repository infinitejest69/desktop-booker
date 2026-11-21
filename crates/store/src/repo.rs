//! Repository traits: the only way the rest of the app reads or writes data.
//!
//! Today they're implemented by [`crate::SqliteStore`]. A future LAN
//! client/server mode can add another implementation (e.g. one that talks to
//! a host PC over HTTP) without changing callers. See ADR 0002.
//!
//! Each method returns `impl Future<Output = …> + Send` rather than being
//! written `async fn`. It means the same thing, but also promises the future
//! can move between threads, which Tauri's async commands require.
//! Implementations can still use plain `async fn`.

use std::future::Future;

use booker_core::{Floor, FloorId, PropertySettings, RoomType, RoomTypeId, Space, SpaceId};

use crate::StoreResult;

pub trait SettingsRepo {
    /// Returns defaults if settings were never saved.
    fn get_settings(&self) -> impl Future<Output = StoreResult<PropertySettings>> + Send;
    fn save_settings(
        &self,
        settings: &PropertySettings,
    ) -> impl Future<Output = StoreResult<()>> + Send;
}

pub trait FloorRepo {
    /// Active floors ordered by level, then name.
    fn list_floors(&self) -> impl Future<Output = StoreResult<Vec<Floor>>> + Send;
    /// Insert or update (by id), after validation.
    fn save_floor(&self, floor: &Floor) -> impl Future<Output = StoreResult<()>> + Send;
    /// Soft-delete; fails if rooms/suites are still on the floor.
    fn delete_floor(&self, id: FloorId) -> impl Future<Output = StoreResult<()>> + Send;
}

pub trait RoomTypeRepo {
    fn list_room_types(&self) -> impl Future<Output = StoreResult<Vec<RoomType>>> + Send;
    fn save_room_type(&self, room_type: &RoomType) -> impl Future<Output = StoreResult<()>> + Send;
    /// Soft-delete; fails if any room still uses this type.
    fn delete_room_type(&self, id: RoomTypeId) -> impl Future<Output = StoreResult<()>> + Send;
}

pub trait SpaceRepo {
    /// All active spaces, or only those on one floor.
    fn list_spaces(
        &self,
        floor: Option<FloorId>,
    ) -> impl Future<Output = StoreResult<Vec<Space>>> + Send;
    fn save_space(&self, space: &Space) -> impl Future<Output = StoreResult<()>> + Send;
    fn delete_space(&self, id: SpaceId) -> impl Future<Output = StoreResult<()>> + Send;
}
