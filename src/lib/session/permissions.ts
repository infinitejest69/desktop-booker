import type { Permission } from "../bindings/Permission";
import type { Role } from "../bindings/Role";

// Mirrors Role::allows in booker-core, only to decide what the UI shows.
// The Rust commands check again; this never grants anything on its own.
const STAFF: Permission[] = ["view_hotel", "make_bookings", "cancel_bookings"];

export function allows(role: Role | undefined, permission: Permission): boolean {
  if (role === "admin") return true;
  if (role === "staff") return STAFF.includes(permission);
  return false;
}
