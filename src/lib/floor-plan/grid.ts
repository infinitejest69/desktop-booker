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
