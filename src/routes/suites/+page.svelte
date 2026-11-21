<script lang="ts">
  import { Button } from "@ui-sdk/svelte/button";
  import { Field, FieldLabel } from "@ui-sdk/svelte/field";
  import { Input } from "@ui-sdk/svelte/input";
  import { toast } from "@ui-sdk/svelte/toast";
  import { Toggle, ToggleGroup } from "@ui-sdk/svelte/toggle";
  import { goto } from "$app/navigation";
  import { page } from "$app/state";
  import { api, errorMessage } from "#lib/api.ts";
  import type { Space } from "#lib/bindings/Space.ts";
  import type { SuiteBookingEntry } from "#lib/bindings/SuiteBookingEntry.ts";
  import { addDays, datesFrom, formatDay, isWeekend, today } from "#lib/dates.ts";
  import { formatMoney } from "#lib/format.ts";
  import { datePart, layoutLabel, suiteStatusLabel, timelineBar, timePart } from "#lib/suites/suites.ts";

  /** The day view runs from 07:00 to midnight. */
  const FIRST_HOUR = 7;
  const HOURS = Array.from({ length: 24 - FIRST_HOUR }, (_, i) => FIRST_HOUR + i);

  let day = $state(page.url.searchParams.get("date") ?? today());
  let view = $state<string[]>([page.url.searchParams.get("view") === "week" ? "week" : "day"]);
  let suites = $state<Space[]>([]);
  let entries = $state<SuiteBookingEntry[]>([]);
  let loaded = $state(false);

  const week = $derived(view[0] === "week");
  const days = $derived(week ? datesFrom(day, 7) : [day]);
  const windowFrom = $derived(`${day}T${String(FIRST_HOUR).padStart(2, "0")}:00:00`);
  const windowTo = $derived(`${addDays(day, 1)}T00:00:00`);

  $effect(() => {
    const from = `${day}T00:00:00`;
    const to = `${addDays(day, week ? 7 : 1)}T00:00:00`;
    Promise.all([api.listSpaces(), api.listSuiteBookings(from, to)])
      .then(([spaces, list]) => {
        suites = spaces.filter((s) => s.kind === "suite").sort((a, b) => a.name.localeCompare(b.name));
        entries = list;
      })
      .catch((error) => toast.error("Couldn't load suite bookings", { description: errorMessage(error) }))
      .finally(() => (loaded = true));
  });

  // Keep the date and view in the address so Back returns here.
  $effect(() => {
    const url = `/suites?date=${day}${week ? "&view=week" : ""}`;
    if (page.url.pathname + page.url.search !== url) void goto(url, { replace: true, reset: false });
  });

  function forSuite(suiteId: string): SuiteBookingEntry[] {
    return entries.filter((e) => e.booking.suite_id === suiteId);
  }

  function onDay(suiteId: string, date: string): SuiteBookingEntry[] {
    return forSuite(suiteId).filter((e) => datePart(e.booking.start) === date);
  }

  function times(e: SuiteBookingEntry): string {
    return `${timePart(e.booking.start)} to ${timePart(e.booking.end)}`;
  }

  function label(e: SuiteBookingEntry): string {
    const b = e.booking;
    const event = e.event_name && e.event_name !== b.title ? ` (${e.event_name})` : "";
    return `${e.suite_name}, ${times(e)}: ${b.title}${event}, ${b.attendees} ${layoutLabel[b.layout].toLowerCase()}, ${suiteStatusLabel[b.status].toLowerCase()}`;
  }

  const step = (n: number) => (day = addDays(day, n * (week ? 7 : 1)));
</script>

<div class="page wide">
  <div class="heading">
    <h1>Function suites</h1>
    <Button data-variant="solid" onclick={() => goto(`/suites/booking?date=${day}`)}>Book a suite</Button>
  </div>

  <div class="row toolbar" role="toolbar" aria-label="Choose the days shown">
    <Field>
      <FieldLabel>{week ? "Week from" : "Day"}</FieldLabel>
      <Input type="date" value={day} onValueChange={(v) => v && (day = v)} />
    </Field>
    <div class="row">
      <Button onclick={() => step(-1)}>{week ? "Previous week" : "Previous day"}</Button>
      <Button onclick={() => (day = today())}>Today</Button>
      <Button onclick={() => step(1)}>{week ? "Next week" : "Next day"}</Button>
    </div>
    <ToggleGroup aria-label="View" bind:value={view}>
      <Toggle value="day">Day</Toggle>
      <Toggle value="week">Week</Toggle>
    </ToggleGroup>
  </div>

  {#if loaded && !suites.length}
    <p class="muted">No function suites yet. Add them under My hotel, Floors and layout.</p>
  {:else if !week}
    <h2 class="day-title">{formatDay(day, { weekday: "long", day: "numeric", month: "long", year: "numeric" })}</h2>
    <div class="timeline" style:--hours={HOURS.length}>
      <div class="hours" aria-hidden="true">
        <span></span>
        <div class="ticks">
          {#each HOURS as h (h)}<span>{String(h).padStart(2, "0")}:00</span>{/each}
        </div>
      </div>
      {#each suites as suite (suite.id)}
        <section class="lane" aria-label={suite.name}>
          <a class="lane-name" href={`/suites/booking?suite=${suite.id}&date=${day}`} title={`Book ${suite.name}`}>
            {suite.name}
          </a>
          <div class="track">
            <!-- Hour lines use the same grid as the header so they stay lined up. -->
            <div class="hour-lines" aria-hidden="true">
              {#each HOURS as h (h)}<span></span>{/each}
            </div>
            {#each forSuite(suite.id) as e (e.booking.id)}
              {const bar = timelineBar(e.booking, windowFrom, windowTo)}
              {#if bar.width > 0}
                <span class="pad" style:left="{bar.padLeft}%" style:width="{bar.padWidth}%" aria-hidden="true"></span>
                <a
                  class="bar"
                  data-status={e.booking.status}
                  href={`/suites/booking?id=${e.booking.id}`}
                  style:left="{bar.left}%"
                  style:width="{bar.width}%"
                  aria-label={label(e)}
                >
                  <span class="bar-title">{e.booking.title}</span>
                  <span class="bar-meta">{times(e)}, {e.booking.attendees} {layoutLabel[e.booking.layout].toLowerCase()}</span>
                </a>
              {/if}
            {/each}
          </div>
        </section>
      {/each}
    </div>
    <p class="muted legend">Hatched: setup and clear-down time, when the suite can't be booked.</p>

    {#if entries.length}
      <table class="list">
        <thead><tr><th>Time</th><th>Suite</th><th>What</th><th>Booked by</th><th>Numbers</th><th>Price</th><th>Status</th></tr></thead>
        <tbody>
          {#each entries as e (e.booking.id)}
            <tr>
              <td>{times(e)}</td>
              <td>{e.suite_name}</td>
              <td>
                <a href={`/suites/booking?id=${e.booking.id}`}>{e.booking.title}</a>
                {#if e.booking.event_id && e.event_name}
                  <br /><a class="muted" href={`/events/${e.booking.event_id}`}>{e.event_name}</a>
                {/if}
              </td>
              <td>{e.guest_name}</td>
              <td>{e.booking.attendees}, {layoutLabel[e.booking.layout].toLowerCase()}</td>
              <td>{formatMoney(e.price)}</td>
              <td><span class="status" data-status={e.booking.status}>{suiteStatusLabel[e.booking.status]}</span></td>
            </tr>
          {/each}
        </tbody>
      </table>
    {:else if loaded}
      <p class="muted">Nothing booked in the suites this day.</p>
    {/if}
  {:else}
    <div class="week-scroll">
      <table class="week">
        <thead>
          <tr>
            <th scope="col">Suite</th>
            {#each days as d (d)}
              <th scope="col" class:weekend={isWeekend(d)} class:today={d === today()}>
                <button type="button" class="day-link" onclick={() => { day = d; view = ["day"]; }}>
                  {formatDay(d, { weekday: "short", day: "numeric", month: "short" })}
                </button>
              </th>
            {/each}
          </tr>
        </thead>
        <tbody>
          {#each suites as suite (suite.id)}
            <tr>
              <th scope="row">{suite.name}</th>
              {#each days as d (d)}
                <td class:weekend={isWeekend(d)}>
                  {#each onDay(suite.id, d) as e (e.booking.id)}
                    <a class="chip" data-status={e.booking.status} href={`/suites/booking?id=${e.booking.id}`} aria-label={label(e)}>
                      <strong>{timePart(e.booking.start)}</strong> {e.booking.title}
                    </a>
                  {:else}
                    <a
                      class="free"
                      href={`/suites/booking?suite=${suite.id}&date=${d}`}
                      aria-label={`Book ${suite.name} on ${formatDay(d, { weekday: "long", day: "numeric", month: "long" })}`}
                    >Free</a>
                  {/each}
                </td>
              {/each}
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  {/if}
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

  .toolbar {
    align-items: flex-end;
    margin-bottom: calc(var(--ui-spacing) * 4);
  }

  .toolbar > :global([data-slot="field"]) {
    flex: 0 1 12rem;
  }

  .day-title {
    font-size: 1.1rem;
    margin-bottom: calc(var(--ui-spacing) * 3);
  }

  .timeline {
    border: 1px solid var(--ui-border);
    border-radius: 8px;
    background: var(--paper);
    overflow-x: auto;
    margin-bottom: calc(var(--ui-spacing) * 2);
  }

  .hours,
  .lane {
    display: grid;
    grid-template-columns: 10rem minmax(48rem, 1fr);
  }

  .ticks {
    display: grid;
    grid-template-columns: repeat(var(--hours), 1fr);
    font-size: 0.75rem;
    color: var(--ink-muted);
  }

  .ticks span {
    padding: calc(var(--ui-spacing) * 1);
    border-left: 1px solid var(--ui-border);
  }

  .lane {
    border-top: 1px solid var(--ui-border);
    min-height: 4rem;
  }

  .lane-name {
    display: flex;
    align-items: center;
    padding: 0 calc(var(--ui-spacing) * 3);
    font-weight: 700;
    color: var(--ink);
  }

  .track {
    position: relative;
  }

  .hour-lines {
    position: absolute;
    inset: 0;
    display: grid;
    grid-template-columns: repeat(var(--hours), 1fr);
    pointer-events: none;
  }

  .hour-lines span {
    border-left: 1px solid var(--ui-border);
  }

  .pad {
    position: absolute;
    top: 0.5rem;
    bottom: 0.5rem;
    border-radius: 6px;
    background: repeating-linear-gradient(135deg, var(--ground) 0 6px, transparent 6px 12px);
  }

  .bar {
    position: absolute;
    top: 0.5rem;
    bottom: 0.5rem;
    box-sizing: border-box;
    display: flex;
    flex-direction: column;
    justify-content: center;
    overflow: hidden;
    padding: 0 calc(var(--ui-spacing) * 2);
    border: 1.5px solid var(--st-confirmed-edge);
    border-radius: 6px;
    background: var(--st-confirmed-bg);
    color: var(--ink);
    text-decoration: none;
    white-space: nowrap;
  }

  .bar[data-status="provisional"],
  .chip[data-status="provisional"] {
    border-style: dashed;
    border-color: var(--st-provisional-edge);
    background: var(--st-provisional-bg);
  }

  .bar-title {
    font-weight: 700;
    font-size: 0.875rem;
    text-overflow: ellipsis;
    overflow: hidden;
  }

  .bar-meta {
    font-size: 0.75rem;
    text-overflow: ellipsis;
    overflow: hidden;
  }

  .legend {
    margin: 0 0 calc(var(--ui-spacing) * 5);
    font-size: 0.875rem;
  }

  .week-scroll {
    overflow-x: auto;
  }

  .week {
    width: 100%;
    border-collapse: collapse;
    background: var(--paper);
    min-width: 56rem;
  }

  .week th,
  .week td {
    border: 1px solid var(--ui-border);
    padding: calc(var(--ui-spacing) * 2);
    vertical-align: top;
    text-align: left;
    width: calc(100% / 8);
  }

  .week .weekend {
    background: var(--ground);
  }

  .week th.today {
    box-shadow: inset 0 -3px 0 var(--heather);
  }

  .day-link {
    all: unset;
    cursor: pointer;
    font-weight: 700;
  }

  .day-link:focus-visible {
    outline: 2px solid var(--heather);
    outline-offset: 2px;
  }

  .chip {
    display: block;
    margin-bottom: calc(var(--ui-spacing) * 1);
    padding: calc(var(--ui-spacing) * 1) calc(var(--ui-spacing) * 2);
    border: 1.5px solid var(--st-confirmed-edge);
    border-radius: 6px;
    background: var(--st-confirmed-bg);
    color: var(--ink);
    font-size: 0.85rem;
    text-decoration: none;
  }

  .free {
    font-size: 0.85rem;
    color: var(--ink-muted);
  }
</style>
