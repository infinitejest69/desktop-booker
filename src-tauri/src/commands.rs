//! Tauri commands: the functions the frontend can call with `invoke`.
//!
//! Keep these thin: take input, call the store, return the result. Rules
//! live in `booker-core`, persistence in `booker-store`.
//!
//! Note: Tauri renames arguments to camelCase for JavaScript, so a Rust
//! parameter `floor_id` is passed from the frontend as `{ floorId }`.
//!
//! `State<'_, SqliteStore>` is the store registered with `app.manage(...)` at
//! startup. `'_` is an elided lifetime: the borrow lasts as long as the call.

use booker_core::{Floor, FloorId, PropertySettings, RoomType, RoomTypeId, Space, SpaceId};
use booker_store::{FloorRepo, RoomTypeRepo, SettingsRepo, SpaceRepo, SqliteStore};
use tauri::State;

use crate::error::CmdResult;

type Store<'a> = State<'a, SqliteStore>;

#[tauri::command]
pub fn app_version() -> String {
    booker_core::version().to_string()
}

// ---- settings ----

#[tauri::command]
pub async fn get_settings(store: Store<'_>) -> CmdResult<PropertySettings> {
    Ok(store.get_settings().await?)
}

#[tauri::command]
pub async fn save_settings(store: Store<'_>, settings: PropertySettings) -> CmdResult<()> {
    Ok(store.save_settings(&settings).await?)
}

// ---- floors ----

#[tauri::command]
pub async fn list_floors(store: Store<'_>) -> CmdResult<Vec<Floor>> {
    Ok(store.list_floors().await?)
}

#[tauri::command]
pub async fn save_floor(store: Store<'_>, floor: Floor) -> CmdResult<()> {
    Ok(store.save_floor(&floor).await?)
}

#[tauri::command]
pub async fn delete_floor(store: Store<'_>, id: FloorId) -> CmdResult<()> {
    Ok(store.delete_floor(id).await?)
}

// ---- room types ----

#[tauri::command]
pub async fn list_room_types(store: Store<'_>) -> CmdResult<Vec<RoomType>> {
    Ok(store.list_room_types().await?)
}

#[tauri::command]
pub async fn save_room_type(store: Store<'_>, room_type: RoomType) -> CmdResult<()> {
    Ok(store.save_room_type(&room_type).await?)
}

#[tauri::command]
pub async fn delete_room_type(store: Store<'_>, id: RoomTypeId) -> CmdResult<()> {
    Ok(store.delete_room_type(id).await?)
}

// ---- spaces (rooms & suites) ----

#[tauri::command]
pub async fn list_spaces(store: Store<'_>, floor_id: Option<FloorId>) -> CmdResult<Vec<Space>> {
    Ok(store.list_spaces(floor_id).await?)
}

#[tauri::command]
pub async fn save_space(store: Store<'_>, space: Space) -> CmdResult<()> {
    Ok(store.save_space(&space).await?)
}

#[tauri::command]
pub async fn delete_space(store: Store<'_>, id: SpaceId) -> CmdResult<()> {
    Ok(store.delete_space(id).await?)
}

// ---- dev helpers ----

/// Loads the demo hotel if the database is empty. Returns whether it did.
#[tauri::command]
pub async fn seed_demo_data(store: Store<'_>) -> CmdResult<bool> {
    Ok(store.seed_demo().await?)
}
