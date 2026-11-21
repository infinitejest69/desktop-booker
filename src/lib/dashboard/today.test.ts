import { describe, expect, it } from "vitest";
import type { BookingStatus } from "../bindings/BookingStatus";
import type { ChartEntry } from "../bindings/ChartEntry";
import type { Space } from "../bindings/Space";
import { todaySummary } from "./today";

const room = (id: string, out = false): Space => ({
  id,
  floor_id: "f",
  kind: "room",
  name: id,
  room_type_id: "t",
  suite_capacities: { theatre: null, boardroom: null, banquet: null, cabaret: null, reception: null },
  rect: { x: 0, y: 0, w: 1, h: 1 },
  out_of_service: out,
  notes: "",
});

const e = (space_id: string, start: string, end: string, status: BookingStatus): ChartEntry => ({
  booking_id: `${space_id}-${start}`,
  booking_room_id: `${space_id}-${start}-r`,
  reference: "2026-0001",
  guest_name: "Jo",
  status,
  space_id,
  stay: { start, end },
  adults: 2,
  children: 0,
});

describe("todaySummary", () => {
  const rooms = ["101", "102", "103", "104", "105"].map((id) => room(id));
  rooms.push(room("106", true));
  const day = "2026-05-10";
  const entries = [
    e("101", "2026-05-10", "2026-05-12", "confirmed"), // arriving
    e("102", "2026-05-08", "2026-05-11", "checked_in"), // in house
    e("103", "2026-05-07", "2026-05-10", "checked_in"), // due out
    e("104", "2026-05-08", "2026-05-10", "checked_out"), // already gone
    e("104", "2026-05-10", "2026-05-11", "checked_in"), // early check-in
  ];
  const s = todaySummary(entries, rooms, day);

  it("lists today's arrivals, departures and in-house guests", () => {
    expect(s.arrivals.map((x) => x.space_id)).toEqual(["101"]);
    expect(s.departures.map((x) => x.space_id)).toEqual(["103"]);
    expect(s.inHouse.map((x) => x.space_id)).toEqual(["102", "104"]);
    expect(s.checkedOutToday).toBe(1);
  });

  it("counts free rooms and occupancy, ignoring rooms out of service", () => {
    expect(s.occupiedTonight).toBe(3);
    expect(s.freeTonight).toBe(2);
    expect(s.occupancy).toBe(60);
  });
});
