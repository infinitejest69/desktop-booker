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

use booker_core::{
    BoardOption, BoardOptionId, Booking, BookingDetail, BookingId, BookingListItem, BookingStatus,
    ChartEntry, DateRange, Floor, FloorId, Guest, PropertySettings, RateRule, RateRuleId, RoomType,
    RoomTypeId, Space, SpaceId, StayQuote,
};
use booker_store::{
    BookingRepo, FloorRepo, GuestRepo, RateRepo, RoomTypeRepo, SettingsRepo, SpaceRepo, SqliteStore,
};
use chrono::NaiveDate;
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

// ---- rates & board ----

#[tauri::command]
pub async fn list_rate_rules(store: Store<'_>) -> CmdResult<Vec<RateRule>> {
    Ok(store.list_rate_rules().await?)
}

#[tauri::command]
pub async fn save_rate_rule(store: Store<'_>, rule: RateRule) -> CmdResult<()> {
    Ok(store.save_rate_rule(&rule).await?)
}

#[tauri::command]
pub async fn delete_rate_rule(store: Store<'_>, id: RateRuleId) -> CmdResult<()> {
    Ok(store.delete_rate_rule(id).await?)
}

#[tauri::command]
pub async fn list_board_options(store: Store<'_>) -> CmdResult<Vec<BoardOption>> {
    Ok(store.list_board_options().await?)
}

#[tauri::command]
pub async fn save_board_option(store: Store<'_>, board: BoardOption) -> CmdResult<()> {
    Ok(store.save_board_option(&board).await?)
}

#[tauri::command]
pub async fn delete_board_option(store: Store<'_>, id: BoardOptionId) -> CmdResult<()> {
    Ok(store.delete_board_option(id).await?)
}

// ---- guests & bookings ----

#[tauri::command]
pub async fn search_guests(store: Store<'_>, query: String) -> CmdResult<Vec<Guest>> {
    Ok(store.search_guests(&query).await?)
}

#[tauri::command]
pub async fn quote_room(
    store: Store<'_>,
    space_id: SpaceId,
    stay: DateRange,
    adults: u8,
    children: u8,
    board_option_id: Option<BoardOptionId>,
) -> CmdResult<StayQuote> {
    Ok(store
        .quote_room(space_id, stay, adults, children, board_option_id)
        .await?)
}

#[tauri::command]
pub async fn free_rooms(
    store: Store<'_>,
    stay: DateRange,
    ignore_booking: Option<BookingId>,
) -> CmdResult<Vec<SpaceId>> {
    Ok(store.free_rooms(stay, ignore_booking).await?)
}

#[tauri::command]
pub async fn chart(store: Store<'_>, from: NaiveDate, to: NaiveDate) -> CmdResult<Vec<ChartEntry>> {
    Ok(store.chart(from, to).await?)
}

#[tauri::command]
pub async fn get_booking(store: Store<'_>, id: BookingId) -> CmdResult<BookingDetail> {
    Ok(store.get_booking(id).await?)
}

#[tauri::command]
pub async fn save_booking(
    store: Store<'_>,
    booking: Booking,
    guest: Guest,
) -> CmdResult<BookingDetail> {
    Ok(store.save_booking(&booking, &guest).await?)
}

#[tauri::command]
pub async fn set_booking_status(
    store: Store<'_>,
    id: BookingId,
    status: BookingStatus,
) -> CmdResult<BookingDetail> {
    Ok(store.set_booking_status(id, status).await?)
}

#[tauri::command]
pub async fn search_bookings(store: Store<'_>, query: String) -> CmdResult<Vec<BookingListItem>> {
    Ok(store.search_bookings(&query).await?)
}

// ---- dev helpers ----

/// Loads the demo hotel if the database is empty. Returns whether it did.
#[tauri::command]
pub async fn seed_demo_data(store: Store<'_>) -> CmdResult<bool> {
    Ok(store.seed_demo().await?)
}
