import { describe, expect, it } from "vitest";
import { addDays, datesFrom, daysBetween, formatDay, isWeekend } from "./dates";

describe("dates", () => {
  it("adds days across month, year and DST boundaries", () => {
    expect(addDays("2026-01-31", 1)).toBe("2026-02-01");
    expect(addDays("2026-12-31", 1)).toBe("2027-01-01");
    expect(addDays("2026-03-29", 1)).toBe("2026-03-30");
    expect(addDays("2026-10-25", -1)).toBe("2026-10-24");
  });

  it("counts nights between dates", () => {
    expect(daysBetween("2026-05-10", "2026-05-13")).toBe(3);
    expect(daysBetween("2026-03-28", "2026-03-30")).toBe(2);
  });

  it("lists consecutive dates", () => {
    expect(datesFrom("2026-02-27", 3)).toEqual(["2026-02-27", "2026-02-28", "2026-03-01"]);
  });

  it("formats and spots weekends", () => {
    expect(formatDay("2026-11-28", { weekday: "short", day: "numeric", month: "short" })).toBe(
      "Sat 28 Nov",
    );
    expect(isWeekend("2026-11-28")).toBe(true);
    expect(isWeekend("2026-11-30")).toBe(false);
  });
});
