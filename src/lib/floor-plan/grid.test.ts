import { describe, expect, it } from "vitest";
import { findFreeSpot, firstOverlap, fitsWithin, freeSpots, minGridSize, overlaps, placeAt } from "./grid";

const a = { x: 0, y: 0, w: 4, h: 3 };

describe("overlaps", () => {
  it("detects shared cells both ways", () => {
    const b = { x: 3, y: 2, w: 2, h: 2 };
    expect(overlaps(a, b)).toBe(true);
    expect(overlaps(b, a)).toBe(true);
  });

  it("allows touching edges", () => {
    expect(overlaps(a, { x: 4, y: 0, w: 2, h: 3 })).toBe(false);
    expect(overlaps(a, { x: 0, y: 3, w: 4, h: 1 })).toBe(false);
  });

  it("ignores empty rects", () => {
    expect(overlaps(a, { x: 1, y: 1, w: 0, h: 1 })).toBe(false);
  });
});

describe("fitsWithin", () => {
  it("uses exclusive right and bottom edges", () => {
    expect(fitsWithin(a, 4, 3)).toBe(true);
    expect(fitsWithin(a, 3, 3)).toBe(false);
    expect(fitsWithin({ x: -1, y: 0, w: 1, h: 1 }, 4, 3)).toBe(false);
  });
});

describe("firstOverlap", () => {
  it("skips the space itself", () => {
    const placed = [
      { id: "1", name: "101", rect: a },
      { id: "2", name: "102", rect: { x: 4, y: 0, w: 4, h: 3 } },
    ];
    expect(firstOverlap("1", { x: 1, y: 0, w: 4, h: 3 }, placed)?.name).toBe("102");
    expect(firstOverlap("1", { x: 0, y: 0, w: 4, h: 3 }, placed)).toBeUndefined();
  });
});

describe("findFreeSpot", () => {
  it("returns the first free top-left position", () => {
    expect(findFreeSpot([a], 10, 5, 3, 3)).toEqual({ x: 4, y: 0, w: 3, h: 3 });
  });

  it("returns null when the floor is full", () => {
    expect(findFreeSpot([{ x: 0, y: 0, w: 4, h: 4 }], 4, 4, 1, 1)).toBeNull();
  });
});

describe("placeAt", () => {
  it("centres the piece on the drop cell", () => {
    expect(placeAt(5, 5, 3, 4, 20, 10)).toEqual({ x: 4, y: 3, w: 3, h: 4 });
  });

  it("keeps the piece inside the grid", () => {
    expect(placeAt(0, 0, 3, 4, 20, 10)).toEqual({ x: 0, y: 0, w: 3, h: 4 });
    expect(placeAt(19, 9, 3, 4, 20, 10)).toEqual({ x: 17, y: 6, w: 3, h: 4 });
  });
});

describe("minGridSize", () => {
  it("is the furthest right and bottom edge", () => {
    expect(minGridSize([{ x: 2, y: 1, w: 3, h: 4 }, { x: 10, y: 0, w: 2, h: 2 }])).toEqual({ cols: 12, rows: 5 });
    expect(minGridSize([])).toEqual({ cols: 1, rows: 1 });
  });
});

describe("freeSpots", () => {
  it("fills a row, then the next", () => {
    const spots = freeSpots([], 9, 8, 3, 4, 4);
    expect(spots.map((s) => [s.x, s.y])).toEqual([[0, 0], [3, 0], [6, 0], [0, 4]]);
  });

  it("stops when the floor is full", () => {
    expect(freeSpots([], 6, 4, 3, 4, 5)).toHaveLength(2);
  });
});
