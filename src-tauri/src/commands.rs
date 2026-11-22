//! Tauri commands: the functions the frontend can call with `invoke`.
//!
//! Every command first asks the [`Session`] for permission. `session.require`
//! returns the store "acting as" the signed-in user, so each change lands in
//! the audit log under their name. The checks live here in Rust, not only in
//! the UI, so hiding a button is never the only protection.
//!
//! Tauri renames arguments to camelCase for JavaScript, so a Rust parameter
//! `floor_id` is passed from the frontend as `{ floorId }`.

use booker_core::{
    AuditEntry, BoardOption, BoardOptionId, Booking, BookingDetail, BookingId, BookingListItem,
    BookingStatus, Charge, ChargeId, ChartEntry, DateRange, Event, EventDetail, EventId,
    EventListItem, Floor, FloorId, Folio, FolioOwner, Guest, Invoice, InvoiceId, InvoiceSettings,
    Payment, Permission, PropertySettings, RateRule, RateRuleId, Role, RoomType, RoomTypeId,
    SessionStatus, Space, SpaceId, StayQuote, SuiteBooking, SuiteBookingEntry, SuiteBookingId,
    SuiteBookingStatus, SuitePricing, SuiteRates, TaxRate, User, UserId, validate_pin,
};
use booker_core::{
    DayOccupancy, GuestId, GuestProfile, HousekeepingStatus, MonthRevenue, RoomHousekeeping,
    csv_money, occupancy_total, to_csv,
};
use booker_services::pin::{hash_pin, verify_pin};
use booker_store::{
    BookingRepo, EventRepo, FloorRepo, GuestRepo, InvoiceRepo, OperationsRepo, RateRepo,
    RoomTypeRepo, SettingsRepo, SpaceRepo, SqliteStore, StoreError, SuiteRepo,
};
use chrono::{NaiveDate, NaiveDateTime};
use tauri::State;

use crate::error::{CmdResult, CommandError};
use crate::session::Session;

type Store<'a> = State<'a, SqliteStore>;
type Sess<'a> = State<'a, Session>;

#[tauri::command]
pub fn app_version() -> String {
    booker_core::version().to_string()
}

// ---- session & users ----

#[tauri::command]
pub async fn session_status(store: Store<'_>, session: Sess<'_>) -> CmdResult<SessionStatus> {
    let users = store.list_users().await?;
    let settings = store.get_settings().await?;
    Ok(SessionStatus {
        needs_setup: users.is_empty(),
        user: session.current_user(),
        users: users.into_iter().filter(|u| u.active).collect(),
        hotel_name: settings.hotel_name,
        idle_lock_minutes: settings.idle_lock_minutes,
    })
}

/// Creates the very first admin. Refused once any user exists.
#[tauri::command]
pub async fn create_first_admin(
    store: Store<'_>,
    session: Sess<'_>,
    name: String,
    pin: String,
) -> CmdResult<User> {
    if !store.list_users().await?.is_empty() {
        return Err(CommandError::forbidden(
            "The first admin has already been set up.",
        ));
    }
    validate_pin(&pin).map_err(StoreError::from)?;
    let user = User {
        id: UserId::new(),
        name: name.trim().to_string(),
        role: Role::Admin,
        active: true,
    };
    let hash = hash_pin(&pin).map_err(CommandError::internal)?;
    store.as_user(&user).save_user(&user, Some(&hash)).await?;
    session.sign_in(user.clone());
    Ok(user)
}

#[tauri::command]
pub async fn login(
    store: Store<'_>,
    session: Sess<'_>,
    user_id: UserId,
    pin: String,
) -> CmdResult<User> {
    session.check_not_locked_out(user_id)?;
    let hash = store
        .pin_hash(user_id)
        .await?
        .ok_or_else(|| CommandError::validation("That user can't sign in."))?;
    if !verify_pin(&pin, &hash) {
        return Err(session.failed_attempt(user_id));
    }
    let user = store
        .list_users()
        .await?
        .into_iter()
        .find(|u| u.id == user_id && u.active)
        .ok_or_else(|| CommandError::validation("That user can't sign in."))?;
    session.sign_in(user.clone());
    Ok(user)
}

#[tauri::command]
pub fn logout(session: Sess<'_>) {
    session.sign_out();
}

#[tauri::command]
pub async fn list_users(store: Store<'_>, session: Sess<'_>) -> CmdResult<Vec<User>> {
    Ok(session
        .require(&store, Permission::ManageUsers)?
        .list_users()
        .await?)
}

/// Adds or updates a user. `pin` is required for a new user and optional
/// (keep the current one) when editing.
#[tauri::command]
pub async fn save_user(
    store: Store<'_>,
    session: Sess<'_>,
    user: User,
    pin: Option<String>,
) -> CmdResult<()> {
    let acting = session.require(&store, Permission::ManageUsers)?;
    let hash = match pin.as_deref().filter(|p| !p.is_empty()) {
        Some(p) => {
            validate_pin(p).map_err(StoreError::from)?;
            Some(hash_pin(p).map_err(CommandError::internal)?)
        }
        None => None,
    };
    acting.save_user(&user, hash.as_deref()).await?;
    // Keep the session in step if admins edit themselves.
    if session.current_user().is_some_and(|u| u.id == user.id) {
        if user.active {
            session.sign_in(user)
        } else {
            session.sign_out()
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn list_audit(
    store: Store<'_>,
    session: Sess<'_>,
    user_id: Option<UserId>,
    entity: Option<String>,
    limit: Option<u32>,
) -> CmdResult<Vec<AuditEntry>> {
    let acting = session.require(&store, Permission::ViewAudit)?;
    Ok(acting
        .list_audit(user_id, entity.as_deref(), limit.unwrap_or(200))
        .await?)
}

// ---- settings ----

#[tauri::command]
pub async fn get_settings(store: Store<'_>, session: Sess<'_>) -> CmdResult<PropertySettings> {
    Ok(session
        .require(&store, Permission::ViewHotel)?
        .get_settings()
        .await?)
}

#[tauri::command]
pub async fn save_settings(
    store: Store<'_>,
    session: Sess<'_>,
    settings: PropertySettings,
) -> CmdResult<()> {
    session
        .require(&store, Permission::ManageSetup)?
        .save_settings(&settings)
        .await?;
    session.set_idle_lock(settings.idle_lock_minutes);
    Ok(())
}

// ---- floors ----

#[tauri::command]
pub async fn list_floors(store: Store<'_>, session: Sess<'_>) -> CmdResult<Vec<Floor>> {
    Ok(session
        .require(&store, Permission::ViewHotel)?
        .list_floors()
        .await?)
}

#[tauri::command]
pub async fn save_floor(store: Store<'_>, session: Sess<'_>, floor: Floor) -> CmdResult<()> {
    Ok(session
        .require(&store, Permission::ManageSetup)?
        .save_floor(&floor)
        .await?)
}

#[tauri::command]
pub async fn delete_floor(store: Store<'_>, session: Sess<'_>, id: FloorId) -> CmdResult<()> {
    Ok(session
        .require(&store, Permission::ManageSetup)?
        .delete_floor(id)
        .await?)
}

// ---- room types ----

#[tauri::command]
pub async fn list_room_types(store: Store<'_>, session: Sess<'_>) -> CmdResult<Vec<RoomType>> {
    Ok(session
        .require(&store, Permission::ViewHotel)?
        .list_room_types()
        .await?)
}

#[tauri::command]
pub async fn save_room_type(
    store: Store<'_>,
    session: Sess<'_>,
    room_type: RoomType,
) -> CmdResult<()> {
    Ok(session
        .require(&store, Permission::ManageSetup)?
        .save_room_type(&room_type)
        .await?)
}

#[tauri::command]
pub async fn delete_room_type(
    store: Store<'_>,
    session: Sess<'_>,
    id: RoomTypeId,
) -> CmdResult<()> {
    Ok(session
        .require(&store, Permission::ManageSetup)?
        .delete_room_type(id)
        .await?)
}

// ---- spaces (rooms & suites) ----

#[tauri::command]
pub async fn list_spaces(
    store: Store<'_>,
    session: Sess<'_>,
    floor_id: Option<FloorId>,
) -> CmdResult<Vec<Space>> {
    Ok(session
        .require(&store, Permission::ViewHotel)?
        .list_spaces(floor_id)
        .await?)
}

#[tauri::command]
pub async fn save_space(store: Store<'_>, session: Sess<'_>, space: Space) -> CmdResult<()> {
    Ok(session
        .require(&store, Permission::ManageSetup)?
        .save_space(&space)
        .await?)
}

#[tauri::command]
pub async fn delete_space(store: Store<'_>, session: Sess<'_>, id: SpaceId) -> CmdResult<()> {
    Ok(session
        .require(&store, Permission::ManageSetup)?
        .delete_space(id)
        .await?)
}

// ---- rates & board ----

#[tauri::command]
pub async fn list_rate_rules(store: Store<'_>, session: Sess<'_>) -> CmdResult<Vec<RateRule>> {
    Ok(session
        .require(&store, Permission::ViewHotel)?
        .list_rate_rules()
        .await?)
}

#[tauri::command]
pub async fn save_rate_rule(store: Store<'_>, session: Sess<'_>, rule: RateRule) -> CmdResult<()> {
    Ok(session
        .require(&store, Permission::ManageSetup)?
        .save_rate_rule(&rule)
        .await?)
}

#[tauri::command]
pub async fn delete_rate_rule(
    store: Store<'_>,
    session: Sess<'_>,
    id: RateRuleId,
) -> CmdResult<()> {
    Ok(session
        .require(&store, Permission::ManageSetup)?
        .delete_rate_rule(id)
        .await?)
}

#[tauri::command]
pub async fn list_board_options(
    store: Store<'_>,
    session: Sess<'_>,
) -> CmdResult<Vec<BoardOption>> {
    Ok(session
        .require(&store, Permission::ViewHotel)?
        .list_board_options()
        .await?)
}

#[tauri::command]
pub async fn save_board_option(
    store: Store<'_>,
    session: Sess<'_>,
    board: BoardOption,
) -> CmdResult<()> {
    Ok(session
        .require(&store, Permission::ManageSetup)?
        .save_board_option(&board)
        .await?)
}

#[tauri::command]
pub async fn delete_board_option(
    store: Store<'_>,
    session: Sess<'_>,
    id: BoardOptionId,
) -> CmdResult<()> {
    Ok(session
        .require(&store, Permission::ManageSetup)?
        .delete_board_option(id)
        .await?)
}

// ---- guests & bookings ----

#[tauri::command]
pub async fn search_guests(
    store: Store<'_>,
    session: Sess<'_>,
    query: String,
) -> CmdResult<Vec<Guest>> {
    Ok(session
        .require(&store, Permission::ViewHotel)?
        .search_guests(&query)
        .await?)
}

// Tauri commands take each argument by name from the frontend, so a long list is expected.
#[allow(clippy::too_many_arguments)]
#[tauri::command]
pub async fn quote_room(
    store: Store<'_>,
    session: Sess<'_>,
    space_id: SpaceId,
    stay: DateRange,
    adults: u8,
    children: u8,
    board_option_id: Option<BoardOptionId>,
    cars: u8,
) -> CmdResult<StayQuote> {
    let acting = session.require(&store, Permission::ViewHotel)?;
    Ok(acting
        .quote_room(space_id, stay, adults, children, board_option_id, cars)
        .await?)
}

#[tauri::command]
pub async fn free_rooms(
    store: Store<'_>,
    session: Sess<'_>,
    stay: DateRange,
    ignore_booking: Option<BookingId>,
) -> CmdResult<Vec<SpaceId>> {
    Ok(session
        .require(&store, Permission::ViewHotel)?
        .free_rooms(stay, ignore_booking)
        .await?)
}

#[tauri::command]
pub async fn chart(
    store: Store<'_>,
    session: Sess<'_>,
    from: NaiveDate,
    to: NaiveDate,
) -> CmdResult<Vec<ChartEntry>> {
    Ok(session
        .require(&store, Permission::ViewHotel)?
        .chart(from, to)
        .await?)
}

#[tauri::command]
pub async fn get_booking(
    store: Store<'_>,
    session: Sess<'_>,
    id: BookingId,
) -> CmdResult<BookingDetail> {
    Ok(session
        .require(&store, Permission::ViewHotel)?
        .get_booking(id)
        .await?)
}

#[tauri::command]
pub async fn save_booking(
    store: Store<'_>,
    session: Sess<'_>,
    booking: Booking,
    guest: Guest,
) -> CmdResult<BookingDetail> {
    let acting = session.require(&store, Permission::MakeBookings)?;
    // Staff may keep a price an admin set, but not set or change one.
    if session.require(&store, Permission::OverridePrices).is_err() {
        let saved = match acting.get_booking(booking.id).await {
            Ok(d) => Some(d.booking),
            Err(StoreError::NotFound { .. }) => None,
            Err(e) => return Err(e.into()),
        };
        let changed = booking.rooms.iter().any(|room| {
            let before = saved
                .as_ref()
                .and_then(|b| b.rooms.iter().find(|r| r.id == room.id))
                .and_then(|r| r.price_override);
            room.price_override != before
        });
        if changed {
            return Err(CommandError::forbidden(
                "Only an admin can set a price by hand.",
            ));
        }
    }
    Ok(acting.save_booking(&booking, &guest).await?)
}

#[tauri::command]
pub async fn set_booking_status(
    store: Store<'_>,
    session: Sess<'_>,
    id: BookingId,
    status: BookingStatus,
) -> CmdResult<BookingDetail> {
    let needed = match status {
        BookingStatus::Cancelled | BookingStatus::NoShow => Permission::CancelBookings,
        _ => Permission::MakeBookings,
    };
    Ok(session
        .require(&store, needed)?
        .set_booking_status(id, status)
        .await?)
}

#[tauri::command]
pub async fn search_bookings(
    store: Store<'_>,
    session: Sess<'_>,
    query: String,
) -> CmdResult<Vec<BookingListItem>> {
    Ok(session
        .require(&store, Permission::ViewHotel)?
        .search_bookings(&query)
        .await?)
}

// ---- dev helpers ----

/// Loads the demo hotel if the database is empty. Returns whether it did.
/// Recorded in the audit log as "System", not as the person who clicked.
#[tauri::command]
pub async fn seed_demo_data(store: Store<'_>, session: Sess<'_>) -> CmdResult<bool> {
    session.require(&store, Permission::ManageSetup)?;
    // A new demo hotel comes with events; an existing one gets them added.
    let hotel = store.seed_demo().await?;
    let events = store.seed_demo_events().await?;
    let invoicing = store.seed_demo_invoicing().await?;
    Ok(hotel || events || invoicing)
}

// ---------------------------------------------------------------------------
// Function suites
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn get_suite_rates(
    store: Store<'_>,
    session: Sess<'_>,
    suite: SpaceId,
) -> CmdResult<SuiteRates> {
    Ok(session
        .require(&store, Permission::ViewHotel)?
        .get_suite_rates(suite)
        .await?)
}

#[tauri::command]
pub async fn save_suite_rates(
    store: Store<'_>,
    session: Sess<'_>,
    suite: SpaceId,
    rates: SuiteRates,
) -> CmdResult<()> {
    Ok(session
        .require(&store, Permission::ManageSetup)?
        .save_suite_rates(suite, &rates)
        .await?)
}

#[tauri::command]
pub async fn list_suite_bookings(
    store: Store<'_>,
    session: Sess<'_>,
    from: NaiveDateTime,
    to: NaiveDateTime,
) -> CmdResult<Vec<SuiteBookingEntry>> {
    Ok(session
        .require(&store, Permission::ViewHotel)?
        .list_suite_bookings(from, to)
        .await?)
}

#[tauri::command]
pub async fn get_suite_booking(
    store: Store<'_>,
    session: Sess<'_>,
    id: SuiteBookingId,
) -> CmdResult<SuiteBookingEntry> {
    Ok(session
        .require(&store, Permission::ViewHotel)?
        .get_suite_booking(id)
        .await?)
}

#[tauri::command]
pub async fn save_suite_booking(
    store: Store<'_>,
    session: Sess<'_>,
    booking: SuiteBooking,
    guest: Guest,
) -> CmdResult<SuiteBookingEntry> {
    let acting = session.require(&store, Permission::MakeBookings)?;
    // Staff may keep a hand-set price an admin chose, but not set or change one.
    if matches!(booking.pricing, SuitePricing::Manual { .. })
        && session.require(&store, Permission::OverridePrices).is_err()
    {
        let before = match acting.get_suite_booking(booking.id).await {
            Ok(e) => Some(e.booking.pricing),
            Err(StoreError::NotFound { .. }) => None,
            Err(e) => return Err(e.into()),
        };
        if before != Some(booking.pricing) {
            return Err(CommandError::forbidden(
                "Only an admin can set a price by hand.",
            ));
        }
    }
    Ok(acting.save_suite_booking(&booking, &guest).await?)
}

#[tauri::command]
pub async fn set_suite_booking_status(
    store: Store<'_>,
    session: Sess<'_>,
    id: SuiteBookingId,
    status: SuiteBookingStatus,
) -> CmdResult<SuiteBookingEntry> {
    let needed = match status {
        SuiteBookingStatus::Cancelled => Permission::CancelBookings,
        _ => Permission::MakeBookings,
    };
    Ok(session
        .require(&store, needed)?
        .set_suite_booking_status(id, status)
        .await?)
}

// ---------------------------------------------------------------------------
// Events
// ---------------------------------------------------------------------------

fn today() -> NaiveDate {
    chrono::Local::now().date_naive()
}

#[tauri::command]
pub async fn list_events(
    store: Store<'_>,
    session: Sess<'_>,
    from: NaiveDate,
) -> CmdResult<Vec<EventListItem>> {
    Ok(session
        .require(&store, Permission::ViewHotel)?
        .list_events(from)
        .await?)
}

#[tauri::command]
pub async fn get_event(store: Store<'_>, session: Sess<'_>, id: EventId) -> CmdResult<EventDetail> {
    Ok(session
        .require(&store, Permission::ViewHotel)?
        .get_event(id)
        .await?)
}

#[tauri::command]
pub async fn save_event(
    store: Store<'_>,
    session: Sess<'_>,
    event: Event,
    organiser: Guest,
) -> CmdResult<EventDetail> {
    let acting = session.require(&store, Permission::MakeBookings)?;
    // The room block's group rate is a hand-set room price, so it's admin only.
    if session.require(&store, Permission::OverridePrices).is_err() {
        let before = match acting.get_event(event.id).await {
            Ok(d) => d.event.room_block.map(|b| b.rate),
            Err(StoreError::NotFound { .. }) => None,
            Err(e) => return Err(e.into()),
        };
        let rate = event.room_block.map(|b| b.rate);
        if rate.is_some() && rate != before {
            return Err(CommandError::forbidden(
                "Only an admin can set a room block's group rate.",
            ));
        }
    }
    Ok(acting.save_event(&event, &organiser, today()).await?)
}

// ---------------------------------------------------------------------------
// Folios and invoices
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn list_tax_rates(store: Store<'_>, session: Sess<'_>) -> CmdResult<Vec<TaxRate>> {
    Ok(session
        .require(&store, Permission::ViewHotel)?
        .list_tax_rates()
        .await?)
}

#[tauri::command]
pub async fn save_tax_rate(store: Store<'_>, session: Sess<'_>, rate: TaxRate) -> CmdResult<()> {
    Ok(session
        .require(&store, Permission::ManageSetup)?
        .save_tax_rate(&rate)
        .await?)
}

#[tauri::command]
pub async fn get_invoice_settings(
    store: Store<'_>,
    session: Sess<'_>,
) -> CmdResult<InvoiceSettings> {
    Ok(session
        .require(&store, Permission::ViewHotel)?
        .get_invoice_settings()
        .await?)
}

#[tauri::command]
pub async fn save_invoice_settings(
    store: Store<'_>,
    session: Sess<'_>,
    settings: InvoiceSettings,
) -> CmdResult<()> {
    Ok(session
        .require(&store, Permission::ManageSetup)?
        .save_invoice_settings(&settings)
        .await?)
}

#[tauri::command]
pub async fn get_folio(store: Store<'_>, session: Sess<'_>, owner: FolioOwner) -> CmdResult<Folio> {
    Ok(session
        .require(&store, Permission::ViewHotel)?
        .get_folio(owner)
        .await?)
}

#[tauri::command]
pub async fn add_charge(
    store: Store<'_>,
    session: Sess<'_>,
    owner: FolioOwner,
    charge: Charge,
) -> CmdResult<Folio> {
    Ok(session
        .require(&store, Permission::MakeBookings)?
        .add_charge(owner, &charge)
        .await?)
}

#[tauri::command]
pub async fn remove_charge(store: Store<'_>, session: Sess<'_>, id: ChargeId) -> CmdResult<Folio> {
    Ok(session
        .require(&store, Permission::CancelBookings)?
        .remove_charge(id)
        .await?)
}

#[tauri::command]
pub async fn add_payment(
    store: Store<'_>,
    session: Sess<'_>,
    owner: FolioOwner,
    payment: Payment,
) -> CmdResult<Folio> {
    Ok(session
        .require(&store, Permission::MakeBookings)?
        .add_payment(owner, &payment)
        .await?)
}

#[tauri::command]
pub async fn issue_invoice(
    store: Store<'_>,
    session: Sess<'_>,
    owner: FolioOwner,
) -> CmdResult<Invoice> {
    Ok(session
        .require(&store, Permission::MakeBookings)?
        .issue_invoice(owner, today())
        .await?)
}

#[tauri::command]
pub async fn get_invoice(store: Store<'_>, session: Sess<'_>, id: InvoiceId) -> CmdResult<Invoice> {
    Ok(session
        .require(&store, Permission::ViewHotel)?
        .get_invoice(id)
        .await?)
}

// ---------------------------------------------------------------------------
// Housekeeping, reports and guests
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn list_housekeeping(
    store: Store<'_>,
    session: Sess<'_>,
) -> CmdResult<Vec<RoomHousekeeping>> {
    Ok(session
        .require(&store, Permission::ViewHotel)?
        .list_housekeeping(today())
        .await?)
}

#[tauri::command]
pub async fn set_housekeeping(
    store: Store<'_>,
    session: Sess<'_>,
    space: SpaceId,
    status: HousekeepingStatus,
    note: String,
) -> CmdResult<()> {
    Ok(session
        .require(&store, Permission::MakeBookings)?
        .set_housekeeping(space, status, &note)
        .await?)
}

/// Marks every bedroom on a floor, e.g. all clean after the morning round.
#[tauri::command]
pub async fn set_floor_housekeeping(
    store: Store<'_>,
    session: Sess<'_>,
    floor: FloorId,
    status: HousekeepingStatus,
) -> CmdResult<u32> {
    Ok(session
        .require(&store, Permission::MakeBookings)?
        .set_floor_housekeeping(floor, status)
        .await?)
}

/// Reports are for admins, like the rest of the hotel's figures.
#[tauri::command]
pub async fn occupancy_report(
    store: Store<'_>,
    session: Sess<'_>,
    period: DateRange,
) -> CmdResult<Vec<DayOccupancy>> {
    Ok(session
        .require(&store, Permission::ViewReports)?
        .occupancy_report(period)
        .await?)
}

/// Revenue is for admins: it shows the hotel's takings.
#[tauri::command]
pub async fn revenue_report(
    store: Store<'_>,
    session: Sess<'_>,
    period: DateRange,
) -> CmdResult<Vec<MonthRevenue>> {
    Ok(session
        .require(&store, Permission::ViewAudit)?
        .revenue_report(period)
        .await?)
}

/// ADR and RevPAR for a period: admin only, like revenue.
#[tauri::command]
pub async fn room_kpis(
    store: Store<'_>,
    session: Sess<'_>,
    period: DateRange,
) -> CmdResult<booker_core::RoomKpis> {
    let acting = session.require(&store, Permission::ViewAudit)?;
    let days = acting.occupancy_report(period).await?;
    let rooms: booker_core::Money = acting
        .revenue_report(period)
        .await?
        .iter()
        .map(|m| m.rooms)
        .sum();
    Ok(booker_core::room_kpis(&days, rooms))
}

/// A report as CSV text, for spreadsheets.
#[tauri::command]
pub async fn report_csv(
    store: Store<'_>,
    session: Sess<'_>,
    report: String,
    period: DateRange,
) -> CmdResult<String> {
    match report.as_str() {
        "occupancy" => {
            let days = session
                .require(&store, Permission::ViewReports)?
                .occupancy_report(period)
                .await?;
            let pct = |d: &DayOccupancy| format!("{}.{}", d.per_mille / 10, d.per_mille % 10);
            let mut rows: Vec<Vec<String>> = days
                .iter()
                .map(|d| {
                    vec![
                        d.date.to_string(),
                        d.occupied.to_string(),
                        d.rooms.to_string(),
                        pct(d),
                    ]
                })
                .collect();
            let total = occupancy_total(&days);
            rows.push(vec![
                "Total".into(),
                total.occupied.to_string(),
                total.rooms.to_string(),
                pct(&total),
            ]);
            Ok(to_csv(
                &["Night", "Rooms sold", "Rooms", "Occupancy %"],
                &rows,
            ))
        }
        "revenue" => {
            let months = session
                .require(&store, Permission::ViewAudit)?
                .revenue_report(period)
                .await?;
            let rows: Vec<Vec<String>> = months
                .iter()
                .map(|m| {
                    vec![
                        format!("{}-{:02}", m.year, m.month),
                        csv_money(m.rooms),
                        csv_money(m.board),
                        csv_money(m.suites),
                        csv_money(m.extras),
                        csv_money(m.total),
                    ]
                })
                .collect();
            Ok(to_csv(
                &["Month", "Rooms", "Board", "Suites", "Extras", "Total"],
                &rows,
            ))
        }
        other => Err(CommandError::validation(format!(
            "Unknown report {other:?}"
        ))),
    }
}

#[tauri::command]
pub async fn guest_profile(
    store: Store<'_>,
    session: Sess<'_>,
    id: GuestId,
) -> CmdResult<GuestProfile> {
    Ok(session
        .require(&store, Permission::ViewHotel)?
        .guest_profile(id)
        .await?)
}

/// Combines a duplicate guest into another: admin only, it can't be undone.
#[tauri::command]
pub async fn merge_guests(
    store: Store<'_>,
    session: Sess<'_>,
    keep: GuestId,
    duplicate: GuestId,
) -> CmdResult<()> {
    Ok(session
        .require(&store, Permission::ManageSetup)?
        .merge_guests(keep, duplicate)
        .await?)
}

/// GDPR erasure: admin only, and it can't be undone.
#[tauri::command]
pub async fn erase_guest(store: Store<'_>, session: Sess<'_>, id: GuestId) -> CmdResult<()> {
    Ok(session
        .require(&store, Permission::ManageSetup)?
        .erase_guest(id)
        .await?)
}

// ---------------------------------------------------------------------------
// Backups
// ---------------------------------------------------------------------------

async fn status_of(
    store: &SqliteStore,
    app: &tauri::AppHandle,
) -> CmdResult<booker_core::BackupStatus> {
    use booker_store::BackupRepo;
    let settings = store.get_backup_settings().await?;
    let dir = crate::backup_dir(app, &settings.folder);
    // The app's own folder is made on first use; a chosen one must exist.
    let folder_ok = dir.is_dir()
        || (settings.folder.trim().is_empty() && std::fs::create_dir_all(&dir).is_ok());
    let backups = booker_store::list_backups(&dir)?;
    let last = backups.first().map(|b| b.taken_at);
    Ok(booker_core::BackupStatus {
        folder: dir.display().to_string(),
        folder_ok,
        overdue: booker_core::backup_overdue(last, chrono::Local::now().naive_local()),
        last,
        backups,
        settings,
    })
}

#[tauri::command]
pub async fn backup_status(
    app: tauri::AppHandle,
    store: Store<'_>,
    session: Sess<'_>,
) -> CmdResult<booker_core::BackupStatus> {
    let acting = session.require(&store, Permission::ViewHotel)?;
    status_of(&acting, &app).await
}

#[tauri::command]
pub async fn save_backup_settings(
    app: tauri::AppHandle,
    store: Store<'_>,
    session: Sess<'_>,
    settings: booker_core::BackupSettings,
) -> CmdResult<booker_core::BackupStatus> {
    use booker_store::BackupRepo;
    let acting = session.require(&store, Permission::ManageSetup)?;
    let folder = settings.folder.trim();
    if !folder.is_empty() && !std::path::Path::new(folder).is_dir() {
        return Err(CommandError::validation(
            "That folder can't be found. Plug in the drive or choose another folder.",
        ));
    }
    acting.save_backup_settings(&settings).await?;
    status_of(&acting, &app).await
}

#[tauri::command]
pub async fn backup_now(
    app: tauri::AppHandle,
    store: Store<'_>,
    session: Sess<'_>,
) -> CmdResult<booker_core::BackupStatus> {
    use booker_store::BackupRepo;
    let acting = session.require(&store, Permission::ManageSetup)?;
    let settings = acting.get_backup_settings().await?;
    acting
        .backup_now(&crate::backup_dir(&app, &settings.folder), &settings)
        .await?;
    status_of(&acting, &app).await
}

/// Replaces the live database with a backup and restarts the app. A
/// snapshot of the current data is taken first, so a restore can be undone.
#[tauri::command]
pub async fn restore_backup(
    app: tauri::AppHandle,
    store: Store<'_>,
    session: Sess<'_>,
    db: State<'_, crate::DbPath>,
    name: String,
) -> CmdResult<()> {
    use booker_store::BackupRepo;
    let acting = session.require(&store, Permission::ManageSetup)?;
    // Only a plain file name from the list: no paths.
    if booker_core::parse_backup_name(&name).is_none() || name.contains(['/', '\\']) {
        return Err(CommandError::validation("Choose a backup from the list."));
    }
    let settings = acting.get_backup_settings().await?;
    let dir = crate::backup_dir(&app, &settings.folder);
    let chosen = dir.join(&name);
    if !chosen.is_file() {
        return Err(CommandError::validation("That backup can't be found."));
    }
    acting.backup_now(&dir, &settings).await?;
    store.close().await;
    booker_store::restore_file(&chosen, &db.0)?;
    app.restart();
}

// ---------------------------------------------------------------------------
// Email
// ---------------------------------------------------------------------------

use booker_core::{EmailKind, EmailLogEntry, EmailSettings, EmailTemplate};
use booker_services::email as mail;
use booker_store::EmailRepo;

/// The email settings and whether a password is saved in the keychain.
#[derive(serde::Serialize)]
pub struct EmailSetup {
    settings: EmailSettings,
    password_saved: bool,
    templates: Vec<EmailTemplate>,
}

#[tauri::command]
pub async fn get_email_setup(store: Store<'_>, session: Sess<'_>) -> CmdResult<EmailSetup> {
    let acting = session.require(&store, Permission::ViewHotel)?;
    let settings = acting.get_email_settings().await?;
    let username = settings.username.clone();
    let password_saved =
        tauri::async_runtime::spawn_blocking(move || mail::has_password(&username))
            .await
            .unwrap_or(false);
    Ok(EmailSetup {
        settings,
        password_saved,
        templates: acting.list_email_templates().await?,
    })
}

/// Saves the server details; a new password goes to the keychain, not the database.
#[tauri::command]
pub async fn save_email_settings(
    store: Store<'_>,
    session: Sess<'_>,
    settings: EmailSettings,
    password: Option<String>,
) -> CmdResult<()> {
    let acting = session.require(&store, Permission::ManageSetup)?;
    acting.save_email_settings(&settings).await?;
    if let Some(password) = password.filter(|p| !p.trim().is_empty()) {
        let username = settings.username.clone();
        // Gmail shows app passwords in groups of four; the spaces aren't part of it.
        let password: String = password.chars().filter(|c| !c.is_whitespace()).collect();
        tauri::async_runtime::spawn_blocking(move || mail::save_password(&username, &password))
            .await
            .map_err(|e| CommandError::internal(e.to_string()))?
            .map_err(CommandError::validation)?;
    }
    Ok(())
}

#[tauri::command]
pub async fn save_email_template(
    store: Store<'_>,
    session: Sess<'_>,
    template: EmailTemplate,
) -> CmdResult<()> {
    Ok(session
        .require(&store, Permission::ManageSetup)?
        .save_email_template(&template)
        .await?)
}

/// Sends, logs the attempt, and turns a failure into a message for staff.
async fn send_and_log(
    acting: &SqliteStore,
    sender: &str,
    to: &str,
    subject: &str,
    body: &str,
    kind: Option<EmailKind>,
    booking_id: Option<BookingId>,
) -> CmdResult<()> {
    let settings = acting.get_email_settings().await?;
    let result = mail::send(&settings, to, subject, body).await;
    acting
        .log_email(&EmailLogEntry {
            at: chrono::Local::now().naive_local(),
            to: to.to_string(),
            subject: subject.to_string(),
            kind,
            booking_id,
            sent: result.is_ok(),
            error: result.as_ref().err().cloned().unwrap_or_default(),
            sent_by: sender.to_string(),
        })
        .await?;
    result.map_err(CommandError::validation)
}

#[tauri::command]
pub async fn send_test_email(store: Store<'_>, session: Sess<'_>, to: String) -> CmdResult<()> {
    let acting = session.require(&store, Permission::ManageSetup)?;
    let hotel = acting.get_settings().await?;
    let sender = session.current_user().map(|u| u.name).unwrap_or_default();
    send_and_log(
        &acting,
        &sender,
        &to,
        &format!("Test email from {}", hotel.hotel_name),
        &format!(
            "This is a test from DesktopBooker. If you can read this, {} can send emails.",
            hotel.hotel_name
        ),
        None,
        None,
    )
    .await
}

/// "£1,234.50" for GBP, "EUR 1234.50" otherwise.
fn money_text(m: booker_core::Money, currency: &str) -> String {
    let pounds = booker_core::csv_money(m);
    if currency.eq_ignore_ascii_case("GBP") {
        format!("£{pounds}")
    } else {
        format!("{currency} {pounds}")
    }
}

/// Emails the guest the confirmation, amendment or cancellation for a booking.
#[tauri::command]
pub async fn send_booking_email(
    store: Store<'_>,
    session: Sess<'_>,
    booking: BookingId,
    kind: EmailKind,
) -> CmdResult<()> {
    let acting = session.require(&store, Permission::MakeBookings)?;
    let detail = acting.get_booking(booking).await?;
    let to = detail.guest.email.trim().to_string();
    if to.is_empty() {
        return Err(CommandError::validation(
            "This guest has no email address. Add one to the booking first.",
        ));
    }
    let hotel = acting.get_settings().await?;
    let spaces = acting.list_spaces(None).await?;
    let rooms: Vec<String> = detail
        .booking
        .rooms
        .iter()
        .map(|r| {
            spaces
                .iter()
                .find(|s| s.id == r.space_id)
                .map_or_else(|| "?".to_string(), |s| s.name.clone())
        })
        .collect();
    let template = acting
        .list_email_templates()
        .await?
        .into_iter()
        .find(|t| t.kind == kind)
        .unwrap_or_else(|| EmailTemplate::default_for(kind));
    let currency = hotel.currency.clone();
    let values =
        booker_core::booking_values(&detail, &rooms, &hotel.hotel_name, &hotel.phone, |m| {
            money_text(m, &currency)
        });
    let sender = session.current_user().map(|u| u.name).unwrap_or_default();
    send_and_log(
        &acting,
        &sender,
        &to,
        &booker_core::fill(&template.subject, &values),
        &booker_core::fill(&template.body, &values),
        Some(kind),
        Some(booking),
    )
    .await
}

#[tauri::command]
pub async fn list_email_log(
    store: Store<'_>,
    session: Sess<'_>,
    booking: Option<BookingId>,
) -> CmdResult<Vec<EmailLogEntry>> {
    Ok(session
        .require(&store, Permission::ViewHotel)?
        .list_email_log(booking)
        .await?)
}

// ---------------------------------------------------------------------------
// Saving files (the system save dialog, opened from Rust)
// ---------------------------------------------------------------------------

/// Asks where to save, then writes the text there. `None` if cancelled.
async fn save_with_dialog(
    app: &tauri::AppHandle,
    file_name: String,
    filter: (&'static str, &'static str),
    contents: String,
) -> CmdResult<Option<String>> {
    use tauri_plugin_dialog::DialogExt;
    let dialog = app.dialog().clone();
    // The dialog waits for the person, so it runs off the async threads.
    let chosen = tauri::async_runtime::spawn_blocking(move || {
        dialog
            .file()
            .set_file_name(file_name)
            .add_filter(filter.0, &[filter.1])
            .blocking_save_file()
    })
    .await
    .map_err(|e| CommandError::internal(e.to_string()))?;
    let Some(path) = chosen.and_then(|p| p.into_path().ok()) else {
        return Ok(None);
    };
    std::fs::write(&path, contents)
        .map_err(|e| CommandError::validation(format!("Couldn't save {}: {e}", path.display())))?;
    Ok(Some(path.display().to_string()))
}

#[tauri::command]
pub async fn save_report_csv(
    app: tauri::AppHandle,
    store: Store<'_>,
    session: Sess<'_>,
    report: String,
    period: DateRange,
) -> CmdResult<Option<String>> {
    let name = format!("{report}-{}.csv", period.start.format("%Y-%m"));
    let csv = report_csv(store, session, report, period).await?;
    save_with_dialog(&app, name, ("CSV", "csv"), csv).await
}

/// Everything held about a guest, as JSON, for a subject access request.
#[tauri::command]
pub async fn save_guest_data(
    app: tauri::AppHandle,
    store: Store<'_>,
    session: Sess<'_>,
    id: GuestId,
) -> CmdResult<Option<String>> {
    let profile = session
        .require(&store, Permission::ViewHotel)?
        .guest_profile(id)
        .await?;
    let json = serde_json::to_string_pretty(&profile)
        .map_err(|e| CommandError::internal(e.to_string()))?;
    let name = format!(
        "guest-{}.json",
        profile
            .guest
            .display_name()
            .to_lowercase()
            .replace(' ', "-")
    );
    save_with_dialog(&app, name, ("JSON", "json"), json).await
}
