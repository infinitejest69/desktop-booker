import type { RoomFeature } from "../bindings/RoomFeature";

/** In the order they're shown (the same order Rust sorts them in). */
export const roomFeatures: RoomFeature[] = ["wifi", "pets_allowed", "accessible", "smoking"];

export const featureLabel: Record<RoomFeature, string> = {
  wifi: "Wifi",
  pets_allowed: "Pets allowed",
  accessible: "Accessible",
  smoking: "Smoking",
};

/** What a guest can ask for: any feature, or a non-smoking room (one without "smoking"). */
export type RoomNeed = RoomFeature | "non_smoking";

export const needLabel: Record<RoomNeed, string> = { ...featureLabel, non_smoking: "Non-smoking" };

export const roomNeeds: RoomNeed[] = ["wifi", "pets_allowed", "accessible", "non_smoking", "smoking"];

/** Whether a room with these features meets every need. */
export function matchesNeeds(features: readonly RoomFeature[], needs: readonly RoomNeed[]): boolean {
  return needs.every((need) => (need === "non_smoking" ? !features.includes("smoking") : features.includes(need)));
}

/** "Wifi, accessible, non-smoking": the room's features in words. */
export function roomFeatureSummary(features: readonly RoomFeature[]): string {
  const words = roomFeatures.filter((f) => features.includes(f)).map((f) => featureLabel[f].toLowerCase());
  if (!features.includes("smoking")) words.push("non-smoking");
  const text = words.join(", ");
  return text.charAt(0).toUpperCase() + text.slice(1);
}
