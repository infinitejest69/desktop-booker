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
  import type { EmailKind } from "#lib/bindings/EmailKind.ts";
  import type { EmailLogEntry } from "#lib/bindings/EmailLogEntry.ts";

  let detail = $state<BookingDetail | null>(null);
  let rooms = $state<Space[]>([]);
  let editing = $state(page.url.searchParams.get("edit") === "1");
  let version = $state(0);
  let emails = $state<EmailLogEntry[]>([]);
  let sending = $state<EmailKind | null>(null);

  async function loadEmails(id: string) {
    emails = await api.listEmailLog(id).catch(() => []);
  }

  async function sendEmail(kind: EmailKind) {
    if (!detail) return;
    sending = kind;
    try {
      await api.sendBookingEmail(detail.booking.id, kind);
      toast.success(`Emailed ${detail.guest.email}`);
    } catch (error) {
      toast.error("The email wasn't sent", { description: errorMessage(error) });
    } finally {
      sending = null;
      await loadEmails(detail.booking.id);
    }
  }

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
        void loadEmails(d.booking.id);
      })
      .catch((error) => toast.error("Couldn't load the booking", { description: errorMessage(error) }));
  });
</script>

<div class="page">
  {#if detail}
    <Breadcrumbs
      items={[
        { label: "Bookings", href: "/chart" },
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
      <section class="panel stack email" style="max-width: 52rem" aria-labelledby="email-heading">
        <h2 id="email-heading">Email the guest</h2>
        {#if detail.guest.email}
          <p class="muted" style="margin: 0">Sends to {detail.guest.email} from the hotel's email account.</p>
          <div class="row">
            {#if detail.booking.status === "cancelled"}
              <Button onclick={() => sendEmail("cancellation")} disabled={sending !== null}>Send cancellation</Button>
            {:else}
              <Button onclick={() => sendEmail("confirmation")} disabled={sending !== null}>Send confirmation</Button>
              <Button onclick={() => sendEmail("amendment")} disabled={sending !== null}>Send changes</Button>
            {/if}
          </div>
        {:else}
          <p class="muted" style="margin: 0">No email address for this guest. Edit the booking to add one.</p>
        {/if}
        {#if emails.length}
          <ul class="sent">
            {#each emails as e, i (i)}
              <li>
                {new Date(e.at).toLocaleString("en-GB", { day: "numeric", month: "short", hour: "2-digit", minute: "2-digit" })}:
                {e.subject}
                <span class="muted">{e.sent ? `sent by ${e.sent_by}` : `not sent (${e.error})`}</span>
              </li>
            {/each}
          </ul>
        {/if}
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

  .email {
    margin-top: calc(var(--ui-spacing) * 4);
  }

  .email h2 {
    margin: 0;
  }

  .sent {
    margin: 0;
    padding-left: 1.2rem;
  }
</style>
