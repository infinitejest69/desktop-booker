import type { BoardOption } from "../bindings/BoardOption";

/** What a room's board reads as; no board means room only. */
export function boardLabel(name: string | null | undefined): string {
  return name || "Room only";
}

/** The board name for a room, looked up from its board option id. */
export function boardName(id: string | null, boards: BoardOption[]): string {
  return boardLabel(boards.find((b) => b.id === id)?.name);
}

/** One line for a booking's boards: "Bed & breakfast", or "Bed & breakfast, Room only" when rooms differ. */
export function boardsSummary(names: (string | null)[]): string {
  return [...new Set(names.map(boardLabel))].join(", ");
}
