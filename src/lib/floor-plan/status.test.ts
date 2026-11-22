import { describe, expect, it } from "vitest";
import type { ChartEntry } from "../bindings/ChartEntry";
import type { Space } from "../bindings/Space";
import { roomStatuses } from "./status";

const space = (id: string, out = false): Space => ({
  id,
  floor_id: "f",
  kind: "room",
  name: id,
  room_type_id: "t",
  suite_capacities: { theatre: null, boardroom: null, banquet: null, cabaret: null, reception: null },
  rect: { x: 0, y: 0, w: 1, h: 1 },
  out_of_service: out,
  features: [],
  notes: "",
});

const stay = (space_id: string, start: string, end: string): ChartEntry => ({
  booking_id: `${space_id}${start}`,
  booking_room_id: "r",
  reference: "2026-0001",
  guest_name: "Jo",
  status: "confirmed",
  space_id,
  stay: { start, end },
  adults: 2,
  children: 0,
  board: null,
});

describe("roomStatuses", () => {
  it("works out each room's day", () => {
    const rooms = ["a", "b", "c", "d", "e"].map((id) => space(id));
    rooms.push(space("f", true));
    const entries = [
      stay("a", "2026-05-10", "2026-05-12"), // arriving on the 10th
      stay("b", "2026-05-08", "2026-05-12"), // in house
      stay("c", "2026-05-07", "2026-05-10"), // departing
      stay("d", "2026-05-07", "2026-05-10"), // departing…
      stay("d", "2026-05-10", "2026-05-11"), // …and arriving
    ];
    const s = roomStatuses(rooms, entries, "2026-05-10");
    expect(Object.fromEntries(s)).toEqual({
      a: "arriving",
      b: "in_house",
      c: "departing",
      d: "turnover",
      e: "free",
      f: "out_of_service",
    });
  });
});
