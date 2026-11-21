// Dates are ISO "YYYY-MM-DD" strings in the hotel's local calendar, matching
// chrono::NaiveDate on the Rust side. Arithmetic goes through UTC so daylight
// saving changes never shift a day.

export function toISO(d: Date): string {
  return d.toISOString().slice(0, 10);
}

function parseUTC(iso: string): Date {
  const [y, m, d] = iso.split("-").map(Number);
  return new Date(Date.UTC(y, m - 1, d));
}

export function today(): string {
  const now = new Date();
  return toISO(new Date(Date.UTC(now.getFullYear(), now.getMonth(), now.getDate())));
}

export function addDays(iso: string, days: number): string {
  const d = parseUTC(iso);
  d.setUTCDate(d.getUTCDate() + days);
  return toISO(d);
}

export function daysBetween(from: string, to: string): number {
  return Math.round((parseUTC(to).getTime() - parseUTC(from).getTime()) / 86_400_000);
}

export function datesFrom(start: string, count: number): string[] {
  return Array.from({ length: count }, (_, i) => addDays(start, i));
}

export function formatDay(iso: string, options: Intl.DateTimeFormatOptions = {}): string {
  return parseUTC(iso).toLocaleDateString("en-GB", { timeZone: "UTC", ...options });
}

export function isWeekend(iso: string): boolean {
  const day = parseUTC(iso).getUTCDay();
  return day === 0 || day === 6;
}
