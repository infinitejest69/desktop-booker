<script lang="ts">
  import { toast } from "@ui-sdk/svelte/toast";
  import { goto } from "$app/navigation";
  import { page } from "$app/state";
  import { api, errorMessage } from "#lib/api.ts";
  import type { EventDetail } from "#lib/bindings/EventDetail.ts";
  import EventForm from "#lib/events/EventForm.svelte";
  import Breadcrumbs from "#lib/nav/Breadcrumbs.svelte";

  let detail = $state<EventDetail | null>(null);

  $effect(() => {
    const id = page.params.id;
    if (!id) return;
    api
      .getEvent(id)
      .then((d) => (detail = d))
      .catch((error) => toast.error("Couldn't load the event", { description: errorMessage(error) }));
  });
</script>

<div class="page">
  {#if detail}
    <Breadcrumbs
      items={[
        { label: "Events", href: "/events" },
        { label: detail.event.name, href: `/events/${detail.event.id}` },
        { label: "Edit" },
      ]}
    />
    <h1>Edit {detail.event.name}</h1>
    <EventForm {detail} onSaved={(saved) => goto(`/events/${saved.event.id}`)} />
  {:else}
    <p class="muted">Loading…</p>
  {/if}
</div>
