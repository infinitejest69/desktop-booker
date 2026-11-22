<script lang="ts">
  import type { ReportResult } from "../bindings/ReportResult";
  import { axisLabel, compact, formatValue, isTimeSplit, niceMax, periodLabel } from "./format";

  // Draws a report: one figure, bars (columns over time, rows for anything
  // else), a line, or a table. One series, in the app's heather. Every chart
  // but the table has its figures in a table underneath too.
  interface Props {
    result: ReportResult;
    /** Smaller, for dashboards. */
    small?: boolean;
  }

  let { result, small = false }: Props = $props();

  const unit = $derived(result.unit);
  const rows = $derived(result.rows);
  const kind = $derived(result.spec.split === "none" ? "number" : result.spec.chart);
  const overTime = $derived(isTimeSplit(result.spec.split));
  const top = $derived(niceMax(Math.max(0, ...rows.map((r) => r.value))));
  const share = (value: number) => `${Math.max(0, (value / top) * 100)}%`;
  /** Which labels fit under the columns: all of a short run, else first and last. */
  const showLabel = (i: number) => rows.length <= 12 || i === 0 || i === rows.length - 1;

  let hovered = $state<number | null>(null);

  // Line chart geometry, in pixels of the drawing's own width.
  let width = $state(600);
  const height = $derived(small ? 120 : 180);
  const inset = 8;
  const x = (i: number) =>
    inset + (rows.length < 2 ? (width - inset * 2) / 2 : (i * (width - inset * 2)) / (rows.length - 1));
  const y = (value: number) => height - (Math.max(0, value) / top) * (height - inset);
  const path = $derived(rows.map((r, i) => `${i ? "L" : "M"}${x(i)},${y(r.value)}`).join(" "));

  function nearest(event: PointerEvent) {
    const box = (event.currentTarget as SVGElement).getBoundingClientRect();
    const at = event.clientX - box.left;
    let best = 0;
    rows.forEach((_, i) => {
      if (Math.abs(x(i) - at) < Math.abs(x(best) - at)) best = i;
    });
    hovered = best;
  }

</script>

{#snippet figures()}
  <table class="list figures-table">
    <thead><tr><th>{overTime ? "When" : "Group"}</th><th class="num">Value</th></tr></thead>
    <tbody>
      {#each rows as row (row.key)}
        <tr><td>{row.label}</td><td class="num">{formatValue(unit, row.value)}</td></tr>
      {/each}
    </tbody>
    <tfoot><tr><th>Total</th><th class="num">{formatValue(unit, result.total)}</th></tr></tfoot>
  </table>
{/snippet}

{#snippet axis()}
  <div class="axis" aria-hidden="true">
    <span style:bottom="100%">{compact(unit, top)}</span>
    <span style:bottom="50%">{compact(unit, top / 2)}</span>
    <span style:bottom="0%">{compact(unit, 0)}</span>
  </div>
{/snippet}

<figure class="report" class:small>
  {#if kind === "number"}
    <p class="figure">{formatValue(unit, result.total)}</p>
  {:else if kind === "table"}
    {@render figures()}
  {:else}
    <p class="total">Total <strong>{formatValue(unit, result.total)}</strong></p>
    {#if kind === "bar" && !overTime}
      <ul class="rows">
        {#each rows as row (row.key)}
          <li>
            <span class="row-label">{row.label}</span>
            <span class="track"><span class="fill" style:width={share(row.value)}></span></span>
            <span class="row-value">{formatValue(unit, row.value)}</span>
          </li>
        {/each}
      </ul>
    {:else if kind === "bar"}
      <div class="plot">
        {@render axis()}
        <div class="columns" style:height={`${height}px`} role="group" aria-label={result.spec.name}>
          {#each rows as row, i (row.key)}
            <button
              type="button"
              class="column"
              aria-label={`${row.label}: ${formatValue(unit, row.value)}`}
              onpointerenter={() => (hovered = i)}
              onpointerleave={() => (hovered = null)}
              onfocus={() => (hovered = i)}
              onblur={() => (hovered = null)}
            >
              <span class="col-fill" style:height={share(row.value)}></span>
              {#if hovered === i}
                <span class="tip" aria-hidden="true">{row.label}<strong>{formatValue(unit, row.value)}</strong></span>
              {/if}
            </button>
          {/each}
        </div>
        <div class="x-labels" aria-hidden="true">
          {#each rows as row, i (row.key)}
            <span>{showLabel(i) ? axisLabel(result.spec.split, row.key, row.label) : ""}</span>
          {/each}
        </div>
      </div>
    {:else}
      <div class="plot">
        {@render axis()}
        <div class="line" bind:clientWidth={width} style:height={`${height}px`}>
          <!-- Hover is for pointers; keyboard and screen-reader users read the
               same values in the Figures table below. -->
          <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
          <svg
            {width}
            {height}
            role="img"
            aria-label={`${result.spec.name}: line chart; the figures are in the table below.`}
            onpointermove={nearest}
            onpointerleave={() => (hovered = null)}
          >
            <line class="grid" x1="0" x2={width} y1={inset} y2={inset} />
            <line class="grid" x1="0" x2={width} y1={height / 2 + inset / 2} y2={height / 2 + inset / 2} />
            <path d={path} />
            {#if hovered !== null && rows[hovered]}
              <line class="cross" x1={x(hovered)} x2={x(hovered)} y1="0" y2={height} />
              <circle cx={x(hovered)} cy={y(rows[hovered].value)} r="4" />
            {/if}
          </svg>
          {#if hovered !== null && rows[hovered]}
            <span class="tip" style:left={`${x(hovered)}px`} aria-hidden="true">
              {rows[hovered].label}<strong>{formatValue(unit, rows[hovered].value)}</strong>
            </span>
          {/if}
        </div>
        <div class="x-labels ends" aria-hidden="true">
          <span>{rows[0] ? axisLabel(result.spec.split, rows[0].key, rows[0].label) : ""}</span>
          <span>{rows.at(-1) ? axisLabel(result.spec.split, rows.at(-1)!.key, rows.at(-1)!.label) : ""}</span>
        </div>
      </div>
    {/if}
    <details class="figures">
      <summary>Figures</summary>
      {@render figures()}
    </details>
  {/if}
  <figcaption class="muted">{periodLabel(result.spec.period)}</figcaption>
</figure>

<style>
  .report {
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: calc(var(--ui-spacing) * 3);
  }

  figcaption {
    font-size: 0.85rem;
  }

  .figure {
    margin: 0;
    font-size: 2.25rem;
    font-weight: 600;
    line-height: 1.1;
    font-variant-numeric: tabular-nums lining-nums;
  }

  .small .figure {
    font-size: 1.75rem;
  }

  .total {
    margin: 0;
    color: var(--ink-muted);
  }

  .total strong {
    color: var(--ink);
  }

  .num {
    text-align: end;
    font-variant-numeric: tabular-nums lining-nums;
  }

  /* Rows: one bar per group, the value at its tip. */
  .rows {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    grid-template-columns: minmax(6rem, max-content) 1fr max-content;
    align-items: center;
    gap: calc(var(--ui-spacing) * 2) calc(var(--ui-spacing) * 3);
  }

  .rows li {
    display: contents;
  }

  .row-value {
    font-variant-numeric: tabular-nums lining-nums;
  }

  .track {
    display: block;
    height: 14px;
  }

  .fill {
    display: block;
    height: 100%;
    max-width: 100%;
    background: var(--heather);
    border-radius: 0 4px 4px 0;
  }

  /* Columns and the line share an axis on the left. */
  .plot {
    display: grid;
    /* minmax(0, …) so the chart takes its card's width instead of pushing it wider. */
    grid-template-columns: 3rem minmax(0, 1fr);
    column-gap: calc(var(--ui-spacing) * 2);
  }

  .axis {
    position: relative;
    grid-row: 1;
    font-size: 0.75rem;
    color: var(--ui-muted-foreground);
  }

  .axis span {
    position: absolute;
    right: 0;
    transform: translateY(50%);
  }

  .columns {
    grid-column: 2;
    display: flex;
    align-items: flex-end;
    gap: 2px;
    border-bottom: 1px solid var(--ui-border);
    background-image: linear-gradient(to bottom, var(--ui-border) 1px, transparent 1px);
    background-size: 100% 50%;
  }

  .column {
    position: relative;
    padding: 0;
    border: 0;
    background: none;
    font: inherit;
    cursor: default;
    flex: 1;
    height: 100%;
    display: flex;
    align-items: flex-end;
    justify-content: center;
    outline: none;
  }

  .column:focus-visible {
    box-shadow: inset 0 0 0 2px var(--heather);
    border-radius: 4px;
  }

  .col-fill {
    width: min(100%, 24px);
    background: var(--heather);
    border-radius: 4px 4px 0 0;
  }

  .column:hover .col-fill,
  .column:focus-visible .col-fill {
    filter: brightness(1.15);
  }

  .x-labels {
    grid-column: 2;
    display: flex;
    gap: 2px;
    font-size: 0.75rem;
    color: var(--ui-muted-foreground);
    margin-top: calc(var(--ui-spacing) * 1);
  }

  .x-labels span {
    flex: 1;
    min-width: 0;
    text-align: center;
    white-space: nowrap;
  }

  .x-labels.ends {
    justify-content: space-between;
  }

  .x-labels.ends span {
    flex: none;
  }

  .line {
    grid-column: 2;
    min-width: 0;
    position: relative;
    border-bottom: 1px solid var(--ui-border);
  }

  .line svg {
    position: absolute;
    inset: 0;
    display: block;
    overflow: visible;
    outline: none;
  }

  .line path {
    fill: none;
    stroke: var(--heather);
    stroke-width: 2;
    stroke-linejoin: round;
    stroke-linecap: round;
  }

  .line .grid,
  .line .cross {
    stroke: var(--ui-border);
    stroke-width: 1;
  }

  .line circle {
    fill: var(--heather);
    stroke: var(--paper);
    stroke-width: 2;
  }

  .tip {
    position: absolute;
    bottom: calc(100% + 6px);
    z-index: 3;
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: calc(var(--ui-spacing) * 1.5) calc(var(--ui-spacing) * 2);
    border: 1px solid var(--ui-border);
    border-radius: var(--ui-radius);
    background: var(--paper);
    color: var(--ink-muted);
    font-size: 0.8rem;
    white-space: nowrap;
    pointer-events: none;
    box-shadow: 0 2px 8px rgb(0 0 0 / 0.12);
  }

  .column .tip {
    left: 50%;
    transform: translateX(-50%);
  }

  .line .tip {
    top: 0;
    bottom: auto;
    transform: translate(-50%, -110%);
  }

  .tip strong {
    color: var(--ink);
    font-size: 0.9rem;
  }

  .figures summary {
    cursor: pointer;
    color: var(--ink-muted);
    font-size: 0.875rem;
  }

  .figures-table {
    margin-top: calc(var(--ui-spacing) * 2);
  }
</style>
