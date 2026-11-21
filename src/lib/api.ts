import { invoke } from "@tauri-apps/api/core";
import type { AuditEntry } from "./bindings/AuditEntry";
import type { SessionStatus } from "./bindings/SessionStatus";
import type { User } from "./bindings/User";
import type { UserId } from "./bindings/UserId";
import type { BoardOption } from "./bindings/BoardOption";
import type { BoardOptionId } from "./bindings/BoardOptionId";
import type { Booking } from "./bindings/Booking";
import type { BookingDetail } from "./bindings/BookingDetail";
import type { BookingId } from "./bindings/BookingId";
import type { BookingListItem } from "./bindings/BookingListItem";
import type { BookingStatus } from "./bindings/BookingStatus";
import type { ChartEntry } from "./bindings/ChartEntry";
import type { DateRange } from "./bindings/DateRange";
import type { Floor } from "./bindings/Floor";
import type { Guest } from "./bindings/Guest";
import type { RateRule } from "./bindings/RateRule";
import type { RateRuleId } from "./bindings/RateRuleId";
import type { StayQuote } from "./bindings/StayQuote";
import type { FloorId } from "./bindings/FloorId";
import type { PropertySettings } from "./bindings/PropertySettings";
import type { RoomType } from "./bindings/RoomType";
import type { RoomTypeId } from "./bindings/RoomTypeId";
import type { Space } from "./bindings/Space";
import type { SpaceId } from "./bindings/SpaceId";
import type { Event } from "./bindings/Event";
import type { EventDetail } from "./bindings/EventDetail";
import type { EventId } from "./bindings/EventId";
import type { EventListItem } from "./bindings/EventListItem";
import type { SuiteBooking } from "./bindings/SuiteBooking";
import type { SuiteBookingEntry } from "./bindings/SuiteBookingEntry";
import type { SuiteBookingId } from "./bindings/SuiteBookingId";
import type { SuiteBookingStatus } from "./bindings/SuiteBookingStatus";
import type { SuiteRates } from "./bindings/SuiteRates";
import type { Charge } from "./bindings/Charge";
import type { ChargeId } from "./bindings/ChargeId";
import type { Folio } from "./bindings/Folio";
import type { FolioOwner } from "./bindings/FolioOwner";
import type { Invoice } from "./bindings/Invoice";
import type { InvoiceId } from "./bindings/InvoiceId";
import type { InvoiceSettings } from "./bindings/InvoiceSettings";
import type { Payment } from "./bindings/Payment";
import type { TaxRate } from "./bindings/TaxRate";
import type { DayOccupancy } from "./bindings/DayOccupancy";
import type { GuestProfile } from "./bindings/GuestProfile";
import type { HousekeepingStatus } from "./bindings/HousekeepingStatus";
import type { MonthRevenue } from "./bindings/MonthRevenue";
import type { RoomHousekeeping } from "./bindings/RoomHousekeeping";
import type { RoomKpis } from "./bindings/RoomKpis";
import type { BackupSettings } from "./bindings/BackupSettings";
import type { BackupStatus } from "./bindings/BackupStatus";

export type ErrorKind = "validation" | "not_found" | "internal" | "locked" | "forbidden";

export interface CommandError {
  kind: ErrorKind;
  message: string;
  /** Seconds before trying again makes sense, e.g. after too many wrong PINs. */
  retry_after?: number;
}

export function isCommandError(value: unknown): value is CommandError {
  return (
    typeof value === "object" &&
    value !== null &&
    "kind" in value &&
    "message" in value &&
    typeof (value as CommandError).message === "string"
  );
}

export function errorMessage(error: unknown): string {
  if (isCommandError(error)) return error.message;
  if (error instanceof Error) return error.message;
  return String(error);
}

/** Fired on `window` when the backend says nobody is signed in. */
export const LOCKED_EVENT = "booker:locked";

async function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(command, args);
  } catch (error) {
    if (isCommandError(error) && error.kind === "locked") {
      window.dispatchEvent(new CustomEvent(LOCKED_EVENT));
    }
    throw error;
  }
}

export function newId(): string {
  return crypto.randomUUID();
}

export const api = {
  appVersion: () => call<string>("app_version"),

  sessionStatus: () => call<SessionStatus>("session_status"),
  createFirstAdmin: (name: string, pin: string) => call<User>("create_first_admin", { name, pin }),
  login: (userId: UserId, pin: string) => call<User>("login", { userId, pin }),
  logout: () => call<void>("logout"),
  listUsers: () => call<User[]>("list_users"),
  saveUser: (user: User, pin: string | null) => call<void>("save_user", { user, pin }),
  listAudit: (userId: UserId | null, entity: string | null, limit = 200) =>
    call<AuditEntry[]>("list_audit", { userId, entity, limit }),

  getSettings: () => call<PropertySettings>("get_settings"),
  saveSettings: (settings: PropertySettings) => call<void>("save_settings", { settings }),

  listFloors: () => call<Floor[]>("list_floors"),
  saveFloor: (floor: Floor) => call<void>("save_floor", { floor }),
  deleteFloor: (id: FloorId) => call<void>("delete_floor", { id }),

  listRoomTypes: () => call<RoomType[]>("list_room_types"),
  saveRoomType: (roomType: RoomType) => call<void>("save_room_type", { roomType }),
  deleteRoomType: (id: RoomTypeId) => call<void>("delete_room_type", { id }),

  listSpaces: (floorId?: FloorId) => call<Space[]>("list_spaces", { floorId: floorId ?? null }),
  saveSpace: (space: Space) => call<void>("save_space", { space }),
  deleteSpace: (id: SpaceId) => call<void>("delete_space", { id }),

  listRateRules: () => call<RateRule[]>("list_rate_rules"),
  saveRateRule: (rule: RateRule) => call<void>("save_rate_rule", { rule }),
  deleteRateRule: (id: RateRuleId) => call<void>("delete_rate_rule", { id }),
  listBoardOptions: () => call<BoardOption[]>("list_board_options"),
  saveBoardOption: (board: BoardOption) => call<void>("save_board_option", { board }),
  deleteBoardOption: (id: BoardOptionId) => call<void>("delete_board_option", { id }),

  searchGuests: (query: string) => call<Guest[]>("search_guests", { query }),
  quoteRoom: (
    spaceId: SpaceId,
    stay: DateRange,
    adults: number,
    children: number,
    boardOptionId: BoardOptionId | null,
  ) => call<StayQuote>("quote_room", { spaceId, stay, adults, children, boardOptionId }),
  freeRooms: (stay: DateRange, ignoreBooking: BookingId | null = null) =>
    call<SpaceId[]>("free_rooms", { stay, ignoreBooking }),
  chart: (from: string, to: string) => call<ChartEntry[]>("chart", { from, to }),
  getBooking: (id: BookingId) => call<BookingDetail>("get_booking", { id }),
  saveBooking: (booking: Booking, guest: Guest) =>
    call<BookingDetail>("save_booking", { booking, guest }),
  setBookingStatus: (id: BookingId, status: BookingStatus) =>
    call<BookingDetail>("set_booking_status", { id, status }),
  searchBookings: (query: string) => call<BookingListItem[]>("search_bookings", { query }),

  seedDemoData: () => call<boolean>("seed_demo_data"),

  getSuiteRates: (suite: SpaceId) => call<SuiteRates>("get_suite_rates", { suite }),
  saveSuiteRates: (suite: SpaceId, rates: SuiteRates) => call<void>("save_suite_rates", { suite, rates }),
  /** Date-times are "YYYY-MM-DDTHH:MM:SS" in hotel local time. */
  listSuiteBookings: (from: string, to: string) =>
    call<SuiteBookingEntry[]>("list_suite_bookings", { from, to }),
  getSuiteBooking: (id: SuiteBookingId) => call<SuiteBookingEntry>("get_suite_booking", { id }),
  saveSuiteBooking: (booking: SuiteBooking, guest: Guest) =>
    call<SuiteBookingEntry>("save_suite_booking", { booking, guest }),
  setSuiteBookingStatus: (id: SuiteBookingId, status: SuiteBookingStatus) =>
    call<SuiteBookingEntry>("set_suite_booking_status", { id, status }),

  listEvents: (from: string) => call<EventListItem[]>("list_events", { from }),
  getEvent: (id: EventId) => call<EventDetail>("get_event", { id }),
  saveEvent: (event: Event, organiser: Guest) => call<EventDetail>("save_event", { event, organiser }),

  listTaxRates: () => call<TaxRate[]>("list_tax_rates"),
  saveTaxRate: (rate: TaxRate) => call<void>("save_tax_rate", { rate }),
  getInvoiceSettings: () => call<InvoiceSettings>("get_invoice_settings"),
  saveInvoiceSettings: (settings: InvoiceSettings) => call<void>("save_invoice_settings", { settings }),
  getFolio: (owner: FolioOwner) => call<Folio>("get_folio", { owner }),
  addCharge: (owner: FolioOwner, charge: Charge) => call<Folio>("add_charge", { owner, charge }),
  removeCharge: (id: ChargeId) => call<Folio>("remove_charge", { id }),
  addPayment: (owner: FolioOwner, payment: Payment) => call<Folio>("add_payment", { owner, payment }),
  issueInvoice: (owner: FolioOwner) => call<Invoice>("issue_invoice", { owner }),
  getInvoice: (id: InvoiceId) => call<Invoice>("get_invoice", { id }),

  listHousekeeping: () => call<RoomHousekeeping[]>("list_housekeeping"),
  setHousekeeping: (space: SpaceId, status: HousekeepingStatus, note = "") =>
    call<void>("set_housekeeping", { space, status, note }),
  setFloorHousekeeping: (floor: FloorId, status: HousekeepingStatus) =>
    call<number>("set_floor_housekeeping", { floor, status }),
  occupancyReport: (period: DateRange) => call<DayOccupancy[]>("occupancy_report", { period }),
  revenueReport: (period: DateRange) => call<MonthRevenue[]>("revenue_report", { period }),
  roomKpis: (period: DateRange) => call<RoomKpis>("room_kpis", { period }),
  reportCsv: (report: "occupancy" | "revenue", period: DateRange) =>
    call<string>("report_csv", { report, period }),
  guestProfile: (id: string) => call<GuestProfile>("guest_profile", { id }),
  eraseGuest: (id: string) => call<void>("erase_guest", { id }),
  mergeGuests: (keep: string, duplicate: string) => call<void>("merge_guests", { keep, duplicate }),

  backupStatus: () => call<BackupStatus>("backup_status"),
  saveBackupSettings: (settings: BackupSettings) => call<BackupStatus>("save_backup_settings", { settings }),
  backupNow: () => call<BackupStatus>("backup_now"),
  /** Replaces all data with the backup and restarts the app. */
  restoreBackup: (name: string) => call<void>("restore_backup", { name }),
};
