<script lang="ts">
  import { Button } from "@ui-sdk/svelte/button";
  import { toast } from "@ui-sdk/svelte/toast";
  import { page } from "$app/state";
  import BookingEditor from "#lib/booking/BookingEditor.svelte";
  import BookingSummary from "#lib/booking/BookingSummary.svelte";
  import FolioPanel from "#lib/invoices/FolioPanel.svelte";
  import Breadcrumbs from "#lib/nav/Breadcrumbs.svelte";
  import { api, errorMessage } from "#lib/api.ts";
  import type { BookingDetail } from "#lib/bindings/BookingDetail.ts";
  import type { Space } from "#lib/bindings/Space.ts";

  let detail = $state<BookingDetail | null>(null);
  let rooms = $state<Space[]>([]);
  let editing = $state(page.url.searchParams.get("edit") === "1");
  let version = $state(0);

  const roomNames = $derived(new Map(rooms.map((r) => [r.id, r.name])));
  const chartHref = $derived(
    detail ? `/chart?from=${detail.booking.rooms[0]?.stay.start ?? ""}&booking=${detail.booking.id}` : "/chart",
  );

  $effect(() => {
    const id = page.params.id;
    if (!id) return;
    Promise.all([api.getBooking(id), api.listSpaces()])
      .then(([d, s]) => {
        detail = d;
        rooms = s;
      })
      .catch((error) => toast.error("Couldn't load the booking", { description: errorMessage(error) }));
  });
</script>

<div class="page">
  {#if detail}
    <Breadcrumbs
      items={[
        { label: "Bookings", href: "/bookings" },
        { label: detail.booking.reference },
      ]}
    />
    <div class="heading">
      <div>
        <h1>{detail.guest.first_name} {detail.guest.last_name}</h1>
        <p class="muted" style="margin: 0">Booking {detail.booking.reference}</p>
      </div>
      <div class="row">
        <a class="link-button" href={chartHref}>Show on chart</a>
        <Button data-variant={editing ? undefined : "solid"} onclick={() => (editing = !editing)}>
          {editing ? "Close editor" : "Edit booking"}
        </Button>
      </div>
    </div>

    {#if editing}
      {#key version}
        <BookingEditor
          {detail}
          onSaved={(saved) => {
            detail = saved;
            version++;
            editing = false;
          }}
        />
      {/key}
    {:else}
      <section class="panel" style="max-width: 52rem" aria-label="Summary">
        <BookingSummary {detail} {roomNames} onChange={(d) => (detail = d)} />
      </section>
      {#key detail}
        <div style="max-width: 52rem"><FolioPanel owner={{ kind: "booking", id: detail.booking.id }} /></div>
      {/key}
    {/if}
  {:else}
    <p class="muted">Loading…</p>
  {/if}
</div>

<style>
  .heading {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: calc(var(--ui-spacing) * 3);
    flex-wrap: wrap;
    margin-bottom: calc(var(--ui-spacing) * 5);
  }

  .heading h1 {
    margin: 0;
  }
</style>
