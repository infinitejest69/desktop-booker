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
    Charge, ChargeId, ChartEntry, DateRange, DayOccupancy, Event, EventDetail, EventId,
    EventListItem, Floor, FloorId, Folio, FolioOwner, Guest, GuestId, GuestProfile,
    HousekeepingStatus, Invoice, InvoiceId, InvoiceSettings, MonthRevenue, Payment,
    PropertySettings, RateRule, RateRuleId, RoomHousekeeping, RoomType, RoomTypeId, Space, SpaceId,
    StayQuote, SuiteBooking, SuiteBookingEntry, SuiteBookingId, SuiteBookingStatus, SuiteRates,
    TaxRate,
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
        cars: u8,
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

pub trait InvoiceRepo {
    fn list_tax_rates(&self) -> impl Future<Output = StoreResult<Vec<TaxRate>>> + Send;
    fn save_tax_rate(&self, rate: &TaxRate) -> impl Future<Output = StoreResult<()>> + Send;
    /// Saved settings, or the defaults (prices include tax, "INV-") if none.
    fn get_invoice_settings(&self) -> impl Future<Output = StoreResult<InvoiceSettings>> + Send;
    fn save_invoice_settings(
        &self,
        settings: &InvoiceSettings,
    ) -> impl Future<Output = StoreResult<()>> + Send;
    /// The account: worked-out stay or suite charges, extras, payments,
    /// totals and the invoices issued so far.
    fn get_folio(&self, owner: FolioOwner) -> impl Future<Output = StoreResult<Folio>> + Send;
    fn add_charge(
        &self,
        owner: FolioOwner,
        charge: &Charge,
    ) -> impl Future<Output = StoreResult<Folio>> + Send;
    /// Removes an extra added by hand.
    fn remove_charge(&self, id: ChargeId) -> impl Future<Output = StoreResult<Folio>> + Send;
    /// Records money received, or refunded when the amount is negative.
    fn add_payment(
        &self,
        owner: FolioOwner,
        payment: &Payment,
    ) -> impl Future<Output = StoreResult<Folio>> + Send;
    /// Freezes the folio as an invoice under the next number.
    fn issue_invoice(
        &self,
        owner: FolioOwner,
        issued_on: NaiveDate,
    ) -> impl Future<Output = StoreResult<Invoice>> + Send;
    fn get_invoice(&self, id: InvoiceId) -> impl Future<Output = StoreResult<Invoice>> + Send;
}

pub trait OperationsRepo {
    /// Every bedroom's housekeeping state (ready if never set).
    /// Every bedroom's housekeeping state today: rooms slept in last night
    /// are dirty unless marked since the day began.
    fn list_housekeeping(
        &self,
        today: NaiveDate,
    ) -> impl Future<Output = StoreResult<Vec<RoomHousekeeping>>> + Send;
    /// Marks every in-service bedroom on a floor; returns how many.
    fn set_floor_housekeeping(
        &self,
        floor: FloorId,
        status: HousekeepingStatus,
    ) -> impl Future<Output = StoreResult<u32>> + Send;
    fn set_housekeeping(
        &self,
        space: SpaceId,
        status: HousekeepingStatus,
        note: &str,
    ) -> impl Future<Output = StoreResult<()>> + Send;
    /// Rooms sold each night of the period, against rooms in service.
    fn occupancy_report(
        &self,
        period: DateRange,
    ) -> impl Future<Output = StoreResult<Vec<DayOccupancy>>> + Send;
    /// Revenue per month in the period: rooms, board, suites and extras.
    fn revenue_report(
        &self,
        period: DateRange,
    ) -> impl Future<Output = StoreResult<Vec<MonthRevenue>>> + Send;
    fn guest_profile(&self, id: GuestId) -> impl Future<Output = StoreResult<GuestProfile>> + Send;
    /// Combines a duplicate guest record into `keep`: bookings, suite hire
    /// and events move across, and the duplicate is removed.
    fn merge_guests(
        &self,
        keep: GuestId,
        duplicate: GuestId,
    ) -> impl Future<Output = StoreResult<()>> + Send;
    /// Removes a guest's personal details (GDPR) but keeps their bookings.
    fn erase_guest(&self, id: GuestId) -> impl Future<Output = StoreResult<()>> + Send;
}

pub trait BackupRepo {
    /// Saved settings, or the defaults (app folder, 14 daily, 12 monthly).
    fn get_backup_settings(
        &self,
    ) -> impl Future<Output = StoreResult<booker_core::BackupSettings>> + Send;
    fn save_backup_settings(
        &self,
        settings: &booker_core::BackupSettings,
    ) -> impl Future<Output = StoreResult<()>> + Send;
    /// Copies the database into `dir` (safe while running), then deletes
    /// backups the rotation no longer keeps.
    fn backup_now(
        &self,
        dir: &std::path::Path,
        settings: &booker_core::BackupSettings,
    ) -> impl Future<Output = StoreResult<booker_core::BackupInfo>> + Send;
}

pub trait EmailRepo {
    /// Saved settings, or empty ones (port 587) if none.
    fn get_email_settings(
        &self,
    ) -> impl Future<Output = StoreResult<booker_core::EmailSettings>> + Send;
    fn save_email_settings(
        &self,
        settings: &booker_core::EmailSettings,
    ) -> impl Future<Output = StoreResult<()>> + Send;
    /// All three templates: the hotel's own wording, or the defaults.
    fn list_email_templates(
        &self,
    ) -> impl Future<Output = StoreResult<Vec<booker_core::EmailTemplate>>> + Send;
    fn save_email_template(
        &self,
        template: &booker_core::EmailTemplate,
    ) -> impl Future<Output = StoreResult<()>> + Send;
    fn log_email(
        &self,
        entry: &booker_core::EmailLogEntry,
    ) -> impl Future<Output = StoreResult<()>> + Send;
    /// A booking's emails, or the latest 200 of all, newest first.
    fn list_email_log(
        &self,
        booking: Option<BookingId>,
    ) -> impl Future<Output = StoreResult<Vec<booker_core::EmailLogEntry>>> + Send;
}
