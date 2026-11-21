import { describe, expect, it } from "vitest";
import { estimateSuitePrice, joinDateTime, offeredLayouts, timelineBar } from "./suites";

const rates = { hourly: 5000, min_hours: 3, half_day: null, full_day: 40000, setup_minutes: 60, cleardown_minutes: 30 };
const booking = (start: string, end: string, pricing: Parameters<typeof estimateSuitePrice>[0]["pricing"]) => ({
  start: joinDateTime("2026-06-10", start),
  end: joinDateTime("2026-06-10", end),
  attendees: 40,
  pricing,
});

describe("estimateSuitePrice", () => {
  it("rounds hourly hire up and applies the minimum", () => {
    expect(estimateSuitePrice(booking("10:00", "12:00", { kind: "hourly" }), rates)).toBe(15000);
    expect(estimateSuitePrice(booking("10:00", "14:10", { kind: "hourly" }), rates)).toBe(25000);
  });

  it("uses day rates, per head and hand-set prices, or null when not offered", () => {
    expect(estimateSuitePrice(booking("09:00", "17:00", { kind: "full_day" }), rates)).toBe(40000);
    expect(estimateSuitePrice(booking("09:00", "12:00", { kind: "half_day" }), rates)).toBeNull();
    expect(estimateSuitePrice(booking("09:00", "12:00", { kind: "per_head", price: 3500 }), rates)).toBe(140000);
    expect(estimateSuitePrice(booking("09:00", "12:00", { kind: "manual", price: 999 }), rates)).toBe(999);
  });
});

describe("timelineBar", () => {
  it("places the booking and its buffers within the window", () => {
    const bar = timelineBar(
      { start: "2026-06-10T12:00:00", end: "2026-06-10T16:00:00", setup_minutes: 60, cleardown_minutes: 120 },
      "2026-06-10T08:00:00",
      "2026-06-11T00:00:00",
    );
    expect(Math.round(bar.left)).toBe(25);
    expect(Math.round(bar.width)).toBe(25);
    expect(Math.round(bar.padLeft)).toBe(19);
    expect(Math.round(bar.padWidth)).toBe(44);
  });

  it("clips bookings that run past the window", () => {
    const bar = timelineBar(
      { start: "2026-06-10T06:00:00", end: "2026-06-11T02:00:00", setup_minutes: 0, cleardown_minutes: 0 },
      "2026-06-10T08:00:00",
      "2026-06-11T00:00:00",
    );
    expect(bar.left).toBe(0);
    expect(bar.width).toBe(100);
  });
});

describe("offeredLayouts", () => {
  it("lists only layouts with a capacity", () => {
    expect(offeredLayouts({ theatre: 100, boardroom: null, banquet: 60, cabaret: null, reception: null })).toEqual([
      { layout: "theatre", capacity: 100 },
      { layout: "banquet", capacity: 60 },
    ]);
  });
});
