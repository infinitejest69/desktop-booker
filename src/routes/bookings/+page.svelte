<script lang="ts">
  import { Button } from "@ui-sdk/svelte/button";
  import { Field, FieldLabel } from "@ui-sdk/svelte/field";
  import { Input } from "@ui-sdk/svelte/input";
  import { toast } from "@ui-sdk/svelte/toast";
  import { goto } from "$app/navigation";
  import { api, errorMessage } from "#lib/api.ts";
  import type { BookingListItem } from "#lib/bindings/BookingListItem.ts";
  import { boardsSummary } from "#lib/booking/board.ts";
  import { statusLabel } from "#lib/booking/status.ts";
  import { daysBetween, formatDay } from "#lib/dates.ts";
  import BookingViews from "#lib/booking/BookingViews.svelte";

  let query = $state("");
  let results = $state<BookingListItem[]>([]);
  let loaded = $state(false);

  $effect(() => {
    const q = query;
    api
      .searchBookings(q)
      .then((r) => {
        if (q === query) results = r;
      })
      .catch((error) => toast.error("Couldn't search bookings", { description: errorMessage(error) }))
      .finally(() => (loaded = true));
  });
</script>

<div class="page">
  <h1>Bookings</h1>
  <p class="page-intro">Search by reference, guest name or email. With no search, upcoming and current stays are shown.</p>
  <div class="row" style="margin-bottom: 1rem">
    <BookingViews current="list" />
    <Field>
      <FieldLabel>Search</FieldLabel>
      <Input type="search" bind:value={query} placeholder="2026-0012, Hughes, jo@example.com" />
    </Field>
    <Button data-variant="solid" onclick={() => goto("/bookings/new")}>New booking</Button>
  </div>
  {#if results.length}
    <table class="list">
      <thead>
        <tr><th>Reference</th><th>Guest</th><th>Arrival</th><th>Nights</th><th>Rooms</th><th>Board</th><th>Status</th></tr>
      </thead>
      <tbody>
        {#each results as b (b.id)}
          <tr>
            <td><a href={`/bookings/${b.id}`}>{b.reference}</a></td>
            <td>{b.guest_name}</td>
            <td>{formatDay(b.arrival, { weekday: "short", day: "numeric", month: "short", year: "numeric" })}</td>
            <td>{daysBetween(b.arrival, b.departure)}</td>
            <td>{b.rooms.join(", ")}</td>
            <td>{boardsSummary(b.boards)}</td>
            <td><span class="status" data-status={b.status}>{statusLabel[b.status]}</span></td>
          </tr>
        {/each}
      </tbody>
    </table>
  {:else if loaded}
    <p class="muted">{query ? "No bookings match." : "No upcoming bookings."}</p>
  {/if}
</div>
