<script lang="ts">
  import { onMount } from "svelte";
  import { Button } from "@ui-sdk/svelte/button";
  import { Field, FieldLabel } from "@ui-sdk/svelte/field";
  import { Input } from "@ui-sdk/svelte/input";
  import { toast } from "@ui-sdk/svelte/toast";
  import { Toggle, ToggleGroup } from "@ui-sdk/svelte/toggle";
  import { goto } from "$app/navigation";
  import { page } from "$app/state";
  import FloorPlanView from "#lib/floor-plan/FloorPlanView.svelte";
  import RoomCards from "#lib/floor-plan/RoomCards.svelte";
  import FeatureIcons from "#lib/rooms/FeatureIcons.svelte";
  import { roomFeatureSummary } from "#lib/rooms/features.ts";
  import { api, errorMessage } from "#lib/api.ts";
  import type { ChartEntry } from "#lib/bindings/ChartEntry.ts";
  import type { Floor } from "#lib/bindings/Floor.ts";
  import type { HousekeepingStatus } from "#lib/bindings/HousekeepingStatus.ts";
  import type { RoomType } from "#lib/bindings/RoomType.ts";
  import type { Space } from "#lib/bindings/Space.ts";
  import type { SuiteBookingEntry } from "#lib/bindings/SuiteBookingEntry.ts";
  import { timePart } from "#lib/suites/suites.ts";
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
  let selected = $state<string | null>(null);
  let hires = $state<SuiteBookingEntry[]>([]);
  /** Suite use is shown at this time ("HH:MM"), or across the day when empty. */
  let at = $state(new Date().toTimeString().slice(0, 5));

  const isToday = $derived(date === today());
  const suiteCount = $derived(spaces.filter((s) => s.kind === "suite").length);
  const rooms = $derived(spaces.filter((s) => s.kind === "room"));
  const statuses = $derived(roomStatuses(rooms, entries, date));
  const counts = $derived(
    [...statuses.values()].reduce((acc, s) => acc.set(s, (acc.get(s) ?? 0) + 1), new Map<RoomStatus, number>()),
  );
  const dirtyCount = $derived([...hk.values()].filter((s) => s === "dirty").length);
  const typeCodes = $derived(new Map(roomTypes.map((t) => [t.id, t.code])));
  const selectedRoom = $derived(spaces.find((s) => s.id === selected && s.kind === "room") ?? null);
  const selectedSuite = $derived(spaces.find((s) => s.id === selected && s.kind === "suite") ?? null);
  /** Each suite in use at the chosen time (or booked that day), with what for. */
  const suiteUse = $derived.by(() => {
    const use = new Map<string, string>();
    const moment = at ? `${date}T${at}:00` : null;
    for (const h of hires) {
      const b = h.booking;
      if (moment ? b.start <= moment && moment < b.end : true) {
        if (!use.has(b.suite_id)) use.set(b.suite_id, moment ? b.title : `${timePart(b.start)} ${b.title}`);
      }
    }
    return use;
  });
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

  $effect(() => {
    const d = date;
    api
      .listSuiteBookings(`${d}T00:00:00`, `${addDays(d, 1)}T00:00:00`)
      .then((list) => (hires = list))
      .catch(() => (hires = []));
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
      await Promise.all([loadEntries(date), loadHousekeeping()]);
    } catch (error) {
      toast.error("Couldn't load the rooms", { description: errorMessage(error) });
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
  <h1>Rooms</h1>
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
      <Field>
        <FieldLabel>Suites at</FieldLabel>
        <Input type="time" bind:value={at} />
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
    {#if view[0] !== "cards"}
      <div class="legend" aria-hidden="true">
        <span data-key="free">Free</span>
        <span data-key="arriving">Arriving</span>
        <span data-key="in_house">In house</span>
        <span data-key="departing">Departing</span>
        <span data-key="suite">Function suite</span>
        <span data-key="suite-use">Suite in use</span>
        <span data-key="oos">Out of service</span>
        {#if isToday}<span data-key="dirty">Dirty</span>{/if}
      </div>
    {/if}

    <div class="floors">
      {#each floors as floor (floor.id)}
        {@const onFloor = spaces.filter((s) => s.floor_id === floor.id)}
        {@const roomsOnFloor = onFloor
          .filter((s) => s.kind === "room")
          .sort((a, b) => a.name.localeCompare(b.name, undefined, { numeric: true }))}
        <section class="floor" aria-labelledby={`floor-${floor.id}`}>
          <div class="floor-bar">
            <h2 id={`floor-${floor.id}`}>{floor.name}</h2>
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
                {suiteUse}
                onSelect={(id) => (selected = selected === id ? null : id)}
              />
              {#if selectedSuite && selectedSuite.floor_id === floor.id}
                {@const own = hires.filter((h) => h.booking.suite_id === selectedSuite.id)}
                <aside class="panel stack room-panel" aria-label={selectedSuite.name}>
                  <div class="room-head">
                    <strong>{selectedSuite.name}</strong>
                    <Button onclick={() => (selected = null)}>Close</Button>
                  </div>
                  <p>{formatDay(date, { weekday: "long", day: "numeric", month: "short" })}</p>
                  {#each own as h (h.booking.id)}
                    <div class="stay">
                      <a href={`/suites/booking?id=${h.booking.id}`}>{h.booking.title}</a>
                      <span class="muted">
                        {timePart(h.booking.start)} to {timePart(h.booking.end)}, {h.booking.attendees} people{h.event_name ? `, ${h.event_name}` : ""}
                      </span>
                    </div>
                  {:else}
                    <p class="muted">Not booked this day.</p>
                  {/each}
                  <a class="link-button" href={`/suites/booking?suite=${selectedSuite.id}&date=${date}`}>Book this suite</a>
                </aside>
              {/if}
              {#if selectedRoom && selectedRoom.floor_id === floor.id}
                {@const status = hk.get(selectedRoom.id) ?? "inspected"}
                <aside class="panel stack room-panel" aria-label={`Room ${selectedRoom.name}`}>
                  <div class="room-head">
                    <span class="fob">{selectedRoom.name}</span>
                    <span>{typeCodes.get(selectedRoom.room_type_id ?? "") ?? ""}</span>
                    <Button onclick={() => (selected = null)}>Close</Button>
                  </div>
                  <p class="room-features">
                    <FeatureIcons features={selectedRoom.features} />
                    {roomFeatureSummary(selectedRoom.features)}
                  </p>
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
        </section>
      {/each}
    </div>
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
    margin: 0 0 calc(var(--ui-spacing) * 3);
  }

  .floors {
    display: flex;
    flex-direction: column;
    gap: calc(var(--ui-spacing) * 8);
    margin-top: calc(var(--ui-spacing) * 4);
  }

  .floor {
    padding-top: calc(var(--ui-spacing) * 4);
    border-top: 1px solid var(--ui-border);
  }

  .floor-bar h2 {
    margin: 0;
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

  .legend [data-key="suite-use"]::before {
    background: var(--heather);
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

  .room-features {
    display: flex;
    align-items: center;
    gap: calc(var(--ui-spacing) * 2);
    color: var(--ink-muted);
  }

  .stay {
    display: flex;
    flex-direction: column;
  }
</style>
