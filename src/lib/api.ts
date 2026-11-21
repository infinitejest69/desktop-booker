import { invoke } from "@tauri-apps/api/core";
import type { Floor } from "./bindings/Floor";
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

  seedDemoData: () => invoke<boolean>("seed_demo_data"),
};
