import type { ChartEntry } from "../bindings/ChartEntry";
import { daysBetween } from "../dates";

export interface Bar {
  entry: ChartEntry;
  /** First visible night (column) of the stay. */
  col: number;
  /** Visible nights, clipped to the window. */
  span: number;
  /** The stay starts before the window or ends after it. */
  clippedStart: boolean;
  clippedEnd: boolean;
}

/** Bars per room, positioned in a window of `days` nights starting at `from`. */
export function layoutBars(entries: ChartEntry[], from: string, days: number): Map<string, Bar[]> {
  const bars = new Map<string, Bar[]>();
  for (const entry of entries) {
    const start = daysBetween(from, entry.stay.start);
    const end = daysBetween(from, entry.stay.end);
    const col = Math.max(start, 0);
    const last = Math.min(end, days);
    if (last <= col) continue;
    const list = bars.get(entry.space_id) ?? [];
    list.push({ entry, col, span: last - col, clippedStart: start < 0, clippedEnd: end > days });
    bars.set(entry.space_id, list);
  }
  return bars;
}

/** The bar covering a given column of a room, if any. */
export function barAt(bars: Bar[] | undefined, col: number): Bar | undefined {
  return bars?.find((b) => col >= b.col && col < b.col + b.span);
}
