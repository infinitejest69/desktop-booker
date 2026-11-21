import type { ChartEntry } from "../bindings/ChartEntry";
import type { Space } from "../bindings/Space";

export interface TodaySummary {
  /** One entry per booked room arriving today and not yet checked in. */
  arrivals: ChartEntry[];
  /** Booked rooms due out today and not yet checked out. */
  departures: ChartEntry[];
  /** Rooms occupied tonight by guests already checked in. */
  inHouse: ChartEntry[];
  checkedOutToday: number;
  freeTonight: number;
  occupiedTonight: number;
  /** Occupied ÷ in-service rooms, 0–100. */
  occupancy: number;
}

const byRoom = (rooms: Map<string, string>) => (a: ChartEntry, b: ChartEntry) =>
  (rooms.get(a.space_id) ?? "").localeCompare(rooms.get(b.space_id) ?? "", undefined, { numeric: true });

export function todaySummary(entries: ChartEntry[], rooms: Space[], date: string): TodaySummary {
  const bedrooms = rooms.filter((r) => r.kind === "room");
  const names = new Map(bedrooms.map((r) => [r.id, r.name]));
  const sort = byRoom(names);

  const arrivals = entries
    .filter((e) => e.stay.start === date && (e.status === "confirmed" || e.status === "provisional"))
    .sort(sort);
  const departures = entries
    .filter((e) => e.stay.end === date && e.status !== "checked_out")
    .sort(sort);
  const tonight = entries.filter((e) => e.stay.start <= date && date < e.stay.end);
  const inHouse = tonight.filter((e) => e.status === "checked_in").sort(sort);
  const occupiedRooms = new Set(tonight.map((e) => e.space_id));
  const inService = bedrooms.filter((r) => !r.out_of_service);
  const occupiedTonight = inService.filter((r) => occupiedRooms.has(r.id)).length;

  return {
    arrivals,
    departures,
    inHouse,
    checkedOutToday: entries.filter((e) => e.stay.end === date && e.status === "checked_out").length,
    freeTonight: inService.length - occupiedTonight,
    occupiedTonight,
    occupancy: inService.length ? Math.round((occupiedTonight / inService.length) * 100) : 0,
  };
}
