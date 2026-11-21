<script lang="ts">
  import { onMount, untrack } from "svelte";
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
  import { Switch, SwitchThumb } from "@ui-sdk/svelte/switch";
  import { Textarea } from "@ui-sdk/svelte/textarea";
  import { toast } from "@ui-sdk/svelte/toast";
  import GuestPicker from "../components/GuestPicker.svelte";
  import MoneyField from "../components/MoneyField.svelte";
  import NumberField from "../components/NumberField.svelte";
  import TextField from "../components/TextField.svelte";
  import { api, errorMessage, newId } from "../api";
  import { addDays, daysBetween, today } from "../dates";
  import { formatMoney } from "../format";
  import { useSession } from "../session/session.svelte";
  import type { CateringSitting } from "../bindings/CateringSitting";
  import type { Event } from "../bindings/Event";
  import type { EventDetail } from "../bindings/EventDetail";
  import type { EventKind } from "../bindings/EventKind";
  import type { Guest } from "../bindings/Guest";
  import type { RoomType } from "../bindings/RoomType";
  import { eventKindLabel, timePart } from "../suites/suites";

  interface Props {
    detail?: EventDetail | null;
    onSaved: (detail: EventDetail) => void;
  }

  let { detail = null, onSaved }: Props = $props();
  const session = useSession();
  const canSetRate = $derived(session.can("override_prices"));
  const ANY_TYPE = "any";

  interface SittingDraft extends CateringSitting {
    key: string;
  }

  // The form is filled once from the event it opens with.
  const initial = untrack(() => detail);
  const start = initial?.event.start_date ?? addDays(today(), 30);
  const block = initial?.event.room_block ?? null;
  let name = $state(initial?.event.name ?? "");
  let kind = $state<EventKind>(initial?.event.kind ?? "wedding");
  let startDate = $state(start);
  let endDate = $state(initial?.event.end_date ?? start);
  let notes = $state(initial?.event.notes ?? "");
  let runningOrder = $state(initial?.event.running_order ?? "");
  let organiser = $state<Guest>(
    initial
      ? $state.snapshot(initial.organiser)
      : { id: newId(), first_name: "", last_name: "", email: "", phone: "", address: "", notes: "" },
  );
  let sittings = $state<SittingDraft[]>(
    (initial?.event.sittings ?? []).map((s) => ({ ...s, time: timePart(s.time), key: newId() })),
  );
  let holdRooms = $state(block !== null);
  let blockType = $state<string>(block?.room_type_id ?? ANY_TYPE);
  let blockRooms = $state(block?.rooms ?? 10);
  let blockIn = $state(block?.stay.start ?? start);
  let blockOut = $state(block?.stay.end ?? addDays(start, 1));
  let blockRate = $state(block?.rate ?? 0);
  let blockRelease = $state(block?.release_date ?? addDays(start, -28));
  let roomTypes = $state<RoomType[]>([]);
  let saving = $state(false);

  const kindItems = (Object.keys(eventKindLabel) as EventKind[]).map((k) => ({ value: k, label: eventKindLabel[k] }));
  const typeItems = $derived([
    { value: ANY_TYPE, label: "Any room type" },
    ...roomTypes.map((t) => ({ value: t.id, label: t.name })),
  ]);
  const blockNights = $derived(Math.max(daysBetween(blockIn, blockOut), 0));

  onMount(() => {
    api
      .listRoomTypes()
      .then((t) => {
        roomTypes = t;
        // A new block starts at the first type's normal price.
        if (!block && !blockRate) blockRate = t[0]?.base_rate ?? 0;
      })
      .catch(() => {});
  });

  function addSitting() {
    sittings.push({ key: newId(), name: "", date: startDate, time: "19:00", adults: 50, children: 0, dietary: "" });
  }

  async function save(e: SubmitEvent) {
    e.preventDefault();
    const event: Event = {
      id: detail?.event.id ?? newId(),
      name,
      kind,
      organiser_id: organiser.id,
      start_date: startDate,
      end_date: endDate < startDate ? startDate : endDate,
      notes,
      running_order: runningOrder,
      sittings: sittings.map(({ key: _key, ...s }) => ({ ...s, time: `${s.time.slice(0, 5)}:00` })),
      room_block: holdRooms
        ? {
            room_type_id: blockType === ANY_TYPE ? null : blockType,
            rooms: blockRooms,
            stay: { start: blockIn, end: blockOut },
            rate: blockRate,
            release_date: blockRelease,
          }
        : null,
    };
    saving = true;
    try {
      const saved = await api.saveEvent($state.snapshot(event), $state.snapshot(organiser));
      toast.success(`Saved ${saved.event.name}`, {
        description: saved.event.room_block
          ? `${saved.held_rooms} room${saved.held_rooms === 1 ? "" : "s"} held`
          : undefined,
      });
      onSaved(saved);
    } catch (error) {
      toast.error("Couldn't save the event", { description: errorMessage(error) });
    } finally {
      saving = false;
    }
  }
</script>

<form class="form" onsubmit={save}>
  <section class="panel stack" aria-labelledby="event-heading">
    <h2 id="event-heading">The event</h2>
    <div class="row align-top">
      <TextField label="Event name" bind:value={name} required placeholder="Hughes–Price wedding" />
      <Field>
        <FieldLabel>Type</FieldLabel>
        <Select items={kindItems} value={kind} onValueChange={(v) => v && (kind = v as EventKind)}>
          <SelectTrigger><SelectValue /></SelectTrigger>
          <SelectContent>
            {#each kindItems as k (k.value)}
              <SelectItem value={k.value}>{k.label}<SelectItemIndicator /></SelectItem>
            {/each}
          </SelectContent>
        </Select>
      </Field>
    </div>
    <div class="row align-top">
      <Field>
        <FieldLabel>First day</FieldLabel>
        <Input
          type="date"
          value={startDate}
          onValueChange={(v) => {
            if (!v) return;
            if (endDate < v) endDate = v;
            startDate = v;
          }}
          required
        />
      </Field>
      <Field>
        <FieldLabel>Last day</FieldLabel>
        <Input type="date" bind:value={endDate} min={startDate} required />
      </Field>
    </div>
  </section>

  <section class="panel stack" aria-labelledby="organiser-heading">
    <h2 id="organiser-heading">Organiser</h2>
    <GuestPicker bind:guest={organiser} searchLabel="Find an existing contact" />
  </section>

  <section class="panel stack" aria-labelledby="catering-heading">
    <h2 id="catering-heading">Catering</h2>
    {#each sittings as s, i (s.key)}
      <fieldset class="stack sitting">
        <legend>Sitting {i + 1}</legend>
        <div class="row align-top">
          <TextField label="Name" bind:value={s.name} required placeholder="Wedding breakfast" />
          <Field>
            <FieldLabel>Date</FieldLabel>
            <Input type="date" bind:value={s.date} required />
          </Field>
          <Field>
            <FieldLabel>Time</FieldLabel>
            <Input type="time" bind:value={s.time} required />
          </Field>
        </div>
        <div class="row align-top">
          <NumberField label="Adults" bind:value={s.adults} min={0} />
          <NumberField label="Children" bind:value={s.children} min={0} />
        </div>
        <Field>
          <FieldLabel>Dietary needs</FieldLabel>
          <Textarea bind:value={s.dietary} rows={3} placeholder={"One per line, e.g.\n4 vegetarian\n1 nut allergy"} />
        </Field>
        <div>
          <Button data-variant="destructive" onclick={() => sittings.splice(i, 1)}>Remove sitting {i + 1}</Button>
        </div>
      </fieldset>
    {:else}
      <p class="muted">No food or drink sittings yet.</p>
    {/each}
    <div><Button onclick={addSitting}>Add a sitting</Button></div>
  </section>

  <section class="panel stack" aria-labelledby="block-heading">
    <h2 id="block-heading">Rooms for guests</h2>
    <label data-slot="label" class="switch-row">
      Hold a block of rooms
      <Switch bind:checked={holdRooms}><SwitchThumb /></Switch>
    </label>
    {#if holdRooms}
      <p class="muted" style="margin: 0">
        Held rooms are provisional bookings at the group rate. Confirm one when a guest takes it up; any
        still held on the release date go back on sale.
      </p>
      <div class="row align-top">
        <Field>
          <FieldLabel>Room type</FieldLabel>
          <Select items={typeItems} value={blockType} onValueChange={(v) => v && (blockType = v)}>
            <SelectTrigger><SelectValue /></SelectTrigger>
            <SelectContent>
              {#each typeItems as t (t.value)}
                <SelectItem value={t.value}>{t.label}<SelectItemIndicator /></SelectItem>
              {/each}
            </SelectContent>
          </Select>
        </Field>
        <NumberField label="Rooms to hold" bind:value={blockRooms} min={1} max={500} />
      </div>
      <div class="row align-top">
        <Field>
          <FieldLabel>Arrival</FieldLabel>
          <Input type="date" bind:value={blockIn} required />
        </Field>
        <Field>
          <FieldLabel>Departure</FieldLabel>
          <Input type="date" bind:value={blockOut} min={addDays(blockIn, 1)} required />
          <FieldDescription>{blockNights} night{blockNights === 1 ? "" : "s"}</FieldDescription>
        </Field>
        <Field>
          <FieldLabel>Release date</FieldLabel>
          <Input type="date" bind:value={blockRelease} max={blockIn} required />
          <FieldDescription>Unclaimed rooms go back on sale</FieldDescription>
        </Field>
      </div>
      {#if canSetRate}
        <MoneyField
          label="Group rate per room per night"
          bind:value={blockRate}
          description={`${formatMoney(blockRate * blockNights)} per room for the stay`}
        />
      {:else}
        <p>
          Group rate <strong>{formatMoney(blockRate)}</strong> per room per night
          <span class="muted">(only an admin can change it)</span>
        </p>
      {/if}
    {/if}
  </section>

  <section class="panel stack" aria-labelledby="plan-heading">
    <h2 id="plan-heading">Running order and notes</h2>
    <Field>
      <FieldLabel>Running order</FieldLabel>
      <Textarea bind:value={runningOrder} rows={6} placeholder={"13:00 Ceremony\n15:00 Wedding breakfast"} />
    </Field>
    <Field>
      <FieldLabel>Notes</FieldLabel>
      <Textarea bind:value={notes} rows={3} placeholder="Contacts on the day, suppliers, invoicing…" />
    </Field>
  </section>

  <div class="row">
    <Button type="submit" data-variant="solid" disabled={saving}>{detail ? "Save changes" : "Create event"}</Button>
  </div>
</form>

<style>
  .form {
    display: flex;
    flex-direction: column;
    gap: calc(var(--ui-spacing) * 4);
    max-width: 52rem;
  }

  .sitting {
    margin: 0;
    padding: calc(var(--ui-spacing) * 3);
    border: 1px solid var(--ui-border);
    border-radius: 8px;
  }

  .switch-row {
    display: flex;
    align-items: center;
    gap: calc(var(--ui-spacing) * 3);
    font-weight: 600;
  }
</style>
