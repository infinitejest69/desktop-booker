import type { ChartEntry } from "../bindings/ChartEntry";
import type { Space } from "../bindings/Space";

export type RoomStatus = "free" | "arriving" | "in_house" | "departing" | "turnover" | "out_of_service";

export const roomStatusLabel: Record<RoomStatus, string> = {
  free: "Free",
  arriving: "Arriving",
  in_house: "In house",
  departing: "Departing",
  turnover: "Departing + arriving",
  out_of_service: "Out of service",
};

/** Each room's status on `date`, from chart entries covering date − 1 to date + 1. */
export function roomStatuses(spaces: Space[], entries: ChartEntry[], date: string): Map<string, RoomStatus> {
  const result = new Map<string, RoomStatus>();
  for (const space of spaces) {
    if (space.out_of_service) {
      result.set(space.id, "out_of_service");
      continue;
    }
    const mine = entries.filter((e) => e.space_id === space.id);
    const staying = mine.find((e) => e.stay.start <= date && date < e.stay.end);
    const leaving = mine.some((e) => e.stay.end === date);
    let status: RoomStatus = "free";
    if (staying && leaving) status = "turnover";
    else if (staying) status = staying.stay.start === date ? "arriving" : "in_house";
    else if (leaving) status = "departing";
    result.set(space.id, status);
  }
  return result;
}
