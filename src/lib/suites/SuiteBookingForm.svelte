<script lang="ts">
  import { onMount } from "svelte";
  import { confirm } from "@ui-sdk/svelte/alert-dialog";
  import { Button } from "@ui-sdk/svelte/button";
  import { Field, FieldDescription, FieldLabel } from "@ui-sdk/svelte/field";
  import { Input } from "@ui-sdk/svelte/input";
  import {
    Select,
    SelectContent,
    SelectItem,
    SelectItemIndicator,
    SelectTrigger,
    SelectValue,
  } from "@ui-sdk/svelte/select";
  import { Textarea } from "@ui-sdk/svelte/textarea";
  import { toast } from "@ui-sdk/svelte/toast";
  import GuestPicker from "../components/GuestPicker.svelte";
  import MoneyField from "../components/MoneyField.svelte";
  import NumberField from "../components/NumberField.svelte";
  import TextField from "../components/TextField.svelte";
  import { api, errorMessage, newId } from "../api";
  import { addDays, today } from "../dates";
  import { formatMoney } from "../format";
  import { useSession } from "../session/session.svelte";
  import type { EventListItem } from "../bindings/EventListItem";
  import type { Guest } from "../bindings/Guest";
  import type { Layout } from "../bindings/Layout";
  import type { Space } from "../bindings/Space";
  import type { SuiteBooking } from "../bindings/SuiteBooking";
  import type { SuiteBookingEntry } from "../bindings/SuiteBookingEntry";
  import type { SuiteBookingStatus } from "../bindings/SuiteBookingStatus";
  import type { SuitePricing } from "../bindings/SuitePricing";
  import type { SuiteRates } from "../bindings/SuiteRates";
  import {
    datePart,
    estimateSuitePrice,
    joinDateTime,
    layoutLabel,
    offeredLayouts,
    pricingLabel,
    suiteStatusLabel,
    timePart,
  } from "./suites";

  interface Props {
    entry?: SuiteBookingEntry | null;
    initialSuite?: string | null;
    initialDate?: string | null;
    initialEvent?: string | null;
    onSaved: (entry: SuiteBookingEntry) => void;
  }

  let { entry = null, initialSuite = null, initialDate = null, initialEvent = null, onSaved }: Props = $props();
  const session = useSession();
  const canSetPrice = $derived(session.can("override_prices"));
  const NO_EVENT = "none";

  let suites = $state<Space[]>([]);
  let events = $state<EventListItem[]>([]);
  let rates = $state<SuiteRates | null>(null);
  let guest = $state<Guest>(blankGuest());
  let saving = $state(false);

  // The form's fields, filled from the booking being edited.
  let suiteId = $state<string | null>(null);
  let eventId = $state<string | null>(null);
  let title = $state("");
  let date = $state(today());
  let startTime = $state("09:00");
  let endTime = $state("17:00");
  let layout = $state<Layout | null>(null);
  let attendees = $state(10);
  let setup = $state(60);
  let cleardown = $state(30);
  let pricingKind = $state<SuitePricing["kind"]>("hourly");
  let amount = $state(0);
  let status = $state<SuiteBookingStatus>("confirmed");
  let notes = $state("");

  const suite = $derived(suites.find((s) => s.id === suiteId) ?? null);
  const layouts = $derived(suite ? offeredLayouts(suite.suite_capacities) : []);
  const capacity = $derived(layouts.find((l) => l.layout === layout)?.capacity ?? undefined);
  /** A finish at or before the start means it runs past midnight. */
  const overnight = $derived(endTime <= startTime);
  const start = $derived(joinDateTime(date, startTime));
  const end = $derived(joinDateTime(overnight ? addDays(date, 1) : date, endTime));
  const pricing = $derived<SuitePricing>(
    pricingKind === "per_head" || pricingKind === "manual"
      ? { kind: pricingKind, price: amount }
      : { kind: pricingKind },
  );
  const estimate = $derived(rates ? estimateSuitePrice({ start, end, attendees, pricing }, rates) : null);
  const pricingItems = $derived(
    (Object.keys(pricingLabel) as SuitePricing["kind"][])
      .filter((k) => k !== "manual" || canSetPrice || entry?.booking.pricing.kind === "manual")
      .map((k) => ({ value: k, label: pricingLabel[k], disabled: !offered(k) })),
  );
  const suiteItems = $derived(suites.map((s) => ({ value: s.id, label: s.name })));
  const eventItems = $derived([
    { value: NO_EVENT, label: "No event" },
    ...events.map((e) => ({ value: e.id, label: e.name })),
  ]);
  const layoutItems = $derived(
    layouts.map((l) => ({ value: l.layout, label: `${layoutLabel[l.layout]} (seats ${l.capacity})` })),
  );
  const statusItems = (["provisional", "confirmed"] as SuiteBookingStatus[]).map((s) => ({
    value: s,
    label: suiteStatusLabel[s],
  }));

  function blankGuest(): Guest {
    return { id: newId(), first_name: "", last_name: "", email: "", phone: "", address: "", notes: "" };
  }

  function offered(kind: SuitePricing["kind"]): boolean {
    if (!rates) return true;
    if (kind === "hourly") return rates.hourly !== null;
    if (kind === "half_day") return rates.half_day !== null;
    if (kind === "full_day") return rates.full_day !== null;
    return true;
  }

  onMount(async () => {
    try {
      const [spaces, evs] = await Promise.all([api.listSpaces(), api.listEvents(addDays(today(), -60))]);
      suites = spaces.filter((s) => s.kind === "suite").sort((a, b) => a.name.localeCompare(b.name));
      events = evs;
    } catch (error) {
      toast.error("Couldn't load suites", { description: errorMessage(error) });
    }
    if (entry) {
      const b = entry.booking;
      suiteId = b.suite_id;
      eventId = b.event_id;
      title = b.title;
      date = datePart(b.start);
      startTime = timePart(b.start);
      endTime = timePart(b.end);
      layout = b.layout;
      attendees = b.attendees;
      setup = b.setup_minutes;
      cleardown = b.cleardown_minutes;
      pricingKind = b.pricing.kind;
      amount = "price" in b.pricing ? b.pricing.price : 0;
      status = b.status;
      notes = b.notes;
      api
        .searchGuests(entry.guest_name)
        .then((found) => {
          const g = found.find((x) => x.id === b.guest_id);
          if (g) guest = g;
        })
        .catch(() => {});
      await loadRates(false);
    } else {
      suiteId = initialSuite ?? suites[0]?.id ?? null;
      if (initialDate) date = initialDate;
      if (initialEvent) await chooseEvent(initialEvent);
      await loadRates(true);
    }
  });

  /** Loads the suite's rate card; for a new booking, also its default buffers and layout. */
  async function loadRates(useDefaults: boolean) {
    if (!suiteId) return;
    try {
      rates = await api.getSuiteRates(suiteId);
      if (useDefaults) {
        setup = rates.setup_minutes;
        cleardown = rates.cleardown_minutes;
        if (!offered(pricingKind)) pricingKind = rates.full_day !== null ? "full_day" : "per_head";
      }
    } catch (error) {
      toast.error("Couldn't load the suite's rates", { description: errorMessage(error) });
    }
    if (!layouts.some((l) => l.layout === layout)) layout = layouts[0]?.layout ?? null;
  }

  async function chooseSuite(id: string | null) {
    suiteId = id;
    await loadRates(!entry);
  }

  /** Picking an event fills in its organiser and date for a new booking. */
  async function chooseEvent(id: string | null) {
    eventId = id === NO_EVENT ? null : id;
    if (!eventId || entry) return;
    try {
      const detail = await api.getEvent(eventId);
      if (!guest.last_name.trim()) guest = detail.organiser;
      date = detail.event.start_date;
      if (!title.trim()) title = detail.event.name;
    } catch (error) {
      toast.error("Couldn't load the event", { description: errorMessage(error) });
    }
  }

  async function save(event: SubmitEvent) {
    event.preventDefault();
    if (!suiteId || !layout) {
      toast.warning("Choose a suite and a layout");
      return;
    }
    const booking: SuiteBooking = {
      id: entry?.booking.id ?? newId(),
      suite_id: suiteId,
      event_id: eventId,
      guest_id: guest.id,
      title,
      start,
      end,
      layout,
      attendees,
      setup_minutes: setup,
      cleardown_minutes: cleardown,
      pricing,
      status,
      notes,
    };
    saving = true;
    try {
      const saved = await api.saveSuiteBooking(booking, $state.snapshot(guest));
      toast.success(`${saved.suite_name} booked for ${saved.booking.title}`);
      onSaved(saved);
    } catch (error) {
      toast.error("Couldn't save the suite booking", { description: errorMessage(error) });
    } finally {
      saving = false;
    }
  }

  async function cancelBooking() {
    if (!entry) return;
    const yes = await confirm({
      title: `Cancel ${entry.booking.title}?`,
      description: `${entry.suite_name} becomes free again for that time.`,
      confirmLabel: "Cancel booking",
      cancelLabel: "Keep it",
    });
    if (!yes) return;
    try {
      onSaved(await api.setSuiteBookingStatus(entry.booking.id, "cancelled"));
      toast.success("Suite booking cancelled");
    } catch (error) {
      toast.error("Couldn't cancel", { description: errorMessage(error) });
    }
  }
</script>

{#snippet choice<T extends string>(
  label: string,
  items: { value: T; label: string; disabled?: boolean }[],
  value: T | null,
  onchange: (v: T | null) => void,
  description?: string,
)}
  <Field>
    <FieldLabel>{label}</FieldLabel>
    <Select {items} {value} onValueChange={(v) => onchange(v as T | null)}>
      <SelectTrigger><SelectValue /></SelectTrigger>
      <SelectContent>
        {#each items as item (item.value)}
          <SelectItem value={item.value} disabled={item.disabled}>{item.label}<SelectItemIndicator /></SelectItem>
        {/each}
      </SelectContent>
    </Select>
    {#if description}<FieldDescription>{description}</FieldDescription>{/if}
  </Field>
{/snippet}

<form class="form" onsubmit={save}>
  <section class="panel stack" aria-labelledby="hire-heading">
    <h2 id="hire-heading">The hire</h2>
    <div class="row align-top">
      {@render choice("Suite", suiteItems, suiteId, chooseSuite)}
      {@render choice("Event", eventItems, eventId ?? NO_EVENT, chooseEvent, "Part of a wedding, conference or dinner?")}
    </div>
    <TextField label="What it's for" bind:value={title} required placeholder="Wedding breakfast, board meeting…" />
    <div class="row align-top">
      <Field>
        <FieldLabel>Date</FieldLabel>
        <Input type="date" bind:value={date} required />
      </Field>
      <Field>
        <FieldLabel>Starts</FieldLabel>
        <Input type="time" bind:value={startTime} required />
      </Field>
      <Field>
        <FieldLabel>Ends</FieldLabel>
        <Input type="time" bind:value={endTime} required />
        {#if overnight}<FieldDescription>Finishes the next day</FieldDescription>{/if}
      </Field>
    </div>
    <div class="row align-top">
      {@render choice(
        "Layout",
        layoutItems,
        layout,
        (v) => (layout = v),
        layouts.length ? undefined : "Set capacities for this suite in Floors and layout",
      )}
      <NumberField label="Attendees" bind:value={attendees} min={1} max={capacity} />
    </div>
    <div class="row align-top">
      <NumberField label="Setup (minutes)" bind:value={setup} min={0} max={1439} description="The suite is held this long before" />
      <NumberField label="Clear-down (minutes)" bind:value={cleardown} min={0} max={1439} description="…and this long after" />
    </div>
  </section>

  <section class="panel stack" aria-labelledby="price-heading">
    <h2 id="price-heading">Price</h2>
    <div class="row align-top">
      {@render choice("How it's charged", pricingItems, pricingKind, (v) => v && (pricingKind = v))}
      {#if pricingKind === "per_head"}
        <MoneyField label="Price per head" bind:value={amount} />
      {:else if pricingKind === "manual"}
        <MoneyField label="Price" bind:value={amount} />
      {/if}
    </div>
    <p class="estimate">
      {#if estimate === null}
        <span class="muted">This suite has no {pricingLabel[pricingKind].toLowerCase()} price.</span>
      {:else}
        Price <strong>{formatMoney(estimate)}</strong>
        {#if entry && entry.price !== estimate}
          <span class="muted">(booked at {formatMoney(entry.price)}; changing times, numbers or pricing re-prices it)</span>
        {/if}
      {/if}
    </p>
  </section>

  <section class="panel stack" aria-labelledby="organiser-heading">
    <h2 id="organiser-heading">Booked by</h2>
    <GuestPicker bind:guest />
  </section>

  <section class="panel stack" aria-labelledby="more-heading">
    <h2 id="more-heading">Status and notes</h2>
    {#if entry?.booking.status !== "cancelled"}
      {@render choice("Status", statusItems, status, (v) => v && (status = v))}
    {/if}
    <Field>
      <FieldLabel>Notes</FieldLabel>
      <Textarea bind:value={notes} rows={3} placeholder="AV, flowers, access times…" />
    </Field>
  </section>

  <div class="row">
    <Button type="submit" data-variant="solid" disabled={saving}>
      {entry ? "Save changes" : "Book the suite"}
    </Button>
    {#if entry && entry.booking.status !== "cancelled" && session.can("cancel_bookings")}
      <Button data-variant="destructive" onclick={cancelBooking}>Cancel booking</Button>
    {/if}
  </div>
</form>

<style>
  .form {
    display: flex;
    flex-direction: column;
    gap: calc(var(--ui-spacing) * 4);
    max-width: 52rem;
  }

  .estimate {
    margin: 0;
    font-size: 1.05rem;
  }
</style>
