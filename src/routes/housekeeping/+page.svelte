<script lang="ts">
  import { onMount } from "svelte";
  import { toast } from "@ui-sdk/svelte/toast";
  import { Toggle, ToggleGroup } from "@ui-sdk/svelte/toggle";
  import { api, errorMessage } from "#lib/api.ts";
  import type { ChartEntry } from "#lib/bindings/ChartEntry.ts";
  import type { Floor } from "#lib/bindings/Floor.ts";
  import type { HousekeepingStatus } from "#lib/bindings/HousekeepingStatus.ts";
  import type { RoomHousekeeping } from "#lib/bindings/RoomHousekeeping.ts";
  import type { Space } from "#lib/bindings/Space.ts";
  import { addDays, today } from "#lib/dates.ts";

  const statusLabel: Record<HousekeepingStatus, string> = {
    dirty: "Dirty",
    clean: "Clean",
    inspected: "Ready",
  };
  const day = today();

  let floors = $state<Floor[]>([]);
  let rooms = $state<Space[]>([]);
  let states = $state<RoomHousekeeping[]>([]);
  let entries = $state<ChartEntry[]>([]);

  const byRoom = $derived(new Map(states.map((s) => [s.space_id, s])));
  const counts = $derived(
    (["dirty", "clean", "inspected"] as HousekeepingStatus[]).map((s) => ({
      status: s,
      n: states.filter((x) => x.status === s).length,
    })),
  );

  onMount(async () => {
    try {
      const [f, spaces, hk, chart] = await Promise.all([
        api.listFloors(),
        api.listSpaces(),
        api.listHousekeeping(),
        api.chart(day, addDays(day, 1)),
      ]);
      floors = f.sort((a, b) => a.level - b.level);
      rooms = spaces
        .filter((s) => s.kind === "room")
        .sort((a, b) => a.name.localeCompare(b.name, undefined, { numeric: true }));
      states = hk;
      entries = chart;
    } catch (error) {
      toast.error("Couldn't load housekeeping", { description: errorMessage(error) });
    }
  });

  /** What's happening in the room today, to help plan the cleaning. */
  function dayNote(roomId: string): string {
    const mine = entries.filter((e) => e.space_id === roomId);
    const leaving = mine.some((e) => e.stay.end === day);
    const arriving = mine.some((e) => e.stay.start === day);
    const staying = mine.some((e) => e.stay.start < day && day < e.stay.end);
    if (leaving && arriving) return "Departure and arrival today";
    if (arriving) return "Arrival today";
    if (leaving) return "Departure today";
    if (staying) return "Occupied";
    return "Empty";
  }

  async function set(room: Space, status: HousekeepingStatus) {
    try {
      await api.setHousekeeping(room.id, status);
      states = states.map((s) => (s.space_id === room.id ? { ...s, status } : s));
      toast.success(`${room.name}: ${statusLabel[status].toLowerCase()}`);
    } catch (error) {
      toast.error(`Couldn't update ${room.name}`, { description: errorMessage(error) });
    }
  }
</script>

<div class="page wide">
  <div class="heading">
    <h1>Housekeeping</h1>
    <a class="link-button" href="/lists">Daily lists</a>
  </div>
  <p class="page-intro">
    Rooms go dirty when guests check out. Mark them clean, then ready once checked.
    {#each counts as c, i (c.status)}{i ? ", " : ""}<strong>{c.n}</strong> {statusLabel[c.status].toLowerCase()}{/each}.
  </p>

  {#each floors as floor (floor.id)}
    {@const onFloor = rooms.filter((r) => r.floor_id === floor.id)}
    {#if onFloor.length}
      <section class="floor" aria-labelledby={`floor-${floor.id}`}>
        <h2 id={`floor-${floor.id}`}>{floor.name}</h2>
        <ul class="board">
          {#each onFloor as room (room.id)}
            {@const status = byRoom.get(room.id)?.status ?? "inspected"}
            <li class="card" data-status={status} aria-label={`Room ${room.name}`}>
              <div class="top">
                <span class="fob">{room.name}</span>
                <span class="state">{room.out_of_service ? "Out of service" : statusLabel[status]}</span>
              </div>
              <p class="muted">{dayNote(room.id)}</p>
              <ToggleGroup
                aria-label={`Housekeeping for room ${room.name}`}
                value={[status]}
                onValueChange={(v) => v[0] && v[0] !== status && set(room, v[0] as HousekeepingStatus)}
              >
                <Toggle value="dirty">Dirty</Toggle>
                <Toggle value="clean">Clean</Toggle>
                <Toggle value="inspected">Ready</Toggle>
              </ToggleGroup>
            </li>
          {/each}
        </ul>
      </section>
    {/if}
  {/each}
</div>

<style>
  .wide {
    max-width: 90rem;
  }

  .heading {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: calc(var(--ui-spacing) * 3);
  }

  .floor {
    margin-bottom: calc(var(--ui-spacing) * 6);
  }

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

  .card[data-status="dirty"] {
    border-left-color: var(--brass);
  }

  .card[data-status="clean"] {
    border-left-color: var(--estuary);
  }

  .card[data-status="inspected"] {
    border-left-color: var(--moss);
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
