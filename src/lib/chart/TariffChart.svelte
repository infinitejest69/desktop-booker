<script lang="ts">
  import type { GridCell } from "@ui-sdk/core";
  import type { ChartEntry } from "../bindings/ChartEntry";
  import type { Space } from "../bindings/Space";
  import { boardLabel } from "../booking/board";
  import { datesFrom, formatDay, isWeekend, today } from "../dates";
  import { ChartCell, ChartGrid, ChartHeader, ChartRow, ChartRowHeader } from "./chart-parts";
  import { barAt, layoutBars } from "./layout";
  import FeatureIcons from "../rooms/FeatureIcons.svelte";
  import { roomFeatureSummary } from "../rooms/features";

  interface Props {
    rooms: Space[];
    codes: Map<string, string>;
    entries: ChartEntry[];
    from: string;
    /** Visible nights. With `fit`, the chart sets this to fill its width. */
    days: number;
    fit?: boolean;
    highlight?: (string | null)[];
    label?: string;
    onBooking: (bookingId: string) => void;
    onFree: (roomId: string, date: string) => void;
  }

  let {
    rooms,
    codes,
    entries,
    from,
    days = $bindable(),
    fit = false,
    highlight = [],
    label = "Booking calendar",
    onBooking,
    onFree,
  }: Props = $props();

  const statusLabel: Record<string, string> = {
    provisional: "provisional",
    confirmed: "confirmed",
    checked_in: "checked in",
    checked_out: "checked out",
  };

  const ROOM_COL_PX = 136; // --room-col (8.5rem) at 16px
  const MIN_DAY_PX = 50;
  let width = $state(0);

  $effect(() => {
    if (!fit || width === 0) return;
    const fitting = Math.floor((width - ROOM_COL_PX - 2) / MIN_DAY_PX);
    const next = Math.min(Math.max(fitting, 7), 62);
    if (next !== days) days = next;
  });

  const dates = $derived(datesFrom(from, days));
  const bars = $derived(layoutBars(entries, from, days));
  const todayIso = today();

  function cellLabel(room: Space, date: string, col: number): string {
    const day = formatDay(date, { weekday: "long", day: "numeric", month: "long" });
    const bar = barAt(bars.get(room.id), col);
    if (!bar) return `Room ${room.name}, ${day}: free`;
    const e = bar.entry;
    return `Room ${room.name}, ${day}: ${e.guest_name}, ${e.reference}, ${boardLabel(e.board)}, ${statusLabel[e.status] ?? e.status}`;
  }

  function onActivate(cell: GridCell) {
    const room = rooms[cell.row];
    if (!room) return;
    const bar = barAt(bars.get(room.id), cell.col);
    if (bar) onBooking(bar.entry.booking_id);
    else onFree(room.id, dates[cell.col]);
  }
</script>

<div class="scroll" bind:clientWidth={width}>
  <ChartGrid
    class="chart"
    aria-label={label}
    rows={rooms.length}
    cols={days}
    {onActivate}
    style={`--days: ${days}`}
  >
    <ChartRow class="chart-row head">
      <div class="corner" aria-hidden="true"></div>
      {#each dates as date (date)}
        <ChartHeader
          class="day"
          data-weekend={isWeekend(date) ? "" : undefined}
          data-today={date === todayIso ? "" : undefined}
        >
          <span class="dow">{formatDay(date, { weekday: "short" })}</span>
          <span class="dom">{formatDay(date, { day: "numeric" })}</span>
          <span class="mon">{formatDay(date, { month: "short" })}</span>
        </ChartHeader>
      {/each}
    </ChartRow>
    {#each rooms as room, row (room.id)}
      <ChartRow class="chart-row" data-out-of-service={room.out_of_service ? "" : undefined}>
        <ChartRowHeader class="room">
          <span class="fob">{room.name}</span>
          <span class="room-about">
            <span class="room-type">{codes.get(room.room_type_id ?? "") ?? ""}</span>
            <span class="room-icons" title={roomFeatureSummary(room.features)}><FeatureIcons features={room.features} size={11} /></span>
          </span>
        </ChartRowHeader>
        {#each dates as date, col (date)}
          <ChartCell
            id={`${row}:${col}`}
            {row}
            {col}
            class="cell"
            aria-label={cellLabel(room, date, col)}
            data-weekend={isWeekend(date) ? "" : undefined}
            data-today={date === todayIso ? "" : undefined}
          >
            {#each (bars.get(room.id) ?? []).filter((b) => b.col === col) as bar (bar.entry.booking_room_id)}
              <div
                class="bar"
                aria-hidden="true"
                data-status={bar.entry.status}
                data-highlight={highlight.includes(bar.entry.booking_id) ? "" : undefined}
                data-clipped-start={bar.clippedStart ? "" : undefined}
                data-clipped-end={bar.clippedEnd ? "" : undefined}
                style={`--span: ${bar.span}`}
              >
                <span class="guest">{bar.entry.guest_name}</span>
                <span class="ref">{bar.entry.reference}</span>
              </div>
            {/each}
          </ChartCell>
        {/each}
      </ChartRow>
    {/each}
  </ChartGrid>
</div>
<div class="legend" aria-hidden="true">
  <span data-status="provisional">Provisional</span>
  <span data-status="confirmed">Confirmed</span>
  <span data-status="checked_in">Checked in</span>
  <span data-status="checked_out">Checked out</span>
</div>

<style>
  .scroll {
    background: var(--paper);
    overflow: auto;
    max-height: calc(100vh - 15rem);
    border: 1px solid var(--ui-border);
    border-radius: var(--ui-radius);
  }

  .scroll :global(.chart) {
    --room-col: 8.5rem;
    --day-col: 3.125rem;
    --row-h: 2.4rem;
    display: grid;
    grid-template-columns: var(--room-col) repeat(var(--days), minmax(var(--day-col), 1fr));
    min-width: 100%;
    width: max-content;
  }

  .scroll :global(.chart-row) {
    display: contents;
  }

  .scroll :global(.corner),
  .scroll :global(.day) {
    position: sticky;
    top: 0;
    z-index: 3;
    background: var(--ui-background);
    border-bottom: 1px solid var(--ui-border);
  }

  .scroll :global(.corner) {
    left: 0;
    z-index: 4;
  }

  .scroll :global(.day) {
    display: flex;
    flex-direction: column;
    align-items: center;
    padding: calc(var(--ui-spacing) * 1) 0;
    font-size: 0.75rem;
    line-height: 1.2;
  }

  .scroll :global(.day .dom) {
    font-size: 0.95rem;
    font-weight: 600;
  }

  .scroll :global(.day[data-weekend]) {
    background: var(--ui-muted);
  }

  .scroll :global(.day[data-today]) {
    color: var(--heather);
    box-shadow: inset 0 -3px 0 var(--heather);
  }

  .scroll :global(.room) {
    position: sticky;
    left: 0;
    z-index: 2;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.25rem;
    padding: 0 calc(var(--ui-spacing) * 2);
    height: var(--row-h);
    background: var(--paper);
    border-bottom: 1px solid var(--ui-border);
    border-inline-end: 1px solid var(--ui-border);
  }

  .scroll :global(.room-name) {
    font-weight: 600;
  }

  .scroll :global(.room-about) {
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    gap: 2px;
  }

  .scroll :global(.room-icons) {
    color: var(--ui-muted-foreground);
  }

  .scroll :global(.room-type) {
    font-size: 0.75rem;
    color: var(--ui-muted-foreground);
  }

  .scroll :global(.cell) {
    position: relative;
    height: var(--row-h);
    border-bottom: 1px solid var(--ui-border);
    border-inline-end: 1px solid var(--ui-border);
    cursor: pointer;
  }

  .scroll :global(.cell[data-weekend]) {
    background: color-mix(in oklab, var(--ui-muted) 70%, var(--paper));
  }

  .scroll :global(.cell:hover) {
    background: var(--ui-muted);
  }

  .scroll :global(.cell:focus-visible),
  .scroll :global(.cell[data-active]:focus) {
    outline: 2px solid var(--ui-ring-border);
    outline-offset: -2px;
    z-index: 2;
  }

  .scroll :global(.chart-row[data-out-of-service] .cell) {
    background-image: repeating-linear-gradient(
      45deg,
      transparent 0 6px,
      light-dark(oklch(0 0 0 / 0.06), oklch(1 0 0 / 0.08)) 6px 12px
    );
  }

  .scroll :global(.bar) {
    position: absolute;
    z-index: 1;
    inset-block: 4px;
    /* The bar sits in its first night's cell, so 100% is one column: it starts
       mid-cell (afternoon check-in) and ends mid-cell on departure day. */
    inset-inline-start: 50%;
    width: calc(var(--span) * 100% - 4px);
    box-sizing: border-box;
    display: flex;
    align-items: center;
    gap: 0.4rem;
    padding: 0 0.4rem;
    border-radius: 5px;
    border: 1.5px solid transparent;
    font-size: 0.75rem;
    overflow: hidden;
    white-space: nowrap;
    pointer-events: none;
  }

  .scroll :global(.bar[data-clipped-start]) {
    inset-inline-start: 0;
    width: calc(var(--span) * 100% + 50% - 4px);
    border-start-start-radius: 0;
    border-end-start-radius: 0;
  }

  .scroll :global(.bar[data-clipped-end]) {
    width: calc(var(--span) * 100% - 50%);
    border-start-end-radius: 0;
    border-end-end-radius: 0;
  }

  .scroll :global(.bar[data-clipped-start][data-clipped-end]) {
    width: calc(var(--span) * 100%);
  }

  .scroll :global(.bar .guest) {
    font-weight: 600;
  }

  .scroll :global(.bar[data-status="confirmed"]),
  .legend [data-status="confirmed"]::before {
    background: var(--st-confirmed-bg);
    border-color: var(--st-confirmed-edge);
  }

  .scroll :global(.bar[data-status="provisional"]),
  .legend [data-status="provisional"]::before {
    background: var(--st-provisional-bg);
    border-color: var(--st-provisional-edge);
    border-style: dashed;
  }

  .scroll :global(.bar[data-status="checked_in"]),
  .legend [data-status="checked_in"]::before {
    background: var(--st-in-house-bg);
    border-color: var(--st-in-house-edge);
  }

  .scroll :global(.bar[data-status="checked_out"]),
  .legend [data-status="checked_out"]::before {
    background: var(--st-done-bg);
    border-color: var(--st-done-edge);
  }

  .scroll :global(.bar[data-highlight]) {
    outline: 3px solid var(--heather);
    outline-offset: 1px;
  }


  .legend {
    display: flex;
    gap: calc(var(--ui-spacing) * 5);
    margin-top: calc(var(--ui-spacing) * 3);
    color: var(--ui-muted-foreground);
  }

  .legend span::before {
    content: "";
    display: inline-block;
    width: 0.9em;
    height: 0.9em;
    margin-inline-end: 0.4em;
    vertical-align: -0.1em;
    border-radius: 3px;
    border: 1.5px solid;
  }
</style>
