import { describe, expect, it } from "vitest";
import { errorMessage, isCommandError } from "./api";

describe("errorMessage", () => {
  it("uses the message from a command error", () => {
    const err = { kind: "validation", message: '"102" overlaps "101"' };
    expect(isCommandError(err)).toBe(true);
    expect(errorMessage(err)).toBe('"102" overlaps "101"');
  });

  it("falls back to Error and plain values", () => {
    expect(errorMessage(new Error("boom"))).toBe("boom");
    expect(errorMessage("offline")).toBe("offline");
    expect(isCommandError({ kind: "validation" })).toBe(false);
  });
});
