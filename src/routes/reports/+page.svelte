<script lang="ts">
  import { Button } from "@ui-sdk/svelte/button";
  import { Field, FieldLabel } from "@ui-sdk/svelte/field";
  import { Input } from "@ui-sdk/svelte/input";
  import { toast } from "@ui-sdk/svelte/toast";
  import { api, errorMessage } from "#lib/api.ts";
  import type { DateRange } from "#lib/bindings/DateRange.ts";
  import type { DayOccupancy } from "#lib/bindings/DayOccupancy.ts";
  import type { MonthRevenue } from "#lib/bindings/MonthRevenue.ts";
  import type { RoomKpis } from "#lib/bindings/RoomKpis.ts";
  import { formatDay, today } from "#lib/dates.ts";
  import { formatMoney } from "#lib/format.ts";
  import { useSession } from "#lib/session/session.svelte.ts";
  import ViewSwitch from "#lib/nav/ViewSwitch.svelte";
  import { reportViews } from "#lib/nav/views.ts";

  const session = useSession();
  // Reports are admin only; the Rust commands refuse anyone else too.
  const allowed = $derived(session.can("view_reports"));

  /** "2026-06" → that month's first day up to the next month's first day. */
  function monthRange(month: string): DateRange {
    const [y, m] = month.split("-").map(Number);
    const next = m === 12 ? `${y + 1}-01` : `${y}-${String(m + 1).padStart(2, "0")}`;
    return { start: `${month}-01`, end: `${next}-01` };
  }

  function yearRange(month: string): DateRange {
    const y = Number(month.slice(0, 4));
    return { start: `${y}-01-01`, end: `${y + 1}-01-01` };
  }

  let month = $state(today().slice(0, 7));
  let days = $state<DayOccupancy[]>([]);
  let revenue = $state<MonthRevenue[]>([]);
  let kpis = $state<RoomKpis | null>(null);

  const occupied = $derived(days.reduce((n, d) => n + d.occupied, 0));
  const available = $derived(days.reduce((n, d) => n + d.rooms, 0));
  const pct = (perMille: number) => `${(perMille / 10).toFixed(1)}%`;
  const totalPct = $derived(available ? pct(Math.round((occupied * 1000) / available)) : "0.0%");

  $effect(() => {
    const m = month;
    if (!allowed) return;
    api
      .occupancyReport(monthRange(m))
      .then((d) => (days = d))
      .catch((error) => toast.error("Couldn't load occupancy", { description: errorMessage(error) }));
    api
      .roomKpis(monthRange(m))
      .then((k) => (kpis = k))
      .catch(() => (kpis = null));
    api
      .revenueReport(yearRange(m))
      .then((r) => (revenue = r))
      .catch((error) => toast.error("Couldn't load revenue", { description: errorMessage(error) }));
  });

  async function saveCsv(report: "occupancy" | "revenue") {
    try {
      const range = report === "occupancy" ? monthRange(month) : yearRange(month);
      const path = await api.saveReportCsv(report, range);
      if (path) toast.success("Saved", { description: path });
    } catch (error) {
      toast.error("Couldn't save the report", { description: errorMessage(error) });
    }
  }

  async function copyCsv(report: "occupancy" | "revenue") {
    try {
      const range = report === "occupancy" ? monthRange(month) : yearRange(month);
      await navigator.clipboard.writeText(await api.reportCsv(report, range));
      toast.success("Copied as CSV", { description: "Paste it into a spreadsheet." });
    } catch (error) {
      toast.error("Couldn't copy the report", { description: errorMessage(error) });
    }
  }

  const monthName = (y: number, m: number) =>
    new Date(Date.UTC(y, m - 1, 1)).toLocaleDateString("en-GB", { month: "long", year: "numeric", timeZone: "UTC" });
</script>

<div class="page wide">
  <h1>Reports</h1>
  {#if !allowed}
    <p class="muted">Reports are for admins. Ask an admin to sign in.</p>
  {:else}
  <div class="row toolbar">
    <ViewSwitch views={reportViews} current="Standard" label="Reports" />
    <Field>
      <FieldLabel>Month</FieldLabel>
      <Input type="month" value={month} onValueChange={(v) => v && (month = v)} />
    </Field>
  </div>

  <section class="panel stack" aria-labelledby="occupancy-heading">
    <div class="head">
      <h2 id="occupancy-heading">Occupancy, {monthName(Number(month.slice(0, 4)), Number(month.slice(5, 7)))}</h2>
      <div class="row">
        <Button onclick={() => saveCsv("occupancy")}>Save as CSV…</Button>
        <Button onclick={() => copyCsv("occupancy")}>Copy as CSV</Button>
      </div>
    </div>
    <p class="summary"><strong>{totalPct}</strong> full: {occupied} of {available} room-nights sold.</p>
    {#if kpis}
      <p class="kpis" aria-label="Room rates">
        <span>Average rate per night sold (ADR) <strong>{formatMoney(kpis.adr)}</strong></span>
        <span>Revenue per available room (RevPAR) <strong>{formatMoney(kpis.revpar)}</strong></span>
      </p>
    {/if}
    <div class="bars" role="list" aria-label="Occupancy by night">
      {#each days as d (d.date)}
        <div
          class="bar"
          role="listitem"
          aria-label={`${formatDay(d.date, { weekday: "short", day: "numeric", month: "short" })}: ${pct(d.per_mille)}, ${d.occupied} of ${d.rooms}`}
        >
          <span class="fill" style:height="{d.per_mille / 10}%"></span>
          <span class="day">{Number(d.date.slice(8))}</span>
        </div>
      {/each}
    </div>
  </section>

    <section class="panel stack revenue" aria-labelledby="revenue-heading">
      <div class="head">
        <h2 id="revenue-heading">Revenue, {month.slice(0, 4)}</h2>
        <div class="row">
          <Button onclick={() => saveCsv("revenue")}>Save as CSV…</Button>
          <Button onclick={() => copyCsv("revenue")}>Copy as CSV</Button>
        </div>
      </div>
      {#if revenue.length}
        <table class="list">
          <thead>
            <tr><th>Month</th><th class="num">Rooms</th><th class="num">Board</th><th class="num">Suites</th><th class="num">Extras</th><th class="num">Total</th></tr>
          </thead>
          <tbody>
            {#each revenue as r (`${r.year}-${r.month}`)}
              <tr>
                <td>{monthName(r.year, r.month)}</td>
                <td class="num">{formatMoney(r.rooms)}</td>
                <td class="num">{formatMoney(r.board)}</td>
                <td class="num">{formatMoney(r.suites)}</td>
                <td class="num">{formatMoney(r.extras)}</td>
                <td class="num"><strong>{formatMoney(r.total)}</strong></td>
              </tr>
            {/each}
          </tbody>
        </table>
        <p class="muted">Amounts include VAT, as charged. Rooms and board count on the night stayed.</p>
      {:else}
        <p class="muted">No revenue recorded this year.</p>
      {/if}
    </section>
  {/if}
</div>

<style>
  .wide {
    max-width: 72rem;
  }

  .toolbar {
    margin-bottom: calc(var(--ui-spacing) * 4);
  }

  .toolbar > :global([data-slot="field"]) {
    flex: 0 1 12rem;
  }

  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: calc(var(--ui-spacing) * 3);
  }

  .head h2,
  .summary {
    margin: 0;
  }

  .summary {
    font-size: 1.1rem;
  }

  .kpis {
    display: flex;
    flex-wrap: wrap;
    gap: calc(var(--ui-spacing) * 6);
    margin: 0;
  }

  .bars {
    display: flex;
    align-items: flex-end;
    gap: 3px;
    height: 10rem;
    padding-top: calc(var(--ui-spacing) * 2);
  }

  .bar {
    flex: 1;
    height: 100%;
    display: flex;
    flex-direction: column;
    justify-content: flex-end;
    align-items: center;
  }

  .fill {
    width: 100%;
    min-height: 2px;
    border-radius: 3px 3px 0 0;
    background: var(--estuary);
  }

  .day {
    font-size: 0.7rem;
    color: var(--ink-muted);
  }

  .revenue {
    margin-top: calc(var(--ui-spacing) * 4);
  }

  .num {
    text-align: right;
    font-variant-numeric: tabular-nums lining-nums;
  }
</style>
