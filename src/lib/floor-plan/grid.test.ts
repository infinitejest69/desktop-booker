import { describe, expect, it } from "vitest";
import { findFreeSpot, firstOverlap, fitsWithin, overlaps } from "./grid";

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
