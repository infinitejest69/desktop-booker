import { describe, expect, it } from "vitest";
import { featureLabel, matchesNeeds, roomFeatureSummary } from "./features";

describe("room features", () => {
  it("labels every feature", () => {
    expect(featureLabel.pets_allowed).toBe("Pets allowed");
    expect(featureLabel.accessible).toBe("Accessible");
  });

  it("a room matches when it has every feature asked for", () => {
    expect(matchesNeeds(["wifi", "pets_allowed"], ["pets_allowed"])).toBe(true);
    expect(matchesNeeds(["wifi"], ["pets_allowed"])).toBe(false);
    expect(matchesNeeds(["wifi"], [])).toBe(true);
  });

  it("non-smoking means the room isn't a smoking room", () => {
    expect(matchesNeeds(["wifi"], ["non_smoking"])).toBe(true);
    expect(matchesNeeds(["smoking"], ["non_smoking"])).toBe(false);
    expect(matchesNeeds(["smoking"], ["smoking"])).toBe(true);
  });

  it("summarises a room in words, non-smoking included", () => {
    expect(roomFeatureSummary(["wifi", "accessible"])).toBe("Wifi, accessible, non-smoking");
    expect(roomFeatureSummary(["smoking"])).toBe("Smoking");
    expect(roomFeatureSummary([])).toBe("Non-smoking");
  });
});
