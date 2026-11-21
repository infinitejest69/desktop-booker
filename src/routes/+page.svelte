<script lang="ts">
  import { onMount } from "svelte";
  import { goto } from "$app/navigation";
  import { Button } from "@ui-sdk/svelte/button";
  import { toast } from "@ui-sdk/svelte/toast";
  import { api, errorMessage } from "#lib/api.ts";
  import type { BookingListItem } from "#lib/bindings/BookingListItem.ts";
  import type { BookingStatus } from "#lib/bindings/BookingStatus.ts";
  import type { ChartEntry } from "#lib/bindings/ChartEntry.ts";
  import type { Space } from "#lib/bindings/Space.ts";
  import { statusLabel } from "#lib/booking/status.ts";
  import TariffChart from "#lib/chart/TariffChart.svelte";
  import { todaySummary } from "#lib/dashboard/today.ts";
  import { addDays, daysBetween, formatDay, today } from "#lib/dates.ts";

  const day = today();
  const chartFrom = addDays(day, -1);

  let loaded = $state(false);
  let hotelName = $state("");
  let hasFloors = $state(false);
  let rooms = $state<Space[]>([]);
  let codes = $state(new Map<string, string>());
  let entries = $state<ChartEntry[]>([]);
  let upcoming = $state<BookingListItem[]>([]);
  let busy = $state<string | null>(null);
  let chartDays = $state(14);

  const summary = $derived(todaySummary(entries, rooms, day));
  const roomName = $derived(new Map(rooms.map((r) => [r.id, r.name])));

  async function load() {
    try {
      const [settings, floors, types, spaces] = await Promise.all([
        api.getSettings(),
        api.listFloors(),
        api.listRoomTypes(),
        api.listSpaces(),
      ]);
      hotelName = settings.hotel_name;
      hasFloors = floors.length > 0;
      const level = new Map(floors.map((f) => [f.id, f.level]));
      codes = new Map(types.map((t) => [t.id, t.code]));
      rooms = spaces
        .filter((s) => s.kind === "room")
        .sort(
          (a, b) =>
            (level.get(a.floor_id) ?? 0) - (level.get(b.floor_id) ?? 0) ||
            a.name.localeCompare(b.name, undefined, { numeric: true }),
        );
      await loadBookings();
    } catch (error) {
      toast.error("Couldn't load the dashboard", { description: errorMessage(error) });
    } finally {
      loaded = true;
    }
  }

  async function loadBookings() {
    const [e, u] = await Promise.all([
      api.chart(addDays(day, -1), addDays(day, Math.max(chartDays, 2))),
      api.searchBookings(""),
    ]);
    entries = e;
    upcoming = u.filter((b) => b.arrival > day).slice(0, 8);
  }

  onMount(load);

  $effect(() => {
    // Refetch when the chart grows to show more days.
    const count = chartDays;
    if (loaded && hasFloors && count) void loadBookings();
  });

  async function loadDemo() {
    try {
      if (await api.seedDemoData()) toast.success("Demo hotel loaded");
      await load();
    } catch (error) {
      toast.error("Couldn't load demo data", { description: errorMessage(error) });
    }
  }

  async function setStatus(entry: ChartEntry, status: BookingStatus) {
    busy = entry.booking_room_id;
    try {
      const saved = await api.setBookingStatus(entry.booking_id, status);
      toast.success(`${saved.booking.reference} ${entry.guest_name}: ${statusLabel[status].toLowerCase()}`);
      await loadBookings();
    } catch (error) {
      toast.error(`Couldn't update ${entry.reference}`, { description: errorMessage(error) });
    } finally {
      busy = null;
    }
  }

  function guests(e: ChartEntry): string {
    return `${e.adults} adult${e.adults === 1 ? "" : "s"}${e.children ? `, ${e.children} child${e.children === 1 ? "" : "ren"}` : ""}`;
  }

  function nights(e: ChartEntry): string {
    const n = daysBetween(e.stay.start, e.stay.end);
    return `${n} night${n === 1 ? "" : "s"}`;
  }
</script>

<div class="page wide">
  <div class="heading">
    <div>
      <h1>{hotelName || "Dashboard"}</h1>
      <p class="page-intro">{formatDay(day, { weekday: "long", day: "numeric", month: "long", year: "numeric" })}</p>
    </div>
    {#if hasFloors}
      <Button data-variant="solid" onclick={() => goto("/bookings/new")}>New booking</Button>
    {/if}
  </div>

  {#if !loaded}
    <p class="muted">Loading…</p>
  {:else if !hasFloors}
    <div class="panel stack" style="max-width: 32rem">
      <h2>Welcome</h2>
      <p class="muted" style="margin: 0">
        Start by setting up your hotel under My Hotel, or load a demo hotel to look around.
      </p>
      <div class="row">
        <Button data-variant="solid" onclick={loadDemo}>Load demo hotel</Button>
        <a class="link-button" href="/setup/property">Set up my hotel</a>
      </div>
    </div>
  {:else}
    <section class="tiles" aria-label="Today at a glance">
      <div class="tile" data-tone="arriving"><span class="n">{summary.arrivals.length}</span><span>Arriving</span></div>
      <div class="tile" data-tone="departing"><span class="n">{summary.departures.length}</span><span>Departing</span></div>
      <div class="tile" data-tone="in-house"><span class="n">{summary.inHouse.length}</span><span>In house</span></div>
      <div class="tile"><span class="n">{summary.freeTonight}</span><span>Free tonight</span></div>
      <div class="tile"><span class="n">{summary.occupancy}%</span><span>Occupancy tonight</span></div>
    </section>

    <div class="today">
      <section class="panel" aria-labelledby="arrivals-heading">
        <h2 id="arrivals-heading">Arrivals <span class="count">{summary.arrivals.length}</span></h2>
        {#if summary.arrivals.length}
          <ul class="people">
            {#each summary.arrivals as a (a.booking_room_id)}
              <li>
                <div>
                  <a href={`/bookings/${a.booking_id}`} class="who">{a.guest_name}</a>
                  <span class="muted">Room {roomName.get(a.space_id)} · {guests(a)} · {nights(a)}{a.status === "provisional" ? " · provisional" : ""}</span>
                </div>
                <Button data-variant="solid" disabled={busy === a.booking_room_id} onclick={() => setStatus(a, "checked_in")}>
                  Check in
                </Button>
              </li>
            {/each}
          </ul>
        {:else}
          <p class="muted">No more arrivals today.</p>
        {/if}
      </section>

      <section class="panel" aria-labelledby="departures-heading">
        <h2 id="departures-heading">Departures <span class="count">{summary.departures.length}</span></h2>
        {#if summary.departures.length}
          <ul class="people">
            {#each summary.departures as d (d.booking_room_id)}
              <li>
                <div>
                  <a href={`/bookings/${d.booking_id}`} class="who">{d.guest_name}</a>
                  <span class="muted">Room {roomName.get(d.space_id)} · {d.reference}</span>
                </div>
                {#if d.status === "checked_in"}
                  <Button data-variant="solid" disabled={busy === d.booking_room_id} onclick={() => setStatus(d, "checked_out")}>
                    Check out
                  </Button>
                {:else}
                  <span class="status" data-status={d.status}>{statusLabel[d.status]}</span>
                {/if}
              </li>
            {/each}
          </ul>
        {:else}
          <p class="muted">No more departures today{summary.checkedOutToday ? ` (${summary.checkedOutToday} already checked out)` : ""}.</p>
        {/if}
      </section>

      <section class="panel" aria-labelledby="inhouse-heading">
        <h2 id="inhouse-heading">In house <span class="count">{summary.inHouse.length}</span></h2>
        {#if summary.inHouse.length}
          <ul class="people compact">
            {#each summary.inHouse as h (h.booking_room_id)}
              <li>
                <a href={`/bookings/${h.booking_id}`} class="who">{h.guest_name}</a>
                <span class="muted">Room {roomName.get(h.space_id)} · leaves {formatDay(h.stay.end, { weekday: "short", day: "numeric", month: "short" })}</span>
              </li>
            {/each}
          </ul>
        {:else}
          <p class="muted">Nobody checked in yet.</p>
        {/if}
      </section>
    </div>

    <section aria-labelledby="chart-heading" class="chart-section">
      <div class="section-head">
        <h2 id="chart-heading">Coming weeks</h2>
        <a href="/chart" class="link-button">Full tariff chart</a>
      </div>
      <TariffChart
        {rooms}
        {codes}
        {entries}
        from={chartFrom}
        bind:days={chartDays}
        fit
        label="Coming weeks"
        onBooking={(id) => goto(`/chart?from=${chartFrom}&booking=${id}`)}
        onFree={(roomId, date) => goto(`/bookings/new?room=${roomId}&start=${date}&from=${chartFrom}`)}
      />
    </section>

    <section aria-labelledby="upcoming-heading" class="panel">
      <div class="section-head">
        <h2 id="upcoming-heading">Coming up</h2>
        <a href="/bookings" class="link-button">All bookings</a>
      </div>
      {#if upcoming.length}
        <table class="list">
          <thead><tr><th>Arrival</th><th>Guest</th><th>Rooms</th><th>Nights</th><th>Status</th></tr></thead>
          <tbody>
            {#each upcoming as b (b.id)}
              <tr>
                <td>{formatDay(b.arrival, { weekday: "short", day: "numeric", month: "short" })}</td>
                <td><a href={`/bookings/${b.id}`}>{b.guest_name}</a></td>
                <td>{b.rooms.join(", ")}</td>
                <td>{daysBetween(b.arrival, b.departure)}</td>
                <td><span class="status" data-status={b.status}>{statusLabel[b.status]}</span></td>
              </tr>
            {/each}
          </tbody>
        </table>
      {:else}
        <p class="muted">No upcoming bookings.</p>
      {/if}
    </section>
  {/if}
</div>

<style>
  .wide {
    max-width: 90rem;
  }

  .heading {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: calc(var(--ui-spacing) * 4);
  }

  .tiles {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(9rem, 1fr));
    gap: calc(var(--ui-spacing) * 3);
    margin-bottom: calc(var(--ui-spacing) * 5);
  }

  .tile {
    display: flex;
    flex-direction: column;
    gap: calc(var(--ui-spacing) * 1);
    padding: calc(var(--ui-spacing) * 3) calc(var(--ui-spacing) * 4);
    border: 1px solid var(--ui-border);
    border-radius: var(--ui-radius);
    border-inline-start: 4px solid var(--ui-border);
  }

  .tile .n {
    font-size: 1.6rem;
    font-weight: 700;
    line-height: 1.1;
  }

  .tile[data-tone="arriving"] {
    border-inline-start-color: light-dark(oklch(0.6 0.12 155), oklch(0.7 0.12 155));
  }

  .tile[data-tone="departing"] {
    border-inline-start-color: light-dark(oklch(0.65 0.12 60), oklch(0.72 0.12 60));
  }

  .tile[data-tone="in-house"] {
    border-inline-start-color: light-dark(oklch(0.55 0.12 245), oklch(0.7 0.1 245));
  }

  .today {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(20rem, 1fr));
    gap: calc(var(--ui-spacing) * 4);
    margin-bottom: calc(var(--ui-spacing) * 6);
    align-items: start;
  }

  .today h2 {
    margin-bottom: calc(var(--ui-spacing) * 3);
  }

  .count {
    display: inline-block;
    min-width: 1.4rem;
    padding: 0 0.35rem;
    border-radius: 999px;
    background: var(--ui-muted);
    text-align: center;
    font-size: 0.8rem;
  }

  .people {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
  }

  .people li {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: calc(var(--ui-spacing) * 3);
    padding: calc(var(--ui-spacing) * 2) 0;
    border-bottom: 1px solid var(--ui-border);
  }

  .people li:last-child {
    border-bottom: 0;
  }

  .people li > div {
    display: flex;
    flex-direction: column;
  }

  .people.compact li {
    justify-content: flex-start;
    flex-wrap: wrap;
  }

  .who {
    font-weight: 600;
    color: inherit;
  }

  .chart-section {
    margin-bottom: calc(var(--ui-spacing) * 6);
  }

  .section-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: calc(var(--ui-spacing) * 3);
  }
</style>
