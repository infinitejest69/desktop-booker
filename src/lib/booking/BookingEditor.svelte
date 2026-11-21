<script lang="ts">
  import { onMount } from "svelte";
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
  import MoneyField from "../components/MoneyField.svelte";
  import NumberField from "../components/NumberField.svelte";
  import TextField from "../components/TextField.svelte";
  import { api, errorMessage, newId } from "../api";
  import { useSession } from "../session/session.svelte";
  import { addDays, daysBetween, formatDay, today } from "../dates";
  import { formatMoney } from "../format";
  import type { BoardOption } from "../bindings/BoardOption";
  import type { Booking } from "../bindings/Booking";
  import type { BookingDetail } from "../bindings/BookingDetail";
  import type { BookingSource } from "../bindings/BookingSource";
  import type { BookingStatus } from "../bindings/BookingStatus";
  import type { Guest } from "../bindings/Guest";
  import type { RoomType } from "../bindings/RoomType";
  import type { Space } from "../bindings/Space";
  import type { StayQuote } from "../bindings/StayQuote";

  interface Props {
    detail?: BookingDetail | null;
    initialRoom?: string | null;
    initialStart?: string | null;
    onSaved: (detail: BookingDetail) => void;
  }

  let { detail = null, initialRoom = null, initialStart = null, onSaved }: Props = $props();
  const session = useSession();
  const canSetPrice = $derived(session.can("override_prices"));

  interface RoomDraft {
    id: string;
    space_id: string | null;
    start: string;
    end: string;
    adults: number;
    children: number;
    board: string | null;
    overrideOn: boolean;
    override: number;
    quote: StayQuote | null;
    free: string[];
  }

  const sources: { value: BookingSource; label: string }[] = [
    { value: "phone", label: "Phone" },
    { value: "email", label: "Email" },
    { value: "walk_in", label: "Walk-in" },
    { value: "website", label: "Website" },
    { value: "agent", label: "Agent / OTA" },
    { value: "other", label: "Other" },
  ];
  const NO_BOARD = "none";

  let rooms = $state<Space[]>([]);
  let roomTypes = $state<RoomType[]>([]);
  let boards = $state<BoardOption[]>([]);
  let guest = $state<Guest>(blankGuest());
  let guestQuery = $state("");
  let guestResults = $state<Guest[]>([]);
  let status = $state<BookingStatus>("confirmed");
  let source = $state<BookingSource>("phone");
  let notes = $state("");
  let drafts = $state<RoomDraft[]>([]);
  let saving = $state(false);

  const isNew = $derived(detail === null);
  const roomById = $derived(new Map(rooms.map((r) => [r.id, r])));
  const typeById = $derived(new Map(roomTypes.map((t) => [t.id, t])));
  const boardItems = $derived([
    { value: NO_BOARD, label: "Room only" },
    ...boards.filter((b) => b.adult_price > 0 || b.child_price > 0).map((b) => ({ value: b.id, label: b.name })),
  ]);
  const total = $derived(
    drafts.reduce((sum, d) => sum + (d.overrideOn ? d.override : (d.quote?.total ?? 0)), 0),
  );

  function blankGuest(): Guest {
    return { id: newId(), first_name: "", last_name: "", email: "", phone: "", address: "", notes: "" };
  }

  function blankRoom(spaceId: string | null, start: string): RoomDraft {
    return {
      id: newId(),
      space_id: spaceId,
      start,
      end: addDays(start, 1),
      adults: 2,
      children: 0,
      board: null,
      overrideOn: false,
      override: 0,
      quote: null,
      free: [],
    };
  }

  onMount(async () => {
    try {
      const [spaces, types, b] = await Promise.all([
        api.listSpaces(),
        api.listRoomTypes(),
        api.listBoardOptions(),
      ]);
      rooms = spaces
        .filter((s) => s.kind === "room")
        .sort((a, c) => a.name.localeCompare(c.name, undefined, { numeric: true }));
      roomTypes = types;
      boards = b;
    } catch (error) {
      toast.error("Couldn't load rooms", { description: errorMessage(error) });
    }
    if (detail) {
      guest = structuredClone(detail.guest);
      status = detail.booking.status;
      source = detail.booking.source;
      notes = detail.booking.notes;
      drafts = detail.booking.rooms.map((r) => ({
        id: r.id,
        space_id: r.space_id,
        start: r.stay.start,
        end: r.stay.end,
        adults: r.adults,
        children: r.children,
        board: r.board_option_id,
        overrideOn: r.price_override !== null,
        override: r.price_override ?? 0,
        quote: null,
        free: [],
      }));
    } else {
      const bb = boards.find((b) => b.code === "BB");
      const first = blankRoom(initialRoom, initialStart ?? today());
      first.board = bb?.id ?? null;
      drafts = [first];
    }
  });

  // Re-quote and re-check free rooms whenever a room's stay details change.
  $effect(() => {
    for (const d of drafts) {
      const key = [d.space_id, d.start, d.end, d.adults, d.children, d.board];
      void key;
      void refresh(d);
    }
  });

  async function refresh(d: RoomDraft) {
    if (daysBetween(d.start, d.end) < 1) {
      d.quote = null;
      return;
    }
    const stay = { start: d.start, end: d.end };
    try {
      d.free = await api.freeRooms(stay, detail?.booking.id ?? null);
      d.quote = d.space_id
        ? await api.quoteRoom(d.space_id, stay, d.adults, d.children, d.board)
        : null;
    } catch {
      d.quote = null;
    }
  }

  function roomItems(d: RoomDraft) {
    const takenHere = new Set(drafts.filter((o) => o !== d).map((o) => o.space_id));
    return rooms.map((r) => {
      const type = typeById.get(r.room_type_id ?? "");
      const free = d.free.includes(r.id) && !takenHere.has(r.id);
      return {
        value: r.id,
        label: `${r.name}${type ? `, ${type.name}` : ""}${free || r.id === d.space_id ? "" : " (taken)"}`,
        disabled: !free && r.id !== d.space_id,
      };
    });
  }

  function maxGuests(d: RoomDraft): number {
    const room = roomById.get(d.space_id ?? "");
    return typeById.get(room?.room_type_id ?? "")?.max_occupancy ?? 10;
  }

  async function findGuests() {
    const q = guestQuery.trim();
    guestResults = q.length < 2 ? [] : await api.searchGuests(q).catch(() => []);
  }

  function useGuest(g: Guest) {
    guest = structuredClone($state.snapshot(g));
    guestResults = [];
    guestQuery = "";
  }

  async function save(event: SubmitEvent) {
    event.preventDefault();
    if (drafts.some((d) => !d.space_id)) {
      toast.warning("Choose a room for every line");
      return;
    }
    const booking: Booking = {
      id: detail?.booking.id ?? newId(),
      reference: detail?.booking.reference ?? "",
      guest_id: guest.id,
      status,
      source,
      notes,
      rooms: drafts.map((d) => ({
        id: d.id,
        space_id: d.space_id as string,
        stay: { start: d.start, end: d.end },
        adults: d.adults,
        children: d.children,
        board_option_id: d.board,
        price_override: d.overrideOn ? d.override : null,
      })),
    };
    saving = true;
    try {
      const saved = await api.saveBooking(booking, $state.snapshot(guest));
      if (!isNew) toast.success(`Saved ${saved.booking.reference}`);
      onSaved(saved);
    } catch (error) {
      toast.error("Couldn't save the booking", { description: errorMessage(error) });
    } finally {
      saving = false;
    }
  }
</script>

<form class="editor" onsubmit={save}>
  <section class="panel stack" aria-labelledby="guest-heading">
    <h2 id="guest-heading">Guest</h2>
    <div class="find">
      <Field>
        <FieldLabel>Find an existing guest</FieldLabel>
        <Input
          type="search"
          bind:value={guestQuery}
          oninput={findGuests}
          placeholder="Name, email or phone"
        />
      </Field>
      {#if guestResults.length}
        <ul class="results" aria-label="Matching guests">
          {#each guestResults as g (g.id)}
            <li>
              <Button onclick={() => useGuest(g)}>
                {g.first_name} {g.last_name}{g.email ? ` (${g.email})` : ""}
              </Button>
            </li>
          {/each}
        </ul>
      {/if}
    </div>
    <div class="row">
      <TextField label="First name" bind:value={guest.first_name} />
      <TextField label="Last name" bind:value={guest.last_name} required />
    </div>
    <div class="row">
      <TextField label="Email" type="email" bind:value={guest.email} />
      <TextField label="Phone" type="tel" bind:value={guest.phone} />
    </div>
  </section>

  <section class="stack" aria-labelledby="rooms-heading">
    <h2 id="rooms-heading">Rooms</h2>
    {#each drafts as d, i (d.id)}
      <fieldset class="panel stack room-line">
        <legend>Room {i + 1}</legend>
        <div class="row align-top">
          <Field>
            <FieldLabel>Arrival</FieldLabel>
            <Input
              type="date"
              value={d.start}
              onValueChange={(v) => {
                if (!v) return;
                const nights = Math.max(daysBetween(d.start, d.end), 1);
                d.start = v;
                d.end = addDays(v, nights);
              }}
              required
            />
          </Field>
          <Field>
            <FieldLabel>Departure</FieldLabel>
            <Input type="date" bind:value={d.end} min={addDays(d.start, 1)} required />
            <FieldDescription>
              {daysBetween(d.start, d.end)} night{daysBetween(d.start, d.end) === 1 ? "" : "s"}
            </FieldDescription>
          </Field>
          <Field>
            <FieldLabel>Room</FieldLabel>
            <Select items={roomItems(d)} bind:value={d.space_id} placeholder="Choose a room">
              <SelectTrigger><SelectValue /></SelectTrigger>
              <SelectContent>
                {#each roomItems(d) as item (item.value)}
                  <SelectItem value={item.value} disabled={item.disabled}>
                    {item.label}<SelectItemIndicator />
                  </SelectItem>
                {/each}
              </SelectContent>
            </Select>
          </Field>
        </div>
        <div class="row">
          <NumberField label="Adults" bind:value={d.adults} min={1} max={maxGuests(d)} />
          <NumberField label="Children" bind:value={d.children} min={0} max={maxGuests(d)} />
          <Field>
            <FieldLabel>Board</FieldLabel>
            <Select
              items={boardItems}
              value={d.board ?? NO_BOARD}
              onValueChange={(v) => (d.board = v === NO_BOARD ? null : v)}
            >
              <SelectTrigger><SelectValue /></SelectTrigger>
              <SelectContent>
                {#each boardItems as b (b.value)}
                  <SelectItem value={b.value}>{b.label}<SelectItemIndicator /></SelectItem>
                {/each}
              </SelectContent>
            </Select>
          </Field>
        </div>

        {#if d.quote}
          <details class="quote">
            <summary>
              Price {formatMoney(d.quote.total)} for {d.quote.nights.length} night{d.quote.nights.length === 1 ? "" : "s"}
            </summary>
            <table class="list">
              <thead>
                <tr><th>Night</th><th>Room</th><th>Extras</th><th>Board</th><th>Total</th></tr>
              </thead>
              <tbody>
                {#each d.quote.nights as n (n.night)}
                  <tr>
                    <td>{formatDay(n.night, { weekday: "short", day: "numeric", month: "short" })}</td>
                    <td>{formatMoney(n.room)}{n.rules.length ? ` (${n.rules.join(", ")})` : ""}</td>
                    <td>{formatMoney(n.supplements)}</td>
                    <td>{formatMoney(n.board)}</td>
                    <td>{formatMoney(n.total)}</td>
                  </tr>
                {/each}
              </tbody>
            </table>
          </details>
        {/if}
        <div class="row">
          {#if canSetPrice}
          <label data-slot="label" class="switch-row">
            Set the price by hand
            <Switch
              checked={d.overrideOn}
              onCheckedChange={(on) => {
                d.overrideOn = on;
                if (on && d.override === 0) d.override = d.quote?.total ?? 0;
              }}
            ><SwitchThumb /></Switch>
          </label>
          {#if d.overrideOn}
            <MoneyField label="Price for this room's stay" bind:value={d.override} />
          {/if}
          {:else if d.overrideOn}
            <p class="muted" style="margin: 0">Price set by an admin: {formatMoney(d.override)}</p>
          {/if}
          {#if drafts.length > 1}
            <Button data-variant="destructive" onclick={() => (drafts = drafts.filter((x) => x !== d))}>
              Remove room {i + 1}
            </Button>
          {/if}
        </div>
      </fieldset>
    {/each}
    <div>
      <Button
        onclick={() => {
          const last = drafts.at(-1);
          const next = blankRoom(null, last?.start ?? today());
          if (last) next.end = last.end;
          next.board = last?.board ?? null;
          drafts.push(next);
        }}>Add another room</Button
      >
    </div>
  </section>

  <section class="panel stack" aria-labelledby="details-heading">
    <h2 id="details-heading">Booking</h2>
    <div class="row">
      {#if isNew}
        <Field>
          <FieldLabel>Status</FieldLabel>
          <Select
            items={[
              { value: "confirmed", label: "Confirmed" },
              { value: "provisional", label: "Provisional" },
            ]}
            value={status}
            onValueChange={(v) => v && (status = v as BookingStatus)}
          >
            <SelectTrigger><SelectValue /></SelectTrigger>
            <SelectContent>
              <SelectItem value="confirmed">Confirmed<SelectItemIndicator /></SelectItem>
              <SelectItem value="provisional">Provisional<SelectItemIndicator /></SelectItem>
            </SelectContent>
          </Select>
        </Field>
      {/if}
      <Field>
        <FieldLabel>Booked by</FieldLabel>
        <Select items={sources} value={source} onValueChange={(v) => v && (source = v as BookingSource)}>
          <SelectTrigger><SelectValue /></SelectTrigger>
          <SelectContent>
            {#each sources as s (s.value)}
              <SelectItem value={s.value}>{s.label}<SelectItemIndicator /></SelectItem>
            {/each}
          </SelectContent>
        </Select>
      </Field>
    </div>
    <Field>
      <FieldLabel>Notes</FieldLabel>
      <Textarea bind:value={notes} rows={2} placeholder="Late arrival, dog, anniversary…" />
    </Field>
    <div class="row total-row">
      <p class="total">Total <strong>{formatMoney(total)}</strong></p>
      <Button type="submit" data-variant="solid" disabled={saving}>
        {isNew ? "Make booking" : "Save changes"}
      </Button>
    </div>
  </section>
</form>

<style>
  .editor {
    display: flex;
    flex-direction: column;
    gap: calc(var(--ui-spacing) * 6);
    max-width: 52rem;
  }

  .find {
    position: relative;
  }

  .results {
    list-style: none;
    margin: calc(var(--ui-spacing) * 2) 0 0;
    padding: 0;
    display: flex;
    flex-wrap: wrap;
    gap: calc(var(--ui-spacing) * 2);
  }

  .room-line {
    margin: 0;
  }

  .align-top {
    align-items: flex-start;
  }

  .room-line legend {
    font-weight: 600;
    padding: 0 calc(var(--ui-spacing) * 1);
  }

  .quote summary {
    cursor: pointer;
    font-weight: 600;
  }

  .switch-row {
    display: flex;
    align-items: center;
    gap: calc(var(--ui-spacing) * 3);
    min-height: calc(var(--ui-spacing) * 8);
  }

  .total-row {
    justify-content: space-between;
    align-items: center;
  }

  .total {
    margin: 0;
    font-size: 1.05rem;
  }
</style>
