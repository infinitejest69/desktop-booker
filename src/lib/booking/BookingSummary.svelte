<script lang="ts">
  import { onMount, tick } from "svelte";
  import { confirm } from "@ui-sdk/svelte/alert-dialog";
  import { Button } from "@ui-sdk/svelte/button";
  import { toast } from "@ui-sdk/svelte/toast";
  import { api, errorMessage } from "../api";
  import type { BoardOption } from "../bindings/BoardOption";
  import type { BookingDetail } from "../bindings/BookingDetail";
  import { daysBetween, formatDay } from "../dates";
  import { formatMoney } from "../format";
  import { boardName } from "./board";
  import { actionsFor, statusLabel, type StatusAction } from "./status";

  interface Props {
    detail: BookingDetail;
    roomNames: Map<string, string>;
    onChange: (detail: BookingDetail) => void;
    compact?: boolean;
  }

  let { detail, roomNames, onChange, compact = false }: Props = $props();
  let actionsEl = $state<HTMLElement>();
  let boards = $state<BoardOption[]>([]);

  onMount(() => {
    api
      .listBoardOptions()
      .then((b) => (boards = b))
      .catch((error) => toast.error("Couldn't load board options", { description: errorMessage(error) }));
  });

  async function change(action: StatusAction) {
    const ref = detail.booking.reference;
    if (action.confirm) {
      const yes = await confirm({
        title: `${action.label} ${ref}?`,
        description: action.confirm,
        confirmLabel: action.label,
        cancelLabel: "Keep as is",
      });
      if (!yes) return;
    }
    try {
      const saved = await api.setBookingStatus(detail.booking.id, action.to);
      toast.success(`${ref}: ${statusLabel[saved.booking.status]}`);
      onChange(saved);
      // The pressed button is usually replaced, so keep keyboard focus in the actions.
      await tick();
      actionsEl?.querySelector("button")?.focus();
    } catch (error) {
      toast.error(`Couldn't change ${ref}`, { description: errorMessage(error) });
    }
  }
</script>

<div class="summary" class:compact>
  <p class="status-line">
    <span class="status" data-status={detail.booking.status}>{statusLabel[detail.booking.status]}</span>
    <span class="muted">{detail.guest.email || "No email"}</span>
    <span class="muted">{detail.guest.phone || "No phone"}</span>
  </p>

  <div class="row actions" role="group" aria-label="Booking actions" bind:this={actionsEl}>
    {#each actionsFor(detail.booking.status) as action (action.to)}
      <Button data-variant={action.variant} onclick={() => change(action)}>{action.label}</Button>
    {/each}
  </div>

  {#each detail.booking.rooms as room, i (room.id)}
    {const pricing = $derived(detail.pricing[i])}
    <section class="room">
      <h3>
        <span class="fob">{roomNames.get(room.space_id) ?? "?"}</span>
        {formatDay(room.stay.start, { weekday: "short", day: "numeric", month: "short" })} –
        {formatDay(room.stay.end, { weekday: "short", day: "numeric", month: "short" })}
        ({daysBetween(room.stay.start, room.stay.end)} night{daysBetween(room.stay.start, room.stay.end) === 1 ? "" : "s"})
      </h3>
      <p class="muted">
        {room.adults} adult{room.adults === 1 ? "" : "s"}{room.children
          ? `, ${room.children} child${room.children === 1 ? "" : "ren"}`
          : ""}
      </p>
      <p class="board">
        Board: <strong>{boardName(room.board_option_id, boards)}</strong>{#if room.cars}, parking for
          <strong>{room.cars} car{room.cars === 1 ? "" : "s"}</strong>{/if}
      </p>
      {#if pricing && pricing.nights.length}
        {#if compact}
          <p>{formatMoney(pricing.total)}{room.price_override !== null ? " (set by hand)" : ""}</p>
        {:else}
          <table class="list">
            <thead><tr><th>Night</th><th>Room</th><th>Extras</th><th>Board</th><th>Parking</th><th>Total</th></tr></thead>
            <tbody>
              {#each pricing.nights as n (n.night)}
                <tr>
                  <td>{formatDay(n.night, { weekday: "short", day: "numeric", month: "short" })}</td>
                  <td>{formatMoney(n.room)}</td>
                  <td>{formatMoney(n.supplements)}</td>
                  <td>{formatMoney(n.board)}</td>
                  <td>{formatMoney(n.parking)}</td>
                  <td>{formatMoney(n.total)}</td>
                </tr>
              {/each}
            </tbody>
          </table>
          {#if room.price_override !== null}
            <p>
              Price set by hand: <strong>{formatMoney(room.price_override)}</strong> (calculated
              {formatMoney(pricing.calculated)})
            </p>
          {/if}
        {/if}
      {:else}
        <p class="muted">No nights held ({statusLabel[detail.booking.status].toLowerCase()}).</p>
      {/if}
    </section>
  {/each}

  {#if detail.booking.notes}
    <p><strong>Notes:</strong> {detail.booking.notes}</p>
  {/if}
  <p class="total">Total <strong>{formatMoney(detail.total)}</strong></p>
</div>

<style>
  .summary {
    display: flex;
    flex-direction: column;
    gap: calc(var(--ui-spacing) * 3);
  }

  .summary p {
    margin: 0;
  }

  .status-line {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: calc(var(--ui-spacing) * 3);
  }

  .room h3 {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: calc(var(--ui-spacing) * 2);
    font-size: 0.95rem;
    margin: 0 0 calc(var(--ui-spacing) * 1);
  }

  .total {
    font-size: 1.05rem;
  }
</style>
