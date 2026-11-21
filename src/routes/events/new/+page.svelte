<script lang="ts">
  import { goto } from "$app/navigation";
  import EventForm from "#lib/events/EventForm.svelte";
  import Breadcrumbs from "#lib/nav/Breadcrumbs.svelte";
</script>

<div class="page">
  <Breadcrumbs items={[{ label: "Events", href: "/events" }, { label: "New event" }]} />
  <h1>New event</h1>
  <EventForm onSaved={(saved) => goto(`/events/${saved.event.id}`, { replace: true })} />
</div>
