<script lang="ts">
  import { Button } from "@ui-sdk/svelte/button";
  import { Field, FieldLabel } from "@ui-sdk/svelte/field";
  import { Input } from "@ui-sdk/svelte/input";
  import { toast } from "@ui-sdk/svelte/toast";
  import { api, errorMessage } from "#lib/api.ts";
  import type { ChartEntry } from "#lib/bindings/ChartEntry.ts";
  import type { Space } from "#lib/bindings/Space.ts";
  import { boardLabel } from "#lib/booking/board.ts";
  import { todaySummary } from "#lib/dashboard/today.ts";
  import { addDays, formatDay, today } from "#lib/dates.ts";

  let day = $state(today());
  let rooms = $state<Space[]>([]);
  let entries = $state<ChartEntry[]>([]);

  const summary = $derived(todaySummary(entries, rooms, day));
  const roomName = $derived(new Map(rooms.map((r) => [r.id, r.name])));
  /** Everyone sleeping here that night, checked in or not yet. */
  const staying = $derived(
    entries
      .filter((e) => e.stay.start <= day && day < e.stay.end)
      .sort((a, b) =>
        (roomName.get(a.space_id) ?? "").localeCompare(roomName.get(b.space_id) ?? "", undefined, { numeric: true }),
      ),
  );

  $effect(() => {
    const d = day;
    Promise.all([api.listSpaces(), api.chart(addDays(d, -1), addDays(d, 2))])
      .then(([s, e]) => {
        rooms = s.filter((x) => x.kind === "room");
        entries = e;
      })
      .catch((error) => toast.error("Couldn't load the lists", { description: errorMessage(error) }));
  });

  const guests = (e: ChartEntry) => `${e.adults}${e.children ? ` + ${e.children}` : ""}`;
  const short: Intl.DateTimeFormatOptions = { day: "numeric", month: "short" };
</script>

{#snippet list(title: string, rows: ChartEntry[], empty: string)}
  <section class="list-block" aria-label={title}>
    <h2>{title} <span class="muted">({rows.length})</span></h2>
    {#if rows.length}
      <table class="list">
        <thead><tr><th>Room</th><th>Guest</th><th>Booking</th><th>Guests</th><th>Board</th><th>Stay</th></tr></thead>
        <tbody>
          {#each rows as e (e.booking_room_id)}
            <tr>
              <td><strong>{roomName.get(e.space_id)}</strong></td>
              <td>{e.guest_name}</td>
              <td>{e.reference}</td>
              <td>{guests(e)}</td>
              <td>{boardLabel(e.board)}</td>
              <td>{formatDay(e.stay.start, short)} to {formatDay(e.stay.end, short)}</td>
            </tr>
          {/each}
        </tbody>
      </table>
    {:else}
      <p class="muted">{empty}</p>
    {/if}
  </section>
{/snippet}

<div class="page wide">
  <div class="row no-print toolbar">
    <Field>
      <FieldLabel>Day</FieldLabel>
      <Input type="date" value={day} onValueChange={(v) => v && (day = v)} />
    </Field>
    <Button data-variant="solid" onclick={() => window.print()}>Print</Button>
  </div>
  <h1>Daily lists: {formatDay(day, { weekday: "long", day: "numeric", month: "long", year: "numeric" })}</h1>
  {@render list("Arrivals", summary.arrivals, "No arrivals.")}
  {@render list("Departures", summary.departures, "No departures.")}
  {@render list("Staying tonight", staying, "Nobody staying tonight.")}
</div>

<style>
  .wide {
    max-width: 72rem;
  }

  .toolbar {
    align-items: flex-end;
    justify-content: space-between;
  }

  .toolbar > :global([data-slot="field"]) {
    flex: 0 1 12rem;
  }

  .list-block {
    margin-bottom: calc(var(--ui-spacing) * 6);
    break-inside: avoid;
  }

  h2 {
    font-size: 1.1rem;
  }
</style>
