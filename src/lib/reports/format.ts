import type { ChartKind } from "../bindings/ChartKind";
import type { Period } from "../bindings/Period";
import type { Split } from "../bindings/Split";
import type { Unit } from "../bindings/Unit";
import { addDays, formatDay } from "../dates";
import { formatMoney } from "../format";

const tenths = (v: number) => (v / 10).toFixed(1);

/** A report value in words. Percent is per-mille; nights, days and hours are tenths. */
export function formatValue(unit: Unit, value: number): string {
  switch (unit) {
    case "money":
      return formatMoney(value);
    case "percent":
      return `${tenths(value)}%`;
    case "nights":
      return `${tenths(value)} ${value === 10 ? "night" : "nights"}`;
    case "days":
      return `${tenths(value)} ${value === 10 ? "day" : "days"}`;
    case "hours":
      return `${tenths(value)} ${value === 10 ? "hour" : "hours"}`;
    default:
      return value.toLocaleString("en-GB");
  }
}

/** A short axis label: 950, 12.9K, £420K, 50%. */
export function compact(unit: Unit, value: number): string {
  const plain = unit === "money" ? value / 100 : unit === "count" ? value : value / 10;
  const short = new Intl.NumberFormat("en-GB", { notation: "compact", maximumFractionDigits: 1 })
    .format(plain)
    .toUpperCase(); // en-GB writes "12.9k"
  if (unit === "money") return `£${short}`;
  if (unit === "percent") return `${short}%`;
  return short;
}

/** The axis top: the next 1, 2, 2.5 or 5 × a power of ten at or above `max`. */
export function niceMax(max: number): number {
  if (max <= 0) return 1;
  const power = 10 ** Math.floor(Math.log10(max));
  for (const step of [1, 2, 2.5, 5, 10]) {
    if (step * power >= max) return step * power;
  }
  return 10 * power;
}

const periodNames: Record<Exclude<Period["kind"], "fixed">, string> = {
  today: "Today",
  this_week: "This week",
  this_month: "This month",
  last_month: "Last month",
  this_year: "This year",
  last_year: "Last year",
  last12_months: "Last 12 months",
  next7_days: "Next 7 days",
  next30_days: "Next 30 days",
  next90_days: "Next 90 days",
};

export const relativePeriods = Object.keys(periodNames) as Exclude<Period["kind"], "fixed">[];

export function periodLabel(period: Period): string {
  if (period.kind !== "fixed") return periodNames[period.kind];
  const day = (iso: string) => formatDay(iso, { day: "numeric", month: "short", year: "numeric" });
  return `${day(period.start)} to ${day(addDays(period.end, -1))}`;
}

export const splitLabel: Record<Split, string> = {
  none: "Nothing (one total)",
  day: "Day",
  week: "Week",
  month: "Month",
  year: "Year",
  room_type: "Room type",
  floor: "Floor",
  room: "Room",
  board: "Board",
  source: "Booking source",
  status: "Booking status",
  revenue_type: "Revenue type",
  payment_method: "Payment method",
  suite: "Suite",
  event_kind: "Event type",
};

export const isTimeSplit = (split: Split) => ["day", "week", "month", "year"].includes(split);

export const chartLabel: Record<ChartKind, string> = {
  number: "Number",
  bar: "Bars",
  line: "Line",
  table: "Table",
};

const shortMonths = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];

/** A short label for under a column, from the row's key: "Jun", "15 Jun", "2026". */
export function axisLabel(split: Split, key: string, label: string): string {
  if (split === "year") return key;
  if (split === "month") return shortMonths[Number(key.slice(5, 7)) - 1] ?? label;
  if (split === "day" || split === "week") {
    return `${Number(key.slice(8, 10))} ${shortMonths[Number(key.slice(5, 7)) - 1] ?? ""}`.trim();
  }
  return label;
}
