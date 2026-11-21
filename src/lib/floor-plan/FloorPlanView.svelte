<script lang="ts">
  import type { Floor } from "../bindings/Floor";
  import type { HousekeepingStatus } from "../bindings/HousekeepingStatus";
  import type { Space } from "../bindings/Space";
  import { roomStatusLabel, type RoomStatus } from "./status";

  interface Props {
    floor: Floor;
    spaces: Space[];
    typeCodes: Map<string, string>;
    statuses: Map<string, RoomStatus>;
    cell?: number;
    /** Today's housekeeping, shown as a marker on rooms that need doing. */
    housekeeping?: Map<string, HousekeepingStatus>;
    /** Makes rooms clickable; the selected one is outlined. */
    onSelect?: (spaceId: string) => void;
    selected?: string | null;
    /** Small, for the dashboard: names only. */
    compact?: boolean;
  }

  let {
    floor,
    spaces,
    typeCodes,
    statuses,
    cell = 28,
    housekeeping,
    onSelect,
    selected = null,
    compact = false,
  }: Props = $props();

  const hkLabel: Record<HousekeepingStatus, string> = { dirty: "Dirty", clean: "Clean", inspected: "Ready" };

  function hkOf(space: Space): HousekeepingStatus | null {
    if (space.kind !== "room" || space.out_of_service || !housekeeping) return null;
    return housekeeping.get(space.id) ?? null;
  }

  /** Everything about a room in words, for screen readers and the tooltip. */
  function describe(space: Space): string {
    const hk = hkOf(space);
    return `${space.kind === "suite" ? "" : "Room "}${space.name}: ${roomStatusLabel[statusOf(space)]}${hk ? `, ${hkLabel[hk].toLowerCase()}` : ""}`;
  }

  function statusOf(space: Space): RoomStatus {
    if (space.out_of_service) return "out_of_service";
    return space.kind === "room" ? (statuses.get(space.id) ?? "free") : "free";
  }
</script>

<div class="scroll" class:compact>
  <ul
    class="plan"
    aria-label={`${floor.name} rooms`}
    style:width={`${floor.grid_cols * cell}px`}
    style:height={`${floor.grid_rows * cell}px`}
    style:--cell={`${cell}px`}
  >
    {#each spaces as space (space.id)}
      <li
        class="tile"
        class:selected={selected === space.id}
        data-kind={space.kind}
        data-status={statusOf(space)}
        data-hk={hkOf(space)}
        title={describe(space)}
        style:left={`${space.rect.x * cell}px`}
        style:top={`${space.rect.y * cell}px`}
        style:width={`${space.rect.w * cell}px`}
        style:height={`${space.rect.h * cell}px`}
      >
        {#if onSelect && space.kind === "room"}
          <button type="button" class="hit" aria-label={describe(space)} aria-pressed={selected === space.id} onclick={() => onSelect(space.id)}></button>
        {/if}
        <span class="name">{space.name}</span>
        {#if !compact}
          <span class="meta">
            {space.kind === "suite" ? "Suite" : (typeCodes.get(space.room_type_id ?? "") ?? "")}
          </span>
          {#if space.kind === "room" && statusOf(space) !== "free"}
            <span class="state">{roomStatusLabel[statusOf(space)]}</span>
          {/if}
        {/if}
        <span class="visually-hidden">, {roomStatusLabel[statusOf(space)]}</span>
        {#if hkOf(space) === "dirty" || hkOf(space) === "clean"}
          <span class="hk" data-hk={hkOf(space)}>{compact ? "" : hkLabel[hkOf(space) ?? "inspected"]}</span>
          {#if compact}<span class="visually-hidden">, {hkLabel[hkOf(space) ?? "inspected"].toLowerCase()}</span>{/if}
        {/if}
      </li>
    {/each}
  </ul>
</div>

<style>
  .scroll {
    overflow: auto;
    padding: calc(var(--ui-spacing) * 3);
    border: 1px solid var(--ui-border);
    border-radius: var(--ui-radius);
  }

  .plan {
    position: relative;
    list-style: none;
    margin: 0;
    padding: 0;
    background-image:
      linear-gradient(to right, var(--ui-border) 1px, transparent 1px),
      linear-gradient(to bottom, var(--ui-border) 1px, transparent 1px);
    background-size: var(--cell) var(--cell);
    outline: 1px solid var(--ui-border);
  }

  .tile {
    position: absolute;
    box-sizing: border-box;
    border: 2px solid var(--paper);
    border-radius: 6px;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 2px;
    overflow: hidden;
  }

  .tile[data-status="free"] {
    background: var(--paper);
    border-color: var(--rule);
  }

  .tile[data-kind="suite"][data-status="free"] {
    background: var(--heather-wash);
    border-color: var(--ui-background);
  }

  .tile[data-status="in_house"] {
    border-color: var(--st-in-house-edge);
    background: var(--st-in-house-bg);
  }

  .tile[data-status="arriving"] {
    border-color: var(--st-confirmed-edge);
    background: var(--st-confirmed-bg);
  }

  .tile[data-status="departing"] {
    border-color: var(--st-departing-edge);
    background: var(--st-departing-bg);
  }

  .tile[data-status="turnover"] {
    background: linear-gradient(
      135deg,
      var(--st-departing-bg) 50%,
      var(--st-confirmed-bg) 50%
    );
  }

  .state {
    font-size: 0.7rem;
    font-weight: 600;
  }

  .tile[data-status="out_of_service"] {
    background: var(--st-done-bg);
    background-image: repeating-linear-gradient(
      45deg,
      transparent 0 6px,
      light-dark(oklch(0 0 0 / 0.1), oklch(1 0 0 / 0.12)) 6px 12px
    );
  }

  .name {
    font-weight: 800;
  }

  .compact .name {
    font-size: 0.7rem;
  }

  .compact.scroll {
    padding: calc(var(--ui-spacing) * 2);
  }

  /* Housekeeping marker: a small label in the corner (a dot when compact). */
  .hk {
    position: absolute;
    top: 3px;
    right: 3px;
    padding: 0 4px;
    border-radius: 999px;
    font-size: 0.65rem;
    font-weight: 700;
    color: var(--ink);
  }

  .hk[data-hk="dirty"] {
    background: var(--brass);
    color: #fff;
  }

  .hk[data-hk="clean"] {
    background: var(--estuary-tint, var(--st-in-house-bg));
    outline: 1px solid var(--estuary);
  }

  .compact .hk {
    width: 8px;
    height: 8px;
    padding: 0;
  }

  .hit {
    position: absolute;
    inset: 0;
    z-index: 1;
    border: 0;
    background: transparent;
    cursor: pointer;
  }

  .hit:focus-visible {
    outline: 3px solid var(--heather);
    outline-offset: -3px;
    border-radius: 6px;
  }

  .tile.selected {
    box-shadow: 0 0 0 3px var(--heather);
    z-index: 2;
  }

  .meta {
    font-size: 0.75rem;
  }

  .visually-hidden {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip-path: inset(50%);
    white-space: nowrap;
  }
</style>
