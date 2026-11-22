<script lang="ts">
  import { onMount } from "svelte";
  import { goto } from "$app/navigation";
  import { page } from "$app/state";
  import {
    Drawer,
    DrawerClose,
    DrawerContent,
    DrawerTitle,
  } from "@ui-sdk/svelte/drawer";
  import BookingSummary from "#lib/booking/BookingSummary.svelte";
  import type { BookingDetail } from "#lib/bindings/BookingDetail.ts";
  import { Button } from "@ui-sdk/svelte/button";
  import { Field, FieldLabel } from "@ui-sdk/svelte/field";
  import { Input } from "@ui-sdk/svelte/input";
  import { toast } from "@ui-sdk/svelte/toast";
  import { api, errorMessage } from "#lib/api.ts";
  import type { ChartEntry } from "#lib/bindings/ChartEntry.ts";
  import type { Floor } from "#lib/bindings/Floor.ts";
  import type { RoomType } from "#lib/bindings/RoomType.ts";
  import type { Space } from "#lib/bindings/Space.ts";
  import TariffChart from "#lib/chart/TariffChart.svelte";
  import ViewSwitch from "#lib/nav/ViewSwitch.svelte";
  import { bookingViews } from "#lib/nav/views.ts";
  import { addDays, today } from "#lib/dates.ts";


  const defaultFrom = addDays(today(), -2);
  const from = $derived(page.url.searchParams.get("from") || defaultFrom);
  const openBooking = $derived(page.url.searchParams.get("booking"));
  const highlight = $derived(page.url.searchParams.get("highlight"));
  let panel = $state<BookingDetail | null>(null);
  let openedHere = false;

  function chartUrl(params: { from?: string; booking?: string | null; highlight?: string | null }) {
    const q = new URLSearchParams({ from: params.from ?? from });
    if (params.booking) q.set("booking", params.booking);
    if (params.highlight) q.set("highlight", params.highlight);
    return `/chart?${q}`;
  }

  let search = $state("");

  /** Searching from the calendar shows the results in the list view. */
  function searchBookings(event: SubmitEvent) {
    event.preventDefault();
    const q = search.trim();
    if (q) void goto(`/bookings?q=${encodeURIComponent(q)}`);
  }

  function setFrom(next: string) {
    void goto(chartUrl({ from: next, highlight }), { replace: true, reset: false });
  }

  function openPanel(bookingId: string) {
    const switching = openBooking !== null;
    if (!switching) openedHere = true;
    void goto(chartUrl({ booking: bookingId }), { replace: switching, reset: false });
  }

  function closePanel() {
    if (openedHere) {
      openedHere = false;
      history.back();
    } else {
      void goto(chartUrl({}), { replace: true, reset: false });
    }
  }

  $effect(() => {
    const id = openBooking;
    if (!id) {
      panel = null;
      return;
    }
    api
      .getBooking(id)
      .then((d) => (panel = d))
      .catch((error) => toast.error("Couldn't open the booking", { description: errorMessage(error) }));
  });
  let floors = $state<Floor[]>([]);
  let roomTypes = $state<RoomType[]>([]);
  let rooms = $state<Space[]>([]);
  let entries = $state<ChartEntry[]>([]);
  let days = $state(21);

  const codes = $derived(new Map(roomTypes.map((t) => [t.id, t.code])));
  const roomNames = $derived(new Map(rooms.map((r) => [r.id, r.name])));

  async function loadSetup() {
    try {
      const [f, t, s] = await Promise.all([api.listFloors(), api.listRoomTypes(), api.listSpaces()]);
      const level = new Map(f.map((x) => [x.id, x.level]));
      floors = f;
      roomTypes = t;
      rooms = s
        .filter((x) => x.kind === "room")
        .sort(
          (a, b) =>
            (level.get(a.floor_id) ?? 0) - (level.get(b.floor_id) ?? 0) ||
            a.name.localeCompare(b.name, undefined, { numeric: true }),
        );
    } catch (error) {
      toast.error("Couldn't load rooms", { description: errorMessage(error) });
    }
  }

  async function loadEntries(start: string, count: number) {
    try {
      entries = await api.chart(start, addDays(start, count));
    } catch (error) {
      toast.error("Couldn't load bookings", { description: errorMessage(error) });
    }
  }

  onMount(loadSetup);
  $effect(() => {
    void loadEntries(from, days);
  });
</script>

<div class="page wide">
  <h1>Booking calendar</h1>
  <p class="page-intro">
    Choose a free night to start a booking, or a booking to open it. Use the arrow keys to move
    around the calendar and Enter to choose.
  </p>

  <div class="row toolbar">
    <ViewSwitch views={bookingViews} current="Calendar" />
    <Button onclick={() => setFrom(addDays(from, -7))}>← Week</Button>
    <Button onclick={() => setFrom(defaultFrom)}>Today</Button>
    <Button onclick={() => setFrom(addDays(from, 7))}>Week →</Button>
    <Field>
      <FieldLabel>Start from</FieldLabel>
      <Input type="date" value={from} onValueChange={(v) => v && setFrom(v)} />
    </Field>
    <form class="search" role="search" onsubmit={searchBookings}>
      <Field>
        <FieldLabel>Search bookings</FieldLabel>
        <Input type="search" bind:value={search} placeholder="Reference, name or email, then Enter" />
      </Field>
    </form>
    <Button data-variant="solid" onclick={() => goto(`/bookings/new?from=${from}`)}>New booking</Button>
  </div>

  {#if floors.length && rooms.length === 0}
    <p class="muted">No bedrooms yet. Add some under My hotel → Floors and layout.</p>
  {:else}
    <TariffChart
      {rooms}
      {codes}
      {entries}
      {from}
      bind:days
      fit
      highlight={[highlight, openBooking]}
      onBooking={openPanel}
      onFree={(roomId, date) => goto(`/bookings/new?room=${roomId}&start=${date}&from=${from}`)}
    />
  {/if}
</div>

<Drawer
  side="right"
  modal={false}
  open={openBooking !== null}
  onOpenChange={(open) => {
    if (!open) closePanel();
  }}
>
  <DrawerContent class="booking-panel">
    {#if panel}
      <div class="panel-head">
        <div>
          <DrawerTitle>{panel.guest.first_name} {panel.guest.last_name}</DrawerTitle>
          <p class="muted ref">Booking {panel.booking.reference}</p>
        </div>
        <DrawerClose aria-label="Close">
          <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M18 6 6 18M6 6l12 12" /></svg>
        </DrawerClose>
      </div>
      <BookingSummary
        detail={panel}
        {roomNames}
        compact
        onChange={(d) => {
          panel = d;
          void loadEntries(from, days);
        }}
      />
      <div class="row panel-links">
        <a class="link-button" href={`/bookings/${panel.booking.id}?edit=1`}>Edit booking</a>
        <a class="link-button" href={`/bookings/${panel.booking.id}`}>Open full page</a>
      </div>
    {:else}
      <DrawerTitle>Loading booking…</DrawerTitle>
    {/if}
  </DrawerContent>
</Drawer>

<style>
  .wide {
    max-width: none;
  }

  .toolbar {
    margin-bottom: var(--app-gap);
  }

  .toolbar :global([data-slot="field"]) {
    flex: 0 1 11rem;
  }

  .search {
    display: flex;
    flex: 1 1 16rem;
  }

  .toolbar .search :global([data-slot="field"]) {
    flex: 1 1 auto;
  }

  :global(.booking-panel[data-side="right"]) {
    width: min(30rem, 92vw);
    padding: calc(var(--ui-spacing) * 6);
    display: flex;
    flex-direction: column;
    gap: calc(var(--ui-spacing) * 4);
    overflow-y: auto;
  }

  .panel-head {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: calc(var(--ui-spacing) * 3);
  }

  .panel-head :global(svg) {
    width: 1.1rem;
    height: 1.1rem;
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
    stroke-linecap: round;
  }

  :global(.booking-panel [data-slot="drawer-title"]) {
    font-family: var(--font-sign);
    font-weight: 400;
    font-size: 1.5rem;
    line-height: 1.2;
    margin: 0;
  }

  .ref {
    margin: 0;
  }

  .panel-links {
    padding-top: calc(var(--ui-spacing) * 2);
    border-top: 1px solid var(--ui-border);
  }

</style>
