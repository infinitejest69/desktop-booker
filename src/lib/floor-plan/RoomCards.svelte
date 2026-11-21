<script lang="ts">
  import { Toggle, ToggleGroup } from "@ui-sdk/svelte/toggle";
  import type { HousekeepingStatus } from "../bindings/HousekeepingStatus";
  import type { Space } from "../bindings/Space";
  import { roomStatusLabel, type RoomStatus } from "./status";

  interface Props {
    rooms: Space[];
    statuses: Map<string, RoomStatus>;
    housekeeping: Map<string, HousekeepingStatus>;
    onSet: (room: Space, status: HousekeepingStatus) => void;
  }

  let { rooms, statuses, housekeeping, onSet }: Props = $props();

  const hkLabel: Record<HousekeepingStatus, string> = { dirty: "Dirty", clean: "Clean", inspected: "Ready" };
</script>

<ul class="board">
  {#each rooms as room (room.id)}
    {@const status = housekeeping.get(room.id) ?? "inspected"}
    {@const availability = statuses.get(room.id) ?? "free"}
    <li class="card" data-hk={status} data-status={availability} aria-label={`Room ${room.name}`}>
      <div class="top">
        <span class="fob">{room.name}</span>
        <span class="state">{room.out_of_service ? "Out of service" : hkLabel[status]}</span>
      </div>
      <p class="muted">{roomStatusLabel[availability]}</p>
      {#if !room.out_of_service}
        <ToggleGroup
          aria-label={`Housekeeping for room ${room.name}`}
          value={[status]}
          onValueChange={(v) => v[0] && v[0] !== status && onSet(room, v[0] as HousekeepingStatus)}
        >
          <Toggle value="dirty">Dirty</Toggle>
          <Toggle value="clean">Clean</Toggle>
          <Toggle value="inspected">Ready</Toggle>
        </ToggleGroup>
      {/if}
    </li>
  {/each}
</ul>

<style>
  .board {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(15rem, 1fr));
    gap: calc(var(--ui-spacing) * 3);
  }

  .card {
    display: flex;
    flex-direction: column;
    gap: calc(var(--ui-spacing) * 2);
    padding: calc(var(--ui-spacing) * 3);
    border: 1.5px solid var(--ui-border);
    border-left-width: 6px;
    border-radius: 8px;
    background: var(--paper);
  }

  /* The left edge is housekeeping; the background is availability, as on the plan. */
  .card[data-hk="dirty"] {
    border-left-color: var(--brass);
  }

  .card[data-hk="clean"] {
    border-left-color: var(--estuary);
  }

  .card[data-hk="inspected"] {
    border-left-color: var(--moss);
  }

  .card[data-status="in_house"] {
    background: var(--st-in-house-bg);
  }

  .card[data-status="arriving"] {
    background: var(--st-confirmed-bg);
  }

  .card[data-status="departing"],
  .card[data-status="turnover"] {
    background: var(--st-departing-bg);
  }

  .card[data-status="out_of_service"] {
    background: var(--st-done-bg);
  }

  .top {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .state {
    font-weight: 700;
  }

  .card p {
    margin: 0;
  }
</style>
