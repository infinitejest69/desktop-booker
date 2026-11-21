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

use booker_core::{
    BoardOption, BoardOptionId, Booking, BookingDetail, BookingId, BookingListItem, BookingStatus,
    ChartEntry, DateRange, Event, EventDetail, EventId, EventListItem, Floor, FloorId, Guest,
    PropertySettings, RateRule, RateRuleId, RoomType, RoomTypeId, Space, SpaceId, StayQuote,
    SuiteBooking, SuiteBookingEntry, SuiteBookingId, SuiteBookingStatus, SuiteRates,
};
use chrono::{NaiveDate, NaiveDateTime};

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

pub trait RateRepo {
    fn list_rate_rules(&self) -> impl Future<Output = StoreResult<Vec<RateRule>>> + Send;
    fn save_rate_rule(&self, rule: &RateRule) -> impl Future<Output = StoreResult<()>> + Send;
    fn delete_rate_rule(&self, id: RateRuleId) -> impl Future<Output = StoreResult<()>> + Send;
    fn list_board_options(&self) -> impl Future<Output = StoreResult<Vec<BoardOption>>> + Send;
    fn save_board_option(
        &self,
        board: &BoardOption,
    ) -> impl Future<Output = StoreResult<()>> + Send;
    fn delete_board_option(
        &self,
        id: BoardOptionId,
    ) -> impl Future<Output = StoreResult<()>> + Send;
}

pub trait GuestRepo {
    /// Guests whose name, email or phone contains `query` (case-insensitive).
    fn search_guests(&self, query: &str) -> impl Future<Output = StoreResult<Vec<Guest>>> + Send;
}

pub trait BookingRepo {
    /// Prices a room for a stay at today's rates (for the booking form).
    fn quote_room(
        &self,
        space_id: SpaceId,
        stay: DateRange,
        adults: u8,
        children: u8,
        board: Option<BoardOptionId>,
    ) -> impl Future<Output = StoreResult<StayQuote>> + Send;
    /// Bedrooms free for the whole stay, ignoring one booking (the one being edited).
    fn free_rooms(
        &self,
        stay: DateRange,
        ignore: Option<BookingId>,
    ) -> impl Future<Output = StoreResult<Vec<SpaceId>>> + Send;
    /// Booked rooms overlapping `[from, to)`, excluding cancelled and no-shows.
    fn chart(
        &self,
        from: NaiveDate,
        to: NaiveDate,
    ) -> impl Future<Output = StoreResult<Vec<ChartEntry>>> + Send;
    fn get_booking(&self, id: BookingId)
    -> impl Future<Output = StoreResult<BookingDetail>> + Send;
    /// Creates or updates a booking and its guest, after every rule passes.
    fn save_booking(
        &self,
        booking: &Booking,
        guest: &Guest,
    ) -> impl Future<Output = StoreResult<BookingDetail>> + Send;
    fn set_booking_status(
        &self,
        id: BookingId,
        status: BookingStatus,
    ) -> impl Future<Output = StoreResult<BookingDetail>> + Send;
    /// By reference, guest name or email; with an empty query, the next arrivals.
    fn search_bookings(
        &self,
        query: &str,
    ) -> impl Future<Output = StoreResult<Vec<BookingListItem>>> + Send;
}

pub trait SuiteRepo {
    /// A suite's rate card and default setup / clear-down times.
    fn get_suite_rates(
        &self,
        suite: SpaceId,
    ) -> impl Future<Output = StoreResult<SuiteRates>> + Send;
    fn save_suite_rates(
        &self,
        suite: SpaceId,
        rates: &SuiteRates,
    ) -> impl Future<Output = StoreResult<()>> + Send;
    /// Live (not cancelled) suite bookings overlapping `[from, to)`, by start time.
    fn list_suite_bookings(
        &self,
        from: NaiveDateTime,
        to: NaiveDateTime,
    ) -> impl Future<Output = StoreResult<Vec<SuiteBookingEntry>>> + Send;
    fn get_suite_booking(
        &self,
        id: SuiteBookingId,
    ) -> impl Future<Output = StoreResult<SuiteBookingEntry>> + Send;
    /// Creates or updates a suite booking and its organiser, after every rule
    /// passes. The price is fixed now and kept until the times, numbers or
    /// pricing change.
    fn save_suite_booking(
        &self,
        booking: &SuiteBooking,
        guest: &Guest,
    ) -> impl Future<Output = StoreResult<SuiteBookingEntry>> + Send;
    fn set_suite_booking_status(
        &self,
        id: SuiteBookingId,
        status: SuiteBookingStatus,
    ) -> impl Future<Output = StoreResult<SuiteBookingEntry>> + Send;
}

pub trait EventRepo {
    /// Events ending on or after `from`, soonest first.
    fn list_events(
        &self,
        from: NaiveDate,
    ) -> impl Future<Output = StoreResult<Vec<EventListItem>>> + Send;
    fn get_event(&self, id: EventId) -> impl Future<Output = StoreResult<EventDetail>> + Send;
    /// Saves the event, its sittings and organiser, and brings its room block
    /// in line: holds new rooms, reprices or releases held ones.
    fn save_event(
        &self,
        event: &Event,
        organiser: &Guest,
        today: NaiveDate,
    ) -> impl Future<Output = StoreResult<EventDetail>> + Send;
    /// Releases held block rooms whose release date has come. Returns how many.
    fn release_due_blocks(&self, today: NaiveDate)
    -> impl Future<Output = StoreResult<u32>> + Send;
}
