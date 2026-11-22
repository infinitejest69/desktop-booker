<script lang="ts">
  import { Button } from "@ui-sdk/svelte/button";
  import { Switch, SwitchThumb } from "@ui-sdk/svelte/switch";
  import { toast } from "@ui-sdk/svelte/toast";
  import { goto } from "$app/navigation";
  import { api, errorMessage } from "#lib/api.ts";
  import ViewSwitch from "#lib/nav/ViewSwitch.svelte";
  import { functionViews } from "#lib/nav/views.ts";
  import type { EventListItem } from "#lib/bindings/EventListItem.ts";
  import { formatDay, today } from "#lib/dates.ts";
  import { useSession } from "#lib/session/session.svelte.ts";
  import { eventKindLabel } from "#lib/suites/suites.ts";

  const session = useSession();
  let events = $state<EventListItem[]>([]);
  let showPast = $state(false);
  let loaded = $state(false);

  async function load(past: boolean) {
    try {
      events = await api.listEvents(past ? "2000-01-01" : today());
    } catch (error) {
      toast.error("Couldn't load events", { description: errorMessage(error) });
    } finally {
      loaded = true;
    }
  }

  $effect(() => {
    void load(showPast);
  });

  async function loadDemo() {
    try {
      if (await api.seedDemoData()) toast.success("Demo events added");
      else toast.info("Nothing to add: events already exist, or the demo suites are missing");
      await load(showPast);
    } catch (error) {
      toast.error("Couldn't add demo events", { description: errorMessage(error) });
    }
  }

  function dates(e: EventListItem): string {
    const opts: Intl.DateTimeFormatOptions = { weekday: "short", day: "numeric", month: "short", year: "numeric" };
    return e.start_date === e.end_date
      ? formatDay(e.start_date, opts)
      : `${formatDay(e.start_date, { day: "numeric", month: "short" })} to ${formatDay(e.end_date, opts)}`;
  }
</script>

<div class="page wide">
  <div class="heading">
    <h1>Events</h1>
    <Button data-variant="solid" onclick={() => goto("/events/new")}>New event</Button>
  </div>
  <p class="page-intro">Weddings, conferences and dinners: their suites, catering numbers and rooms held for guests.</p>

  <div class="switch-row">
    <ViewSwitch views={functionViews} current="Events" label="Suites or events" />
    <label data-slot="label" class="past">
      Include past events
      <Switch bind:checked={showPast}><SwitchThumb /></Switch>
    </label>
  </div>

  {#if events.length}
    <table class="list">
      <thead>
        <tr><th>When</th><th>Event</th><th>Type</th><th>Organiser</th><th>Suites</th><th>Rooms held</th></tr>
      </thead>
      <tbody>
        {#each events as e (e.id)}
          <tr>
            <td>{dates(e)}</td>
            <td><a href={`/events/${e.id}`}>{e.name}</a></td>
            <td>{eventKindLabel[e.kind]}</td>
            <td>{e.organiser_name}</td>
            <td>{e.suite_bookings}</td>
            <td>
              {#if e.release_date}
                {e.held_rooms}
                {#if e.held_rooms}
                  <span class="muted">until {formatDay(e.release_date, { day: "numeric", month: "short" })}</span>
                {/if}
              {:else}
                <span class="muted">None</span>
              {/if}
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
  {:else if loaded}
    <div class="panel stack" style="max-width: 36rem">
      <p style="margin: 0">{showPast ? "No events yet." : "No events coming up."}</p>
      {#if session.can("manage_setup")}
        <p class="muted" style="margin: 0">
          Add a demo wedding, conference and club dinner to the demo hotel to see how events work.
        </p>
        <div><Button onclick={loadDemo}>Load demo events</Button></div>
      {/if}
    </div>
  {/if}
</div>

<style>
  .wide {
    max-width: 80rem;
  }

  .heading {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: calc(var(--ui-spacing) * 3);
  }

  .switch-row {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: calc(var(--ui-spacing) * 6);
    margin-bottom: calc(var(--ui-spacing) * 4);
  }

  .past {
    display: flex;
    align-items: center;
    gap: calc(var(--ui-spacing) * 3);
  }
</style>
