/**
 * Format an amount stored in minor units (e.g. pence) as currency.
 * Money is always integer minor units across the app — see docs/REQUIREMENTS.md.
 */
export function formatMoney(minor: number, currency = "GBP", locale = "en-GB"): string {
  return new Intl.NumberFormat(locale, { style: "currency", currency }).format(minor / 100);
}

export function poundsToPence(pounds: number): number {
  return Math.round(pounds * 100);
}
