import { describe, expect, it } from "vitest";
import { percent } from "./labels";

describe("percent", () => {
  it("shows basis points as a short percentage", () => {
    expect(percent(2000)).toBe("20%");
    expect(percent(550)).toBe("5.5%");
    expect(percent(0)).toBe("0%");
  });
});
