import { describe, expect, it } from "vitest";
import type { ChartEntry } from "../bindings/ChartEntry";
import { barAt, layoutBars } from "./layout";

function entry(space: string, start: string, end: string): ChartEntry {
  return {
    booking_id: `${space}-${start}`,
    booking_room_id: "r",
    reference: "2026-0001",
    guest_name: "Jo Hughes",
    status: "confirmed",
    space_id: space,
    stay: { start, end },
    adults: 2,
    children: 0,
  };
}

describe("layoutBars", () => {
  it("places a stay by its first night and length", () => {
    const bars = layoutBars([entry("101", "2026-05-12", "2026-05-15")], "2026-05-10", 14);
    expect(bars.get("101")).toMatchObject([
      { col: 2, span: 3, clippedStart: false, clippedEnd: false },
    ]);
  });

  it("clips stays that run off either edge", () => {
    const bars = layoutBars(
      [entry("101", "2026-05-08", "2026-05-11"), entry("102", "2026-05-20", "2026-05-30")],
      "2026-05-10",
      14,
    );
    expect(bars.get("101")).toMatchObject([{ col: 0, span: 1, clippedStart: true }]);
    expect(bars.get("102")).toMatchObject([{ col: 10, span: 4, clippedEnd: true }]);
  });

  it("skips stays outside the window and finds the bar under a column", () => {
    const bars = layoutBars(
      [entry("101", "2026-05-01", "2026-05-10"), entry("101", "2026-05-12", "2026-05-14")],
      "2026-05-10",
      7,
    );
    expect(bars.get("101")).toHaveLength(1);
    expect(barAt(bars.get("101"), 3)?.entry.stay.start).toBe("2026-05-12");
    expect(barAt(bars.get("101"), 4)).toBeUndefined();
    expect(barAt(bars.get("101"), 1)).toBeUndefined();
  });
});
