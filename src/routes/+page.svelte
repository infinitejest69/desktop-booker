<script lang="ts">
  import { onMount } from "svelte";
  import { Button } from "@ui-sdk/svelte/button";
  import { Tabs, TabsList, TabsPanel, TabsTab } from "@ui-sdk/svelte/tabs";
  import { toast } from "@ui-sdk/svelte/toast";
  import FloorPlanView from "#lib/floor-plan/FloorPlanView.svelte";
  import { api, errorMessage } from "#lib/api.ts";
  import type { Floor } from "#lib/bindings/Floor.ts";
  import type { RoomType } from "#lib/bindings/RoomType.ts";
  import type { Space } from "#lib/bindings/Space.ts";

  let loaded = $state(false);
  let hotelName = $state("");
  let floors = $state<Floor[]>([]);
  let roomTypes = $state<RoomType[]>([]);
  let spaces = $state<Space[]>([]);

  const typeCodes = $derived(new Map(roomTypes.map((t) => [t.id, t.code])));
  const roomCount = $derived(spaces.filter((s) => s.kind === "room").length);
  const suiteCount = $derived(spaces.filter((s) => s.kind === "suite").length);

  async function load() {
    try {
      const [settings, f, t, s] = await Promise.all([
        api.getSettings(),
        api.listFloors(),
        api.listRoomTypes(),
        api.listSpaces(),
      ]);
      hotelName = settings.hotel_name;
      floors = f;
      roomTypes = t;
      spaces = s;
    } catch (error) {
      toast.error("Couldn't load the floor plan", { description: errorMessage(error) });
    } finally {
      loaded = true;
    }
  }

  onMount(load);

  async function loadDemo() {
    try {
      if (await api.seedDemoData()) toast.success("Demo hotel loaded");
      await load();
    } catch (error) {
      toast.error("Couldn't load demo data", { description: errorMessage(error) });
    }
  }
</script>

<div class="page">
  <h1>{hotelName || "Floor plan"}</h1>
  {#if !loaded}
    <p class="muted">Loading…</p>
  {:else if floors.length === 0}
    <div class="panel stack" style="max-width: 32rem">
      <h2>Welcome</h2>
      <p class="muted" style="margin: 0">
        Start by adding floors, room types and rooms, or load a demo hotel to look around.
      </p>
      <div class="row">
        <Button data-variant="solid" onclick={loadDemo}>Load demo hotel</Button>
        <a href="/setup/floors">Set up floors</a>
      </div>
    </div>
  {:else}
    <p class="page-intro">{roomCount} bedrooms · {suiteCount} function suites · {floors.length} floors</p>
    <Tabs defaultValue={floors[0].id}>
      <TabsList aria-label="Floors">
        {#each floors as floor (floor.id)}
          <TabsTab value={floor.id}>{floor.name}</TabsTab>
        {/each}
      </TabsList>
      {#each floors as floor (floor.id)}
        <TabsPanel value={floor.id}>
          <div class="legend" aria-hidden="true">
            <span data-key="room">Bedroom</span>
            <span data-key="suite">Function suite</span>
            <span data-key="oos">Out of service</span>
          </div>
          <FloorPlanView
            {floor}
            spaces={spaces.filter((s) => s.floor_id === floor.id)}
            {typeCodes}
          />
        </TabsPanel>
      {/each}
    </Tabs>
    <p class="muted">Booking status colours (arriving, in-house, departing) arrive in Phase 2.</p>
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

  .legend [data-key="room"]::before {
    background: light-dark(oklch(0.9 0.06 155), oklch(0.36 0.06 155));
  }

  .legend [data-key="suite"]::before {
    background: light-dark(oklch(0.92 0.06 80), oklch(0.38 0.06 80));
  }

  .legend [data-key="oos"]::before {
    background: light-dark(oklch(0.88 0 0), oklch(0.32 0 0));
  }
</style>
