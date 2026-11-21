<script lang="ts">
  import { onMount } from "svelte";
  import { Button } from "@ui-sdk/svelte/button";
  import { Field, FieldLabel } from "@ui-sdk/svelte/field";
  import { Input } from "@ui-sdk/svelte/input";
  import { Tabs, TabsList, TabsPanel, TabsTab } from "@ui-sdk/svelte/tabs";
  import { toast } from "@ui-sdk/svelte/toast";
  import { Toggle, ToggleGroup } from "@ui-sdk/svelte/toggle";
  import { goto } from "$app/navigation";
  import { page } from "$app/state";
  import FloorPlanView from "#lib/floor-plan/FloorPlanView.svelte";
  import RoomCards from "#lib/floor-plan/RoomCards.svelte";
  import { api, errorMessage } from "#lib/api.ts";
  import type { ChartEntry } from "#lib/bindings/ChartEntry.ts";
  import type { Floor } from "#lib/bindings/Floor.ts";
  import type { HousekeepingStatus } from "#lib/bindings/HousekeepingStatus.ts";
  import type { RoomType } from "#lib/bindings/RoomType.ts";
  import type { Space } from "#lib/bindings/Space.ts";
  import { boardLabel } from "#lib/booking/board.ts";
  import { addDays, formatDay, today } from "#lib/dates.ts";
  import { roomStatusLabel, roomStatuses, type RoomStatus } from "#lib/floor-plan/status.ts";

  const hkLabel: Record<HousekeepingStatus, string> = { dirty: "Dirty", clean: "Clean", inspected: "Ready" };

  let loaded = $state(false);
  let floors = $state<Floor[]>([]);
  let roomTypes = $state<RoomType[]>([]);
  let spaces = $state<Space[]>([]);
  let date = $state(today());
  let entries = $state<ChartEntry[]>([]);
  let hk = $state(new Map<string, HousekeepingStatus>());
  let view = $state<string[]>([page.url.searchParams.get("view") === "cards" ? "cards" : "plan"]);
  let floorTab = $state<string | null>(null);
  let selected = $state<string | null>(null);

  const isToday = $derived(date === today());
  const suiteCount = $derived(spaces.filter((s) => s.kind === "suite").length);
  const rooms = $derived(spaces.filter((s) => s.kind === "room"));
  const statuses = $derived(roomStatuses(rooms, entries, date));
  const counts = $derived(
    [...statuses.values()].reduce((acc, s) => acc.set(s, (acc.get(s) ?? 0) + 1), new Map<RoomStatus, number>()),
  );
  const dirtyCount = $derived([...hk.values()].filter((s) => s === "dirty").length);
  const typeCodes = $derived(new Map(roomTypes.map((t) => [t.id, t.code])));
  const selectedRoom = $derived(spaces.find((s) => s.id === selected) ?? null);
  /** Who's in the selected room around the chosen day. */
  const selectedStays = $derived(
    entries.filter((e) => e.space_id === selected && e.stay.start <= date && date <= e.stay.end),
  );

  async function loadEntries(d: string) {
    try {
      entries = await api.chart(addDays(d, -1), addDays(d, 1));
    } catch (error) {
      toast.error("Couldn't load bookings", { description: errorMessage(error) });
    }
  }

  async function loadHousekeeping() {
    try {
      hk = new Map((await api.listHousekeeping()).map((r) => [r.space_id, r.status]));
    } catch (error) {
      toast.error("Couldn't load housekeeping", { description: errorMessage(error) });
    }
  }

  $effect(() => {
    void loadEntries(date);
  });

  // Keep the view in the address so Back returns to it.
  $effect(() => {
    const v = view[0] === "cards" ? "cards" : "plan";
    if ((page.url.searchParams.get("view") ?? "plan") !== v)
      void goto(v === "cards" ? "/floor-plan?view=cards" : "/floor-plan", { replace: true, reset: false });
  });

  onMount(async () => {
    try {
      const [f, t, s] = await Promise.all([api.listFloors(), api.listRoomTypes(), api.listSpaces()]);
      floors = f.sort((a, b) => a.level - b.level);
      roomTypes = t;
      spaces = s;
      floorTab = floors[0]?.id ?? null;
      await Promise.all([loadEntries(date), loadHousekeeping()]);
    } catch (error) {
      toast.error("Couldn't load the floor plan", { description: errorMessage(error) });
    } finally {
      loaded = true;
    }
  });

  async function setRoom(room: Space, status: HousekeepingStatus) {
    try {
      await api.setHousekeeping(room.id, status);
      hk = new Map(hk).set(room.id, status);
      toast.success(`${room.name}: ${hkLabel[status].toLowerCase()}`);
    } catch (error) {
      toast.error(`Couldn't update ${room.name}`, { description: errorMessage(error) });
    }
  }

  async function setFloor(floor: Floor, status: HousekeepingStatus) {
    try {
      const n = await api.setFloorHousekeeping(floor.id, status);
      await loadHousekeeping();
      toast.success(`${floor.name}: ${n} room${n === 1 ? "" : "s"} ${hkLabel[status].toLowerCase()}`);
    } catch (error) {
      toast.error(`Couldn't update ${floor.name}`, { description: errorMessage(error) });
    }
  }
</script>

<div class="page wide">
  <h1>Floor plan and housekeeping</h1>
  {#if !loaded}
    <p class="muted">Loading…</p>
  {:else if floors.length === 0}
    <p class="muted">No floors yet. Add them under My hotel, Floors and layout, or load the demo hotel from the Dashboard.</p>
  {:else}
    <p class="page-intro">
      {rooms.length} bedrooms and {suiteCount} function suites on {floors.length} floor{floors.length === 1 ? "" : "s"}
    </p>
    <div class="row toolbar">
      <Field>
        <FieldLabel>Show the day</FieldLabel>
        <Input type="date" value={date} onValueChange={(v) => v && (date = v)} />
      </Field>
      <Button onclick={() => (date = today())}>Today</Button>
      <ToggleGroup aria-label="View" bind:value={view}>
        <Toggle value="plan">Plan</Toggle>
        <Toggle value="cards">Cards</Toggle>
      </ToggleGroup>
      <a class="link-button lists" href="/lists">Daily lists</a>
    </div>
    <p class="counts" aria-live="polite">
      {#each ["arriving", "in_house", "departing", "turnover", "free"] as const as key (key)}
        <span>{roomStatusLabel[key]}: <strong>{counts.get(key) ?? 0}</strong></span>
      {/each}
      {#if isToday}<span>To clean: <strong>{dirtyCount}</strong></span>{/if}
    </p>
    {#if !isToday}
      <p class="muted">Housekeeping is shown for today only.</p>
    {/if}

    <Tabs bind:value={floorTab}>
      <TabsList aria-label="Floors">
        {#each floors as floor (floor.id)}
          <TabsTab value={floor.id}>{floor.name}</TabsTab>
        {/each}
      </TabsList>
      {#each floors as floor (floor.id)}
        {@const onFloor = spaces.filter((s) => s.floor_id === floor.id)}
        {@const roomsOnFloor = onFloor
          .filter((s) => s.kind === "room")
          .sort((a, b) => a.name.localeCompare(b.name, undefined, { numeric: true }))}
        <TabsPanel value={floor.id}>
          <div class="floor-bar">
            {#if view[0] !== "cards"}
              <div class="legend" aria-hidden="true">
                <span data-key="free">Free</span>
                <span data-key="arriving">Arriving</span>
                <span data-key="in_house">In house</span>
                <span data-key="departing">Departing</span>
                <span data-key="suite">Function suite</span>
                <span data-key="oos">Out of service</span>
                {#if isToday}<span data-key="dirty">Dirty</span>{/if}
              </div>
            {:else}
              <span></span>
            {/if}
            {#if isToday && roomsOnFloor.length}
              <div class="row">
                <Button onclick={() => setFloor(floor, "clean")}>Mark floor clean</Button>
                <Button onclick={() => setFloor(floor, "inspected")}>Mark floor ready</Button>
              </div>
            {/if}
          </div>

          {#if view[0] === "cards"}
            <RoomCards rooms={roomsOnFloor} {statuses} housekeeping={hk} onSet={setRoom} />
          {:else}
            <div class="plan-and-panel">
              <FloorPlanView
                {floor}
                spaces={onFloor}
                {typeCodes}
                {statuses}
                housekeeping={isToday ? hk : undefined}
                {selected}
                onSelect={(id) => (selected = selected === id ? null : id)}
              />
              {#if selectedRoom && selectedRoom.floor_id === floor.id}
                {@const status = hk.get(selectedRoom.id) ?? "inspected"}
                <aside class="panel stack room-panel" aria-label={`Room ${selectedRoom.name}`}>
                  <div class="room-head">
                    <span class="fob">{selectedRoom.name}</span>
                    <span>{typeCodes.get(selectedRoom.room_type_id ?? "") ?? ""}</span>
                    <Button onclick={() => (selected = null)}>Close</Button>
                  </div>
                  <p>
                    <strong>{roomStatusLabel[statuses.get(selectedRoom.id) ?? "free"]}</strong> on
                    {formatDay(date, { weekday: "long", day: "numeric", month: "short" })}
                  </p>
                  {#each selectedStays as e (e.booking_room_id)}
                    <div class="stay">
                      <a href={`/bookings/${e.booking_id}`}>{e.guest_name}</a>
                      <span class="muted">
                        Booking {e.reference}, {formatDay(e.stay.start, { day: "numeric", month: "short" })} to
                        {formatDay(e.stay.end, { day: "numeric", month: "short" })}, {boardLabel(e.board)}
                      </span>
                    </div>
                  {:else}
                    <p class="muted">No guests.</p>
                  {/each}
                  {#if isToday && !selectedRoom.out_of_service}
                    <ToggleGroup
                      aria-label={`Housekeeping for room ${selectedRoom.name}`}
                      value={[status]}
                      onValueChange={(v) =>
                        selectedRoom && v[0] && v[0] !== status && setRoom(selectedRoom, v[0] as HousekeepingStatus)}
                    >
                      <Toggle value="dirty">Dirty</Toggle>
                      <Toggle value="clean">Clean</Toggle>
                      <Toggle value="inspected">Ready</Toggle>
                    </ToggleGroup>
                  {/if}
                  <a class="link-button" href={`/bookings/new?room=${selectedRoom.id}&start=${date}`}>Book this room</a>
                </aside>
              {/if}
            </div>
          {/if}
        </TabsPanel>
      {/each}
    </Tabs>
  {/if}
</div>

<style>
  .wide {
    max-width: 90rem;
  }

  .toolbar {
    align-items: flex-end;
  }

  .lists {
    margin-left: auto;
  }

  .toolbar :global([data-slot="field"]) {
    flex: 0 1 11rem;
  }

  .counts {
    display: flex;
    flex-wrap: wrap;
    gap: calc(var(--ui-spacing) * 4);
    margin: calc(var(--ui-spacing) * 3) 0;
  }

  .floor-bar {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: calc(var(--ui-spacing) * 3);
    margin: calc(var(--ui-spacing) * 4) 0 calc(var(--ui-spacing) * 3);
  }

  .legend {
    display: flex;
    flex-wrap: wrap;
    gap: calc(var(--ui-spacing) * 5);
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
    background: var(--paper);
    outline: 1px solid var(--rule);
  }

  .legend [data-key="arriving"]::before {
    background: var(--st-confirmed-bg);
  }

  .legend [data-key="in_house"]::before {
    background: var(--st-in-house-bg);
  }

  .legend [data-key="departing"]::before {
    background: var(--st-departing-bg);
  }

  .legend [data-key="suite"]::before {
    background: var(--heather-wash);
  }

  .legend [data-key="oos"]::before {
    background: var(--st-done-bg);
  }

  .legend [data-key="dirty"]::before {
    background: var(--brass);
    border-radius: 999px;
  }

  .plan-and-panel {
    display: flex;
    flex-wrap: wrap;
    align-items: flex-start;
    gap: calc(var(--ui-spacing) * 4);
  }

  .plan-and-panel > :global(.scroll) {
    flex: 1 1 32rem;
    min-width: 0;
  }

  .room-panel {
    flex: 0 1 20rem;
  }

  .room-panel p {
    margin: 0;
  }

  .room-head {
    display: flex;
    align-items: center;
    gap: calc(var(--ui-spacing) * 3);
  }

  .room-head :global(button) {
    margin-left: auto;
  }

  .stay {
    display: flex;
    flex-direction: column;
  }
</style>
