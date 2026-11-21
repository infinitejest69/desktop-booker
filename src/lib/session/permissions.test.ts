import { describe, expect, it } from "vitest";
import { allows } from "./permissions";

describe("allows", () => {
  it("lets staff run the desk but not setup, prices or people", () => {
    expect(allows("staff", "make_bookings")).toBe(true);
    expect(allows("staff", "cancel_bookings")).toBe(true);
    for (const p of ["manage_setup", "override_prices", "manage_users", "view_audit"] as const) {
      expect(allows("staff", p)).toBe(false);
      expect(allows("admin", p)).toBe(true);
    }
  });

  it("allows nothing when signed out", () => {
    expect(allows(undefined, "view_hotel")).toBe(false);
  });
});
