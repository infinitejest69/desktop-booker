import type { EventKind } from "../bindings/EventKind";
import type { Layout } from "../bindings/Layout";
import type { SuiteBooking } from "../bindings/SuiteBooking";
import type { SuiteBookingStatus } from "../bindings/SuiteBookingStatus";
import type { SuiteCapacities } from "../bindings/SuiteCapacities";
import type { SuitePricing } from "../bindings/SuitePricing";
import type { SuiteRates } from "../bindings/SuiteRates";

export const layoutLabel: Record<Layout, string> = {
  theatre: "Theatre",
  boardroom: "Boardroom",
  banquet: "Banquet",
  cabaret: "Cabaret",
  reception: "Reception",
};

export const LAYOUTS = Object.keys(layoutLabel) as Layout[];

export const pricingLabel: Record<SuitePricing["kind"], string> = {
  hourly: "Hourly",
  half_day: "Half day",
  full_day: "Full day",
  per_head: "Per head (package)",
  manual: "Set by hand",
};

export const suiteStatusLabel: Record<SuiteBookingStatus, string> = {
  provisional: "Provisional",
  confirmed: "Confirmed",
  cancelled: "Cancelled",
};

export const eventKindLabel: Record<EventKind, string> = {
  wedding: "Wedding",
  conference: "Conference",
  dinner: "Dinner",
  party: "Party",
  other: "Other",
};

/** The layouts a suite offers, with how many each seats. */
export function offeredLayouts(caps: SuiteCapacities): { layout: Layout; capacity: number }[] {
  return LAYOUTS.flatMap((layout) => {
    const capacity = caps[layout];
    return capacity ? [{ layout, capacity }] : [];
  });
}

/** "2026-06-20T15:00:00" from a date and an "HH:MM" time. */
export function joinDateTime(date: string, time: string): string {
  return `${date}T${time.slice(0, 5)}:00`;
}

/** The date part of a date-time. */
export const datePart = (dt: string) => dt.slice(0, 10);
/** "15:00" from a date-time or a time. */
export const timePart = (t: string) => (t.includes("T") ? t.slice(11, 16) : t.slice(0, 5));

export function minutesBetween(from: string, to: string): number {
  return Math.round((Date.parse(to) - Date.parse(from)) / 60_000);
}

/**
 * The price the store will charge, for showing while the form is filled in.
 * Mirrors `price_suite_booking` in booker-core; `null` if the suite doesn't
 * offer the chosen kind of pricing.
 */
export function estimateSuitePrice(
  booking: Pick<SuiteBooking, "start" | "end" | "attendees" | "pricing">,
  rates: SuiteRates,
): number | null {
  const p = booking.pricing;
  switch (p.kind) {
    case "hourly": {
      if (rates.hourly === null) return null;
      const hours = Math.max(Math.ceil(Math.max(minutesBetween(booking.start, booking.end), 0) / 60), rates.min_hours);
      return rates.hourly * hours;
    }
    case "half_day":
      return rates.half_day;
    case "full_day":
      return rates.full_day;
    case "per_head":
      return p.price * booking.attendees;
    case "manual":
      return p.price;
  }
}

export interface TimelineBar {
  /** Where the booking itself sits, as percentages of the window. */
  left: number;
  width: number;
  /** Including setup and clear-down. */
  padLeft: number;
  padWidth: number;
}

/** Positions a booking on a timeline running `[from, to)`, clipped to it. */
export function timelineBar(
  b: Pick<SuiteBooking, "start" | "end" | "setup_minutes" | "cleardown_minutes">,
  from: string,
  to: string,
): TimelineBar {
  const span = minutesBetween(from, to);
  const pct = (minutes: number) => (Math.min(Math.max(minutes, 0), span) / span) * 100;
  const start = minutesBetween(from, b.start);
  const end = minutesBetween(from, b.end);
  const left = pct(start);
  const padLeft = pct(start - b.setup_minutes);
  return {
    left,
    width: pct(end) - left,
    padLeft,
    padWidth: pct(end + b.cleardown_minutes) - padLeft,
  };
}
