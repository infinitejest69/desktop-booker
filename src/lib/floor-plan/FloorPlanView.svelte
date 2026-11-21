<script lang="ts">
  import type { Floor } from "../bindings/Floor";
  import type { Space } from "../bindings/Space";
  import { roomStatusLabel, type RoomStatus } from "./status";

  interface Props {
    floor: Floor;
    spaces: Space[];
    typeCodes: Map<string, string>;
    statuses: Map<string, RoomStatus>;
    cell?: number;
  }

  let { floor, spaces, typeCodes, statuses, cell = 28 }: Props = $props();

  function statusOf(space: Space): RoomStatus {
    if (space.out_of_service) return "out_of_service";
    return space.kind === "room" ? (statuses.get(space.id) ?? "free") : "free";
  }
</script>

<div class="scroll">
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
        data-kind={space.kind}
        data-status={statusOf(space)}
        style:left={`${space.rect.x * cell}px`}
        style:top={`${space.rect.y * cell}px`}
        style:width={`${space.rect.w * cell}px`}
        style:height={`${space.rect.h * cell}px`}
      >
        <span class="name">{space.name}</span>
        <span class="meta">
          {space.kind === "suite" ? "Suite" : (typeCodes.get(space.room_type_id ?? "") ?? "")}
        </span>
        {#if space.kind === "room" && statusOf(space) !== "free"}
          <span class="state">{roomStatusLabel[statusOf(space)]}</span>
        {:else}
          <span class="visually-hidden">, {roomStatusLabel[statusOf(space)]}</span>
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
