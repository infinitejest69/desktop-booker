<script lang="ts">
  import { Button } from "@ui-sdk/svelte/button";
  import { toast } from "@ui-sdk/svelte/toast";
  import { goto } from "$app/navigation";
  import { page } from "$app/state";
  import { api, errorMessage } from "#lib/api.ts";
  import type { EventDetail } from "#lib/bindings/EventDetail.ts";
  import { statusLabel } from "#lib/booking/status.ts";
  import { daysBetween, formatDay } from "#lib/dates.ts";
  import { formatMoney } from "#lib/format.ts";
  import FolioPanel from "#lib/invoices/FolioPanel.svelte";
  import Breadcrumbs from "#lib/nav/Breadcrumbs.svelte";
  import { eventKindLabel, layoutLabel, pricingLabel, suiteStatusLabel, timePart } from "#lib/suites/suites.ts";

  let detail = $state<EventDetail | null>(null);

  $effect(() => {
    const id = page.params.id;
    if (!id) return;
    api
      .getEvent(id)
      .then((d) => (detail = d))
      .catch((error) => toast.error("Couldn't load the event", { description: errorMessage(error) }));
  });

  const long: Intl.DateTimeFormatOptions = { weekday: "long", day: "numeric", month: "long", year: "numeric" };
  const short: Intl.DateTimeFormatOptions = { weekday: "short", day: "numeric", month: "short" };

  /** Free text split into its non-empty lines. */
  function lines(text: string): string[] {
    return text
      .split("\n")
      .map((l) => l.trim())
      .filter(Boolean);
  }

  const plural = (n: number, one: string, many = `${one}s`) => `${n} ${n === 1 ? one : many}`;
</script>

<div class="page wide">
  {#if detail}
    {@const e = detail.event}
    {@const block = e.room_block}
    <Breadcrumbs items={[{ label: "Events", href: "/events" }, { label: e.name }]} />
    <div class="heading">
      <div>
        <h1>{e.name}</h1>
        <p class="muted" style="margin: 0">
          {eventKindLabel[e.kind]},
          {e.start_date === e.end_date
            ? formatDay(e.start_date, long)
            : `${formatDay(e.start_date, short)} to ${formatDay(e.end_date, long)}`}
        </p>
      </div>
      <div class="row">
        <Button onclick={() => goto(`/suites/booking?event=${e.id}&date=${e.start_date}`)}>Book a suite</Button>
        <Button data-variant="solid" onclick={() => goto(`/events/${e.id}/edit`)}>Edit event</Button>
      </div>
    </div>

    <section class="glance" aria-label="Event at a glance">
      <p>
        <strong>{formatMoney(detail.total)}</strong> in all:
        {formatMoney(detail.suites_value)} for suites and {formatMoney(detail.rooms_value)} for bedrooms.
        <strong>{detail.catering.adults}</strong> {detail.catering.adults === 1 ? "adult" : "adults"} and
        <strong>{detail.catering.children}</strong> {detail.catering.children === 1 ? "child" : "children"} to feed
        over {plural(detail.catering.sittings, "sitting")}.
      </p>
    </section>

    <div class="grid">
      <section class="panel" aria-labelledby="organiser-heading">
        <h2 id="organiser-heading">Organiser</h2>
        <p class="who">{detail.organiser.first_name} {detail.organiser.last_name}</p>
        <p class="muted">{detail.organiser.email || "No email"}</p>
        <p class="muted">{detail.organiser.phone || "No phone"}</p>
        {#if e.notes}
          <h3>Notes</h3>
          <p class="pre">{e.notes}</p>
        {/if}
      </section>

      <section class="panel" aria-labelledby="running-heading">
        <h2 id="running-heading">Running order</h2>
        {#if e.running_order}
          <p class="pre">{e.running_order}</p>
        {:else}
          <p class="muted">Not written yet.</p>
        {/if}
      </section>
    </div>

    <section class="panel block" aria-labelledby="suites-heading">
      <h2 id="suites-heading">Suites</h2>
      {#if detail.suites.length}
        <table class="list">
          <thead><tr><th>When</th><th>Suite</th><th>What</th><th>Layout</th><th>Pricing</th><th>Price</th><th>Status</th></tr></thead>
          <tbody>
            {#each detail.suites as s (s.booking.id)}
              <tr>
                <td>{formatDay(s.booking.start.slice(0, 10), short)}, {timePart(s.booking.start)} to {timePart(s.booking.end)}</td>
                <td>{s.suite_name}</td>
                <td><a href={`/suites/booking?id=${s.booking.id}`}>{s.booking.title}</a></td>
                <td>{s.booking.attendees}, {layoutLabel[s.booking.layout].toLowerCase()}</td>
                <td>{pricingLabel[s.booking.pricing.kind]}</td>
                <td>{formatMoney(s.price)}</td>
                <td><span class="status" data-status={s.booking.status}>{suiteStatusLabel[s.booking.status]}</span></td>
              </tr>
            {/each}
          </tbody>
        </table>
      {:else}
        <p class="muted">No suites booked for this event yet.</p>
      {/if}
    </section>

    <section class="panel block" aria-labelledby="catering-heading">
      <h2 id="catering-heading">Catering</h2>
      {#if e.sittings.length}
        <div class="sittings">
          {#each e.sittings as s, i (i)}
            <div class="sitting">
              <h3>{s.name}</h3>
              <p class="muted">{formatDay(s.date, short)} at {timePart(s.time)}</p>
              <p><strong>{plural(s.adults, "adult")}</strong>{s.children ? `, ${plural(s.children, "child", "children")}` : ""}</p>
              {#if lines(s.dietary).length}
                <ul class="dietary" aria-label={`Dietary needs for ${s.name}`}>
                  {#each lines(s.dietary) as line, j (j)}<li>{line}</li>{/each}
                </ul>
              {:else}
                <p class="muted">No dietary needs noted.</p>
              {/if}
            </div>
          {/each}
        </div>
      {:else}
        <p class="muted">No sittings yet.</p>
      {/if}
    </section>

    <section class="panel block" aria-labelledby="rooms-heading">
      <h2 id="rooms-heading">Bedrooms</h2>
      {#if block}
        <p>
          Holding <strong>{plural(block.rooms, "room")}</strong>,
          {formatDay(block.stay.start, short)} to {formatDay(block.stay.end, short)}
          ({plural(daysBetween(block.stay.start, block.stay.end), "night")}) at {formatMoney(block.rate)} a night.
          <strong>{detail.held_rooms}</strong> still held until
          <strong>{formatDay(block.release_date, long)}</strong>; confirm a room when a guest takes it up.
        </p>
      {/if}
      {#if detail.rooms.length}
        <table class="list">
          <thead><tr><th>Booking</th><th>Guest</th><th>Room</th><th>Stay</th><th>Value</th><th>Status</th></tr></thead>
          <tbody>
            {#each detail.rooms as r (r.booking_id)}
              <tr>
                <td><a href={`/bookings/${r.booking_id}`}>{r.reference}</a></td>
                <td>{r.guest_name}</td>
                <td>{r.rooms.join(", ")}</td>
                <td>{formatDay(r.stay.start, short)} to {formatDay(r.stay.end, short)}</td>
                <td>{formatMoney(r.total)}</td>
                <td>
                  <span class="status" data-status={r.status}>
                    {r.status === "provisional" ? "Held" : statusLabel[r.status]}
                  </span>
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      {:else if !block}
        <p class="muted">No rooms held for guests.</p>
      {/if}
    </section>
    <FolioPanel owner={{ kind: "event", id: e.id }} />
  {:else}
    <p class="muted">Loading…</p>
  {/if}
</div>

<style>
  .wide {
    max-width: 80rem;
  }

  .heading {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: calc(var(--ui-spacing) * 3);
    flex-wrap: wrap;
  }

  .heading h1 {
    margin: 0;
  }

  .glance p {
    margin: calc(var(--ui-spacing) * 4) 0 calc(var(--ui-spacing) * 5);
    font-size: 1.1rem;
    max-width: 72ch;
  }

  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(20rem, 1fr));
    gap: calc(var(--ui-spacing) * 4);
    margin-bottom: calc(var(--ui-spacing) * 4);
    align-items: start;
  }

  .block {
    margin-bottom: calc(var(--ui-spacing) * 4);
  }

  .panel p {
    margin: 0 0 calc(var(--ui-spacing) * 1);
  }

  .who {
    font-weight: 700;
  }

  .pre {
    white-space: pre-line;
  }

  h3 {
    font-size: 0.95rem;
    margin: calc(var(--ui-spacing) * 3) 0 calc(var(--ui-spacing) * 1);
  }

  .sittings {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(16rem, 1fr));
    gap: calc(var(--ui-spacing) * 4);
  }

  .sitting h3 {
    margin-top: 0;
  }

  .dietary {
    margin: calc(var(--ui-spacing) * 1) 0 0;
    padding-left: 1.2rem;
  }
</style>
