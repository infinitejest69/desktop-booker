<script lang="ts">
  import { toast } from "@ui-sdk/svelte/toast";
  import { goto } from "$app/navigation";
  import { page } from "$app/state";
  import { api, errorMessage } from "#lib/api.ts";
  import type { SuiteBookingEntry } from "#lib/bindings/SuiteBookingEntry.ts";
  import Breadcrumbs from "#lib/nav/Breadcrumbs.svelte";
  import SuiteBookingForm from "#lib/suites/SuiteBookingForm.svelte";
  import { datePart } from "#lib/suites/suites.ts";

  const params = page.url.searchParams;
  const id = params.get("id");
  const eventId = params.get("event");

  let entry = $state<SuiteBookingEntry | null>(null);
  let loaded = $state(id === null);

  if (id) {
    api
      .getSuiteBooking(id)
      .then((e) => (entry = e))
      .catch((error) => toast.error("Couldn't load the suite booking", { description: errorMessage(error) }))
      .finally(() => (loaded = true));
  }

  /** Back to the event it belongs to, or the suites on that day. */
  function done(saved: SuiteBookingEntry) {
    const event = saved.booking.event_id ?? eventId;
    void goto(event ? `/events/${event}` : `/suites?date=${datePart(saved.booking.start)}`);
  }
</script>

<div class="page">
  <Breadcrumbs
    items={[
      { label: "Function suites", href: "/suites" },
      { label: id ? (entry?.booking.title ?? "Suite booking") : "Book a suite" },
    ]}
  />
  <h1>{id ? (entry ? `${entry.suite_name}: ${entry.booking.title}` : "Suite booking") : "Book a suite"}</h1>
  {#if entry?.booking.status === "cancelled"}
    <p><span class="status" data-status="cancelled">Cancelled</span></p>
  {/if}
  {#if loaded}
    <SuiteBookingForm
      {entry}
      initialSuite={params.get("suite")}
      initialDate={params.get("date")}
      initialEvent={eventId}
      onSaved={done}
    />
  {:else}
    <p class="muted">Loading…</p>
  {/if}
</div>
