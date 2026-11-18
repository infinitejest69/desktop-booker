import { describe, expect, it } from "vitest";
import { formatMoney } from "./format";

describe("formatMoney", () => {
  it("formats pence as pounds", () => {
    expect(formatMoney(12345)).toBe("£123.45");
  });
  it("handles zero", () => {
    expect(formatMoney(0)).toBe("£0.00");
  });
});
