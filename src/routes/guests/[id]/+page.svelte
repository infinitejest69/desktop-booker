<script lang="ts">
  import { confirm } from "@ui-sdk/svelte/alert-dialog";
  import { Button } from "@ui-sdk/svelte/button";
  import { Field, FieldLabel } from "@ui-sdk/svelte/field";
  import { Input } from "@ui-sdk/svelte/input";
  import type { Guest } from "#lib/bindings/Guest.ts";
  import { toast } from "@ui-sdk/svelte/toast";
  import { page } from "$app/state";
  import { api, errorMessage } from "#lib/api.ts";
  import type { GuestProfile } from "#lib/bindings/GuestProfile.ts";
  import { statusLabel } from "#lib/booking/status.ts";
  import { daysBetween, formatDay } from "#lib/dates.ts";
  import { formatMoney } from "#lib/format.ts";
  import Breadcrumbs from "#lib/nav/Breadcrumbs.svelte";
  import { useSession } from "#lib/session/session.svelte.ts";

  const session = useSession();
  let profile = $state<GuestProfile | null>(null);
  let dupQuery = $state("");
  let dupResults = $state<Guest[]>([]);

  async function findDuplicates() {
    const q = dupQuery.trim();
    const all = q.length < 2 ? [] : await api.searchGuests(q).catch(() => []);
    dupResults = all.filter((g) => g.id !== profile?.guest.id);
  }

  async function merge(dup: Guest) {
    if (!profile) return;
    const dupName = `${dup.first_name} ${dup.last_name}`.trim();
    const yes = await confirm({
      title: `Merge ${dupName} into ${name}?`,
      description: `${dupName}'s bookings, suite hire and events move to ${name}. Details ${name} hasn't got are filled in from ${dupName}, and both sets of notes are kept. The duplicate record is then removed. This can't be undone.`,
      confirmLabel: "Merge",
      cancelLabel: "Cancel",
    });
    if (!yes) return;
    try {
      await api.mergeGuests(profile.guest.id, dup.id);
      toast.success(`Merged ${dupName} into ${name}`);
      dupQuery = "";
      dupResults = [];
      await load(profile.guest.id);
    } catch (error) {
      toast.error("Couldn't merge the guests", { description: errorMessage(error) });
    }
  }

  async function load(id: string) {
    try {
      profile = await api.guestProfile(id);
    } catch (error) {
      toast.error("Couldn't load the guest", { description: errorMessage(error) });
    }
  }

  $effect(() => {
    const id = page.params.id;
    if (id) void load(id);
  });

  const name = $derived(profile ? `${profile.guest.first_name} ${profile.guest.last_name}`.trim() : "");
  const nights = $derived(
    (profile?.stays ?? [])
      .filter((s) => s.status !== "cancelled" && s.status !== "no_show")
      .reduce((n, s) => n + daysBetween(s.arrival, s.departure), 0),
  );

  /** Copies everything held about the guest, for a subject access request. */
  async function copyData() {
    if (!profile) return;
    try {
      await navigator.clipboard.writeText(JSON.stringify(profile, null, 2));
      toast.success("Copied the guest's data", { description: "Paste it into an email or document." });
    } catch {
      toast.error("Couldn't copy to the clipboard");
    }
  }

  async function erase() {
    if (!profile) return;
    const yes = await confirm({
      title: `Erase ${name}'s details?`,
      description:
        'Their name, email, phone, address and notes are removed for good. Bookings and invoices stay, marked "Erased guest". This can\'t be undone.',
      confirmLabel: "Erase details",
      cancelLabel: "Keep them",
    });
    if (!yes) return;
    try {
      await api.eraseGuest(profile.guest.id);
      toast.success("Guest details erased");
      await load(profile.guest.id);
    } catch (error) {
      toast.error("Couldn't erase the guest", { description: errorMessage(error) });
    }
  }
</script>

<div class="page">
  {#if profile}
    <Breadcrumbs items={[{ label: "Guests", href: "/guests" }, { label: name }]} />
    <div class="heading">
      <h1>{name}</h1>
      <div class="row">
        <Button onclick={copyData}>Copy their data</Button>
        {#if session.can("manage_setup")}
          <Button data-variant="destructive" onclick={erase}>Erase details</Button>
        {/if}
      </div>
    </div>

    <section class="glance" aria-label="Guest at a glance">
      <p>
        <strong>{profile.stays.length}</strong> booking{profile.stays.length === 1 ? "" : "s"},
        <strong>{nights}</strong> night{nights === 1 ? "" : "s"},
        <strong>{formatMoney(profile.total_spend)}</strong> spent.
      </p>
    </section>

    <div class="grid">
      <section class="panel" aria-labelledby="contact-heading">
        <h2 id="contact-heading">Contact</h2>
        <p>{profile.guest.email || "No email"}</p>
        <p>{profile.guest.phone || "No phone"}</p>
        {#if profile.guest.address}<p class="pre">{profile.guest.address}</p>{/if}
      </section>
      <section class="panel" aria-labelledby="notes-heading">
        <h2 id="notes-heading">Notes and preferences</h2>
        <p class="pre">{profile.guest.notes || "None yet."}</p>
      </section>
    </div>

    <section class="panel" aria-labelledby="stays-heading">
      <h2 id="stays-heading">Stays</h2>
      {#if profile.stays.length}
        <table class="list">
          <thead><tr><th>Booking</th><th>Arrival</th><th>Nights</th><th>Rooms</th><th>Status</th></tr></thead>
          <tbody>
            {#each profile.stays as s (s.id)}
              <tr>
                <td><a href={`/bookings/${s.id}`}>{s.reference}</a></td>
                <td>{formatDay(s.arrival, { day: "numeric", month: "short", year: "numeric" })}</td>
                <td>{daysBetween(s.arrival, s.departure)}</td>
                <td>{s.rooms.join(", ")}</td>
                <td><span class="status" data-status={s.status}>{statusLabel[s.status]}</span></td>
              </tr>
            {/each}
          </tbody>
        </table>
      {:else}
        <p class="muted">No stays yet.</p>
      {/if}
    </section>
    {#if session.can("manage_setup")}
      <section class="panel stack merge" aria-labelledby="merge-heading">
        <h2 id="merge-heading">Merge a duplicate</h2>
        <p class="muted" style="margin: 0">Find another record for the same person to combine it into this one.</p>
        <Field>
          <FieldLabel>Find the duplicate</FieldLabel>
          <Input type="search" bind:value={dupQuery} oninput={findDuplicates} placeholder="Name, email or phone" />
        </Field>
        {#if dupResults.length}
          <ul class="dups">
            {#each dupResults as d (d.id)}
              <li>
                <span>{`${d.first_name} ${d.last_name}`.trim()}{d.email ? ` (${d.email})` : ""}</span>
                <Button onclick={() => merge(d)}>Merge into {name}</Button>
              </li>
            {/each}
          </ul>
        {/if}
      </section>
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
  }

  .glance p {
    font-size: 1.1rem;
  }

  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(18rem, 1fr));
    gap: calc(var(--ui-spacing) * 4);
    margin-bottom: calc(var(--ui-spacing) * 4);
  }

  .panel p {
    margin: 0 0 calc(var(--ui-spacing) * 1);
  }

  .pre {
    white-space: pre-line;
  }

  .merge {
    margin-top: calc(var(--ui-spacing) * 4);
  }

  .dups {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: calc(var(--ui-spacing) * 2);
  }

  .dups li {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: calc(var(--ui-spacing) * 3);
  }
</style>
