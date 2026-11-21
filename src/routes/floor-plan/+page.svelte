<script lang="ts">
  import { onMount } from "svelte";
  import { Button } from "@ui-sdk/svelte/button";
  import { Field, FieldLabel } from "@ui-sdk/svelte/field";
  import { Input } from "@ui-sdk/svelte/input";
  import { Tabs, TabsList, TabsPanel, TabsTab } from "@ui-sdk/svelte/tabs";
  import { toast } from "@ui-sdk/svelte/toast";
  import FloorPlanView from "#lib/floor-plan/FloorPlanView.svelte";
  import { api, errorMessage } from "#lib/api.ts";
  import type { Floor } from "#lib/bindings/Floor.ts";
  import type { RoomType } from "#lib/bindings/RoomType.ts";
  import type { ChartEntry } from "#lib/bindings/ChartEntry.ts";
  import type { Space } from "#lib/bindings/Space.ts";
  import { addDays, today } from "#lib/dates.ts";
  import { roomStatusLabel, roomStatuses, type RoomStatus } from "#lib/floor-plan/status.ts";

  let loaded = $state(false);
  let floors = $state<Floor[]>([]);
  let roomTypes = $state<RoomType[]>([]);
  let spaces = $state<Space[]>([]);
  let date = $state(today());
  let entries = $state<ChartEntry[]>([]);

  const statuses = $derived(roomStatuses(spaces.filter((s) => s.kind === "room"), entries, date));
  const counts = $derived(
    [...statuses.values()].reduce(
      (acc, s) => acc.set(s, (acc.get(s) ?? 0) + 1),
      new Map<RoomStatus, number>(),
    ),
  );

  async function loadEntries(d: string) {
    try {
      entries = await api.chart(addDays(d, -1), addDays(d, 1));
    } catch (error) {
      toast.error("Couldn't load bookings", { description: errorMessage(error) });
    }
  }

  $effect(() => {
    void loadEntries(date);
  });

  const typeCodes = $derived(new Map(roomTypes.map((t) => [t.id, t.code])));
  const roomCount = $derived(spaces.filter((s) => s.kind === "room").length);
  const suiteCount = $derived(spaces.filter((s) => s.kind === "suite").length);

  async function load() {
    try {
      const [f, t, s] = await Promise.all([
        api.listFloors(),
        api.listRoomTypes(),
        api.listSpaces(),
      ]);
      floors = f;
      roomTypes = t;
      spaces = s;
      await loadEntries(date);
    } catch (error) {
      toast.error("Couldn't load the floor plan", { description: errorMessage(error) });
    } finally {
      loaded = true;
    }
  }

  onMount(load);
</script>

<div class="page">
  <h1>Floor plan</h1>
  {#if !loaded}
    <p class="muted">Loading…</p>
  {:else if floors.length === 0}
    <p class="muted">No floors yet. Add them under My Hotel → Floors, or load the demo hotel from the Dashboard.</p>
  {:else}
    <p class="page-intro">{roomCount} bedrooms · {suiteCount} function suites · {floors.length} floors</p>
    <div class="row day-bar">
      <Field>
        <FieldLabel>Show the day</FieldLabel>
        <Input type="date" value={date} onValueChange={(v) => v && (date = v)} />
      </Field>
      <Button onclick={() => (date = today())}>Today</Button>
      <p class="counts" aria-live="polite">
        {#each ["arriving", "in_house", "departing", "turnover", "free"] as const as key (key)}
          <span>{roomStatusLabel[key]}: <strong>{counts.get(key) ?? 0}</strong></span>
        {/each}
      </p>
    </div>
    <Tabs defaultValue={floors[0].id}>
      <TabsList aria-label="Floors">
        {#each floors as floor (floor.id)}
          <TabsTab value={floor.id}>{floor.name}</TabsTab>
        {/each}
      </TabsList>
      {#each floors as floor (floor.id)}
        <TabsPanel value={floor.id}>
          <div class="legend" aria-hidden="true">
            <span data-key="free">Free</span>
            <span data-key="arriving">Arriving</span>
            <span data-key="in_house">In house</span>
            <span data-key="departing">Departing</span>
            <span data-key="suite">Function suite</span>
            <span data-key="oos">Out of service</span>
          </div>
          <FloorPlanView
            {floor}
            spaces={spaces.filter((s) => s.floor_id === floor.id)}
            {typeCodes}
            {statuses}
          />
        </TabsPanel>
      {/each}
    </Tabs>
  {/if}
</div>

<style>
  .legend {
    display: flex;
    gap: calc(var(--ui-spacing) * 5);
    margin: calc(var(--ui-spacing) * 4) 0 calc(var(--ui-spacing) * 3);
    color: var(--ui-muted-foreground);
  }

  .legend span::before {
    content: "";
    display: inline-block;
    width: 0.9em;
    height: 0.9em;
    margin-inline-end: 0.4em;
    vertical-align: -0.1em;
    border-radius: 3px;
  }

  .legend [data-key="free"]::before {
    background: light-dark(oklch(0.96 0.02 155), oklch(0.27 0.02 155));
    outline: 1px solid light-dark(oklch(0.8 0.04 155), oklch(0.4 0.04 155));
  }

  .legend [data-key="arriving"]::before {
    background: light-dark(oklch(0.86 0.08 155), oklch(0.42 0.08 155));
  }

  .legend [data-key="in_house"]::before {
    background: light-dark(oklch(0.84 0.07 245), oklch(0.4 0.07 245));
  }

  .legend [data-key="departing"]::before {
    background: light-dark(oklch(0.88 0.08 60), oklch(0.42 0.08 60));
  }

  .day-bar {
    align-items: flex-end;
  }

  .day-bar :global([data-slot="field"]) {
    flex: 0 1 11rem;
  }

  .counts {
    display: flex;
    flex-wrap: wrap;
    gap: calc(var(--ui-spacing) * 4);
    margin: 0 0 calc(var(--ui-spacing) * 2);
  }

  .legend [data-key="suite"]::before {
    background: light-dark(oklch(0.92 0.06 80), oklch(0.38 0.06 80));
  }

  .legend [data-key="oos"]::before {
    background: light-dark(oklch(0.88 0 0), oklch(0.32 0 0));
  }
</style>
