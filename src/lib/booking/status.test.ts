import { describe, expect, it } from "vitest";
import { actionsFor } from "./status";

describe("actionsFor", () => {
  it("offers check-in for confirmed and check-out once in", () => {
    expect(actionsFor("confirmed").map((a) => a.to)).toContain("checked_in");
    expect(actionsFor("checked_in").map((a) => a.to)).toEqual(["checked_out", "confirmed"]);
  });

  it("never offers cancelling after check-out", () => {
    expect(actionsFor("checked_out").map((a) => a.to)).not.toContain("cancelled");
  });
});
