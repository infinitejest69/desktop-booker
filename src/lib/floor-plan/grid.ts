import type { GridRect } from "../bindings/GridRect";

// Mirrors booker-core's grid rules so the editor can react while dragging.
// The Rust side stays authoritative: every save is validated again there.

export function overlaps(a: GridRect, b: GridRect): boolean {
  if (a.w === 0 || a.h === 0 || b.w === 0 || b.h === 0) return false;
  return a.x < b.x + b.w && b.x < a.x + a.w && a.y < b.y + b.h && b.y < a.y + a.h;
}

export function fitsWithin(r: GridRect, cols: number, rows: number): boolean {
  return r.x >= 0 && r.y >= 0 && r.x + r.w <= cols && r.y + r.h <= rows;
}

export interface Placed {
  id: string;
  name: string;
  rect: GridRect;
}

export function firstOverlap(id: string, rect: GridRect, others: Placed[]): Placed | undefined {
  return others.find((o) => o.id !== id && overlaps(rect, o.rect));
}

export function findFreeSpot(
  others: GridRect[],
  cols: number,
  rows: number,
  w: number,
  h: number,
): GridRect | null {
  for (let y = 0; y + h <= rows; y++) {
    for (let x = 0; x + w <= cols; x++) {
      const candidate = { x, y, w, h };
      if (!others.some((o) => overlaps(candidate, o))) return candidate;
    }
  }
  return null;
}

/** A piece of size w×h dropped with its centre on cell (cx, cy), kept inside the grid. */
export function placeAt(cx: number, cy: number, w: number, h: number, cols: number, rows: number): GridRect {
  const clamp = (v: number, max: number) => Math.min(Math.max(v, 0), Math.max(max, 0));
  return { x: clamp(Math.round(cx - w / 2), cols - w), y: clamp(Math.round(cy - h / 2), rows - h), w, h };
}

/** The smallest grid that still holds every rect (at least 1×1). */
export function minGridSize(rects: GridRect[]): { cols: number; rows: number } {
  return {
    cols: Math.max(1, ...rects.map((r) => r.x + r.w)),
    rows: Math.max(1, ...rects.map((r) => r.y + r.h)),
  };
}

/** Up to `count` free spots of size w×h, filled left to right, top to bottom. */
export function freeSpots(
  others: GridRect[],
  cols: number,
  rows: number,
  w: number,
  h: number,
  count: number,
): GridRect[] {
  const taken = [...others];
  const found: GridRect[] = [];
  while (found.length < count) {
    const spot = findFreeSpot(taken, cols, rows, w, h);
    if (!spot) break;
    found.push(spot);
    taken.push(spot);
  }
  return found;
}
