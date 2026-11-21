import { describe, expect, it } from "vitest";
import { boardLabel, boardName, boardsSummary } from "./board";

describe("board labels", () => {
  it("reads no board as room only", () => {
    expect(boardLabel(null)).toBe("Room only");
    expect(boardLabel("Bed & breakfast")).toBe("Bed & breakfast");
  });

  it("looks a board up by id", () => {
    const boards = [{ id: "b1", code: "BB", name: "Bed & breakfast", adult_price: 1200, child_price: 700 }];
    expect(boardName("b1", boards)).toBe("Bed & breakfast");
    expect(boardName(null, boards)).toBe("Room only");
  });

  it("lists each different board once", () => {
    expect(boardsSummary(["Bed & breakfast", "Bed & breakfast"])).toBe("Bed & breakfast");
    expect(boardsSummary(["Bed & breakfast", null])).toBe("Bed & breakfast, Room only");
  });
});
