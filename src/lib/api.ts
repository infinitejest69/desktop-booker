import { invoke } from "@tauri-apps/api/core";
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

export type ErrorKind = "validation" | "not_found" | "internal";

export interface CommandError {
  kind: ErrorKind;
  message: string;
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

export function newId(): string {
  return crypto.randomUUID();
}

export const api = {
  appVersion: () => invoke<string>("app_version"),

  getSettings: () => invoke<PropertySettings>("get_settings"),
  saveSettings: (settings: PropertySettings) => invoke<void>("save_settings", { settings }),

  listFloors: () => invoke<Floor[]>("list_floors"),
  saveFloor: (floor: Floor) => invoke<void>("save_floor", { floor }),
  deleteFloor: (id: FloorId) => invoke<void>("delete_floor", { id }),

  listRoomTypes: () => invoke<RoomType[]>("list_room_types"),
  saveRoomType: (roomType: RoomType) => invoke<void>("save_room_type", { roomType }),
  deleteRoomType: (id: RoomTypeId) => invoke<void>("delete_room_type", { id }),

  listSpaces: (floorId?: FloorId) => invoke<Space[]>("list_spaces", { floorId: floorId ?? null }),
  saveSpace: (space: Space) => invoke<void>("save_space", { space }),
  deleteSpace: (id: SpaceId) => invoke<void>("delete_space", { id }),

  listRateRules: () => invoke<RateRule[]>("list_rate_rules"),
  saveRateRule: (rule: RateRule) => invoke<void>("save_rate_rule", { rule }),
  deleteRateRule: (id: RateRuleId) => invoke<void>("delete_rate_rule", { id }),
  listBoardOptions: () => invoke<BoardOption[]>("list_board_options"),
  saveBoardOption: (board: BoardOption) => invoke<void>("save_board_option", { board }),
  deleteBoardOption: (id: BoardOptionId) => invoke<void>("delete_board_option", { id }),

  searchGuests: (query: string) => invoke<Guest[]>("search_guests", { query }),
  quoteRoom: (
    spaceId: SpaceId,
    stay: DateRange,
    adults: number,
    children: number,
    boardOptionId: BoardOptionId | null,
  ) => invoke<StayQuote>("quote_room", { spaceId, stay, adults, children, boardOptionId }),
  freeRooms: (stay: DateRange, ignoreBooking: BookingId | null = null) =>
    invoke<SpaceId[]>("free_rooms", { stay, ignoreBooking }),
  chart: (from: string, to: string) => invoke<ChartEntry[]>("chart", { from, to }),
  getBooking: (id: BookingId) => invoke<BookingDetail>("get_booking", { id }),
  saveBooking: (booking: Booking, guest: Guest) =>
    invoke<BookingDetail>("save_booking", { booking, guest }),
  setBookingStatus: (id: BookingId, status: BookingStatus) =>
    invoke<BookingDetail>("set_booking_status", { id, status }),
  searchBookings: (query: string) => invoke<BookingListItem[]>("search_bookings", { query }),

  seedDemoData: () => invoke<boolean>("seed_demo_data"),
};
