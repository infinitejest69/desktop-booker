//! Pure domain logic for DesktopBooker.
//!
//! This crate must stay free of I/O (no database, files, or network) so it
//! can be unit-tested quickly and reused by every front end (desktop, LAN
//! server, Android).

pub mod audit;
pub mod backups;
pub mod booking;
pub mod dates;
pub mod demo;
pub mod email;
pub mod events;
pub mod folio;
pub mod grid;
pub mod ids;
pub mod invoices;
pub mod money;
pub mod operations;
pub mod property;
pub mod rates;
pub mod report_builder;
pub mod suites;
pub mod users;
pub mod validation;

// Re-export the main types so callers can write `booker_core::Floor`
// instead of `booker_core::property::Floor`.
pub use audit::AuditEntry;
pub use backups::{
    BackupInfo, BackupSettings, BackupStatus, backup_file_name, backup_overdue, backups_to_delete,
    parse_backup_name,
};
pub use booking::{
    Booking, BookingDetail, BookingListItem, BookingRoom, BookingSource, BookingStatus, ChartEntry,
    Guest, MAX_CARS, Occupancy, RoomPricing, format_reference, free_rooms, validate_booking,
    validate_guest,
};
pub use dates::{DateRange, MAX_NIGHTS, iso_weekday};
pub use demo::{DEMO_SEED, DemoAccount, DemoEvent, DemoHotel, DemoStay, demo_hotel};
pub use email::{
    EmailKind, EmailLogEntry, EmailSettings, EmailTemplate, PLACEHOLDERS, booking_values, fill,
    looks_like_email, validate_email_settings,
};
pub use events::{
    BlockPlan, CateringSitting, CateringTotals, Event, EventDetail, EventKind, EventListItem,
    EventRoom, LinkedRoom, RoomBlock, block_due_for_release, block_room_price, catering_totals,
    pick_block_rooms, plan_room_block, validate_event,
};
pub use folio::{
    Folio, FolioLine, FolioOwner, FolioParts, Invoice, InvoiceSettings, InvoiceSummary,
    stay_charges, suite_charges,
};
pub use grid::{GridRect, MAX_GRID_CELLS};
pub use ids::{
    AuditId, BoardOptionId, BookingId, BookingRoomId, ChargeId, DashboardId, EventId, FloorId,
    GuestId, InvoiceId, PaymentId, RateRuleId, RoomTypeId, SavedReportId, SpaceId, SuiteBookingId,
    TaxRateId, UserId,
};
pub use invoices::{
    BASIS_POINTS, Charge, ChargeCategory, FolioTotals, LineAmounts, Payment, PaymentMethod,
    TaxRate, TaxSummary, folio_totals, format_invoice_number, line_amounts, validate_charge,
    validate_payment, validate_tax_rate,
};
pub use money::Money;
pub use operations::{
    DayOccupancy, GuestProfile, HousekeepingStatus, MonthRevenue, RevenueSource, RoomHousekeeping,
    RoomKpis, anonymised, csv_money, housekeeping_after, housekeeping_today, merged_guest,
    occupancy_by_day, occupancy_total, revenue_by_month, room_kpis, to_csv,
};
pub use property::{
    Floor, PropertySettings, RoomFeature, RoomType, Space, SpaceKind, SuiteCapacities,
    features_from_text, features_to_text,
};
pub use rates::{
    Adjustment, BoardOption, NightPrice, RateRule, StayQuote, quote_stay, room_rate_for_night,
    validate_board_option, validate_rate_rule,
};
pub use report_builder::{
    ALL_MEASURES, ALL_SPLITS, ChargeFact, ChartKind, DASHBOARD_MAX_ITEMS, Dashboard, DashboardItem,
    EventFact, Measure, MeasureInfo, PaymentFact, Period, ReportFacts, ReportNames, ReportResult,
    ReportRow, ReportSpec, RoomFact, RoomNightFact, SavedReport, Split, StayFact, SuiteFact, Unit,
    allows, catalogue as report_catalogue, format_value, report_csv as custom_report_csv,
    run as run_report, validate_dashboard, validate_spec as validate_report_spec,
};
pub use suites::{
    Layout, SuiteBooking, SuiteBookingEntry, SuiteBookingStatus, SuitePricing, SuiteRates,
    price_suite_booking, validate_suite_booking, validate_suite_rates,
};
pub use users::{
    PIN_MAX, PIN_MIN, Permission, Role, SessionStatus, User, validate_pin, validate_user,
};
pub use validation::{
    ValidationError, ValidationResult, validate_floor, validate_floor_delete, validate_room_type,
    validate_room_type_delete, validate_settings, validate_space,
};

/// Crate version, handy for an "About" screen.
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_is_set() {
        assert!(!version().is_empty());
    }
}
