import { describe, expect, it } from "vitest";
import { axisLabel, compact, formatValue, niceMax, periodLabel } from "./format";

describe("report values", () => {
  it("shows each unit in words", () => {
    expect(formatValue("money", 123_456)).toBe("£1,234.56");
    expect(formatValue("percent", 675)).toBe("67.5%");
    expect(formatValue("nights", 25)).toBe("2.5 nights");
    expect(formatValue("nights", 10)).toBe("1.0 night");
    expect(formatValue("days", 75)).toBe("7.5 days");
    expect(formatValue("hours", 55)).toBe("5.5 hours");
    expect(formatValue("count", 1284)).toBe("1,284");
  });

  it("shortens axis numbers", () => {
    expect(compact("count", 950)).toBe("950");
    expect(compact("count", 12_900)).toBe("12.9K");
    expect(compact("money", 420_000_00)).toBe("£420K");
    expect(compact("percent", 500)).toBe("50%");
  });

  it("rounds an axis top up to a clean number", () => {
    expect(niceMax(0)).toBe(1);
    expect(niceMax(7)).toBe(10);
    expect(niceMax(42)).toBe(50);
    expect(niceMax(180)).toBe(200);
    expect(niceMax(2_100)).toBe(2_500);
  });

  it("names periods", () => {
    expect(periodLabel({ kind: "last_month" })).toBe("Last month");
    expect(periodLabel({ kind: "fixed", start: "2026-06-01", end: "2026-07-01" })).toBe("1 Jun 2026 to 30 Jun 2026");
  });

  it("keeps labels under columns short", () => {
    expect(axisLabel("month", "2026-06", "June 2026")).toBe("Jun");
    expect(axisLabel("week", "2026-06-15", "Week of 15 Jun 2026")).toBe("15 Jun");
    expect(axisLabel("day", "2026-06-01", "Mon 1 Jun 2026")).toBe("1 Jun");
    expect(axisLabel("year", "2026", "2026")).toBe("2026");
  });
});
