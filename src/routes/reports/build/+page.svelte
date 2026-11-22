<script lang="ts">
  import { onMount } from "svelte";
  import { confirm } from "@ui-sdk/svelte/alert-dialog";
  import { Button } from "@ui-sdk/svelte/button";
  import { Field, FieldLabel } from "@ui-sdk/svelte/field";
  import { Input } from "@ui-sdk/svelte/input";
  import {
    Select,
    SelectContent,
    SelectGroup,
    SelectGroupLabel,
    SelectItem,
    SelectItemIndicator,
    SelectTrigger,
    SelectValue,
  } from "@ui-sdk/svelte/select";
  import { toast } from "@ui-sdk/svelte/toast";
  import { Toggle, ToggleGroup } from "@ui-sdk/svelte/toggle";
  import { goto } from "$app/navigation";
  import { page } from "$app/state";
  import { api, errorMessage, newId } from "#lib/api.ts";
  import type { ChartKind } from "#lib/bindings/ChartKind.ts";
  import type { MeasureInfo } from "#lib/bindings/MeasureInfo.ts";
  import type { Period } from "#lib/bindings/Period.ts";
  import type { ReportResult } from "#lib/bindings/ReportResult.ts";
  import type { ReportSpec } from "#lib/bindings/ReportSpec.ts";
  import type { SavedReport } from "#lib/bindings/SavedReport.ts";
  import type { Split } from "#lib/bindings/Split.ts";
  import { addDays, today } from "#lib/dates.ts";
  import ViewSwitch from "#lib/nav/ViewSwitch.svelte";
  import { reportViews } from "#lib/nav/views.ts";
  import ReportChart from "#lib/reports/ReportChart.svelte";
  import { chartLabel, isTimeSplit, periodLabel, relativePeriods, splitLabel } from "#lib/reports/format.ts";
  import { useSession } from "#lib/session/session.svelte.ts";

  const session = useSession();
  const allowed = $derived(session.can("view_reports"));

  let catalogue = $state<MeasureInfo[]>([]);
  let saved = $state<SavedReport[]>([]);
  /** The saved report being edited, or null for a new one. */
  let editing = $state<string | null>(page.url.searchParams.get("id"));
  let spec = $state<ReportSpec>({
    name: "",
    measure: "room_revenue",
    split: "month",
    period: { kind: "this_year" },
    chart: "bar",
  });
  /** Until a name is typed, it follows the measure and split. */
  let nameTyped = $state(false);
  let result = $state<ReportResult | null>(null);
  let problem = $state<string | null>(null);
  // Fixed dates, as shown: the last day is included.
  let fromDay = $state(today().slice(0, 8) + "01");
  let toDay = $state(today());

  const info = $derived(catalogue.find((m) => m.measure === spec.measure));
  const areas = $derived([...new Set(catalogue.map((m) => m.area))]);
  const measureItems = $derived(catalogue.map((m) => ({ value: m.measure, label: m.label })));
  const splitItems = $derived((info?.splits ?? ["none"]).map((s) => ({ value: s, label: splitLabel[s] })));
  const periodItems = [
    ...relativePeriods.map((kind) => ({ value: kind, label: periodLabel({ kind } as Period) })),
    { value: "fixed", label: "Fixed dates" },
  ];
  const charts: ChartKind[] = ["number", "bar", "line", "table"];
  const chartAllowed = (c: ChartKind) =>
    spec.split === "none" ? c === "number" : c !== "line" || isTimeSplit(spec.split);

  /** The name a report gets until one is typed: "Room revenue by month". */
  function autoNameFor(s: ReportSpec): string {
    const label = catalogue.find((m) => m.measure === s.measure)?.label ?? "";
    return s.split === "none" ? label : `${label} by ${splitLabel[s.split].toLowerCase()}`;
  }
  const autoName = $derived(autoNameFor(spec));
  $effect(() => {
    if (!nameTyped) spec.name = autoName;
  });

  function setMeasure(value: string | null) {
    const next = catalogue.find((m) => m.measure === value);
    if (!next) return;
    spec.measure = next.measure;
    // Keep the split if the new measure allows it; otherwise by month.
    if (!next.splits.includes(spec.split)) setSplit("month");
  }

  function setSplit(value: string | null) {
    if (!value) return;
    spec.split = value as Split;
    if (!chartAllowed(spec.chart)) spec.chart = spec.split === "none" ? "number" : "bar";
  }

  function setPeriod(value: string | null) {
    if (!value) return;
    spec.period =
      value === "fixed" ? { kind: "fixed", start: fromDay, end: addDays(toDay, 1) } : ({ kind: value } as Period);
  }

  function setFixed() {
    if (spec.period.kind === "fixed" && fromDay && toDay) {
      spec.period = { kind: "fixed", start: fromDay, end: addDays(toDay, 1) };
    }
  }

  // Run the report whenever it changes; only the latest answer is shown.
  let asked = 0;
  $effect(() => {
    if (!allowed || !catalogue.length) return;
    const snapshot = $state.snapshot(spec) as ReportSpec;
    const mine = ++asked;
    api
      .runReport({ ...snapshot, name: snapshot.name.trim() || autoName })
      .then((r) => {
        if (mine !== asked) return;
        result = r;
        problem = null;
      })
      .catch((error) => {
        if (mine !== asked) return;
        result = null;
        problem = errorMessage(error);
      });
  });

  async function load() {
    try {
      [catalogue, saved] = await Promise.all([api.reportCatalogue(), api.listSavedReports()]);
      const open = saved.find((r) => r.id === editing);
      if (open) openReport(open);
      else editing = null;
    } catch (error) {
      toast.error("Couldn't load the report builder", { description: errorMessage(error) });
    }
  }

  onMount(() => {
    if (allowed) void load();
  });

  function openReport(r: SavedReport) {
    editing = r.id;
    // A name that was never typed keeps following the measure and split.
    nameTyped = r.spec.name !== autoNameFor(r.spec);
    spec = $state.snapshot(r.spec) as ReportSpec; // a plain copy, not the list's own object
    if (r.spec.period.kind === "fixed") {
      fromDay = r.spec.period.start;
      toDay = addDays(r.spec.period.end, -1);
    }
    void goto(`/reports/build?id=${r.id}`, { replace: true, reset: false });
  }

  function startNew() {
    editing = null;
    nameTyped = false;
    void goto("/reports/build", { replace: true, reset: false });
  }

  async function save(asNew: boolean) {
    const id = asNew || !editing ? newId() : editing;
    try {
      await api.saveSavedReport({ id, spec: $state.snapshot(spec) as ReportSpec });
      toast.success(`Saved "${spec.name}"`);
      saved = await api.listSavedReports();
      editing = id;
      nameTyped = spec.name !== autoName;
      void goto(`/reports/build?id=${id}`, { replace: true, reset: false });
    } catch (error) {
      toast.error("Couldn't save the report", { description: errorMessage(error) });
    }
  }

  async function remove(r: SavedReport) {
    const yes = await confirm({
      title: `Delete "${r.spec.name}"?`,
      description: "It's also taken off any dashboard it's on.",
      confirmLabel: "Delete report",
      cancelLabel: "Cancel",
    });
    if (!yes) return;
    try {
      await api.deleteSavedReport(r.id);
      saved = saved.filter((s) => s.id !== r.id);
      if (editing === r.id) startNew();
      toast.success(`Deleted "${r.spec.name}"`);
    } catch (error) {
      toast.error("Couldn't delete the report", { description: errorMessage(error) });
    }
  }

  async function copyCsv() {
    try {
      await navigator.clipboard.writeText(await api.customReportCsv($state.snapshot(spec) as ReportSpec));
      toast.success("Copied as CSV", { description: "Paste it into a spreadsheet." });
    } catch (error) {
      toast.error("Couldn't copy the report", { description: errorMessage(error) });
    }
  }

  async function saveCsv() {
    try {
      const path = await api.saveCustomReportCsv($state.snapshot(spec) as ReportSpec);
      if (path) toast.success("Saved", { description: path });
    } catch (error) {
      toast.error("Couldn't save the report", { description: errorMessage(error) });
    }
  }

  const measureName = (r: SavedReport) => catalogue.find((m) => m.measure === r.spec.measure)?.label ?? "";
</script>

<div class="page wide">
  <h1>Build a report</h1>
  {#if !allowed}
    <p class="muted">Reports are for admins. Ask an admin to sign in.</p>
  {:else}
    <div class="row toolbar">
      <ViewSwitch views={reportViews} current="Build a report" label="Reports" />
    </div>
    <p class="page-intro">
      Choose what to count, how to split it and over which dates. The preview updates as you go; save it to open it
      again or put it on a dashboard.
    </p>

    <div class="builder">
      <form
        class="panel stack"
        aria-label="Report"
        onsubmit={(e) => {
          e.preventDefault();
          void save(false);
        }}
      >
        <Field>
          <FieldLabel>Report name</FieldLabel>
          <Input
            value={spec.name}
            required
            oninput={(e: Event) => {
              nameTyped = true;
              spec.name = (e.currentTarget as HTMLInputElement).value;
            }}
          />
        </Field>
        <Field>
          <FieldLabel>Measure</FieldLabel>
          <Select items={measureItems} value={spec.measure} onValueChange={setMeasure}>
            <SelectTrigger><SelectValue /></SelectTrigger>
            <SelectContent>
              {#each areas as area (area)}
                <SelectGroup>
                  <SelectGroupLabel>{area}</SelectGroupLabel>
                  {#each catalogue.filter((m) => m.area === area) as m (m.measure)}
                    <SelectItem value={m.measure}>{m.label}<SelectItemIndicator /></SelectItem>
                  {/each}
                </SelectGroup>
              {/each}
            </SelectContent>
          </Select>
        </Field>
        <Field>
          <FieldLabel>Split by</FieldLabel>
          <Select items={splitItems} value={spec.split} onValueChange={setSplit}>
            <SelectTrigger><SelectValue /></SelectTrigger>
            <SelectContent>
              {#each splitItems as s (s.value)}
                <SelectItem value={s.value}>{s.label}<SelectItemIndicator /></SelectItem>
              {/each}
            </SelectContent>
          </Select>
        </Field>
        <Field>
          <FieldLabel>Period</FieldLabel>
          <Select items={periodItems} value={spec.period.kind} onValueChange={setPeriod}>
            <SelectTrigger><SelectValue /></SelectTrigger>
            <SelectContent>
              {#each periodItems as p (p.value)}
                <SelectItem value={p.value}>{p.label}<SelectItemIndicator /></SelectItem>
              {/each}
            </SelectContent>
          </Select>
        </Field>
        {#if spec.period.kind === "fixed"}
          <div class="row">
            <Field>
              <FieldLabel>From</FieldLabel>
              <Input
                type="date"
                value={fromDay}
                onValueChange={(v) => {
                  if (v) {
                    fromDay = v;
                    setFixed();
                  }
                }}
              />
            </Field>
            <Field>
              <FieldLabel>To (included)</FieldLabel>
              <Input
                type="date"
                value={toDay}
                min={fromDay}
                onValueChange={(v) => {
                  if (v) {
                    toDay = v;
                    setFixed();
                  }
                }}
              />
            </Field>
          </div>
        {/if}
        <div class="stack-tight">
          <span id="chart-label" class="label">Show as</span>
          <ToggleGroup
            aria-labelledby="chart-label"
            value={[spec.chart]}
            deselectable={false}
            onValueChange={(v) => v[0] && (spec.chart = v[0] as ChartKind)}
          >
            {#each charts as c (c)}
              <Toggle value={c} disabled={!chartAllowed(c)}>{chartLabel[c]}</Toggle>
            {/each}
          </ToggleGroup>
        </div>
        <div class="row">
          <Button type="submit" data-variant="solid">{editing ? "Save" : "Save report"}</Button>
          {#if editing}
            <Button onclick={() => save(true)}>Save as new</Button>
            <Button onclick={startNew}>New report</Button>
          {/if}
        </div>
      </form>

      <section class="panel stack preview" aria-labelledby="preview-heading">
        <div class="preview-head">
          <h2 id="preview-heading">{spec.name || autoName}</h2>
          <div class="row">
            <Button onclick={copyCsv} disabled={!result}>Copy as CSV</Button>
            <Button onclick={saveCsv} disabled={!result}>Save as CSV…</Button>
          </div>
        </div>
        {#if problem}
          <p class="problem" role="alert">{problem}</p>
        {:else if result}
          <ReportChart {result} />
        {:else}
          <p class="muted">Working it out…</p>
        {/if}
      </section>
    </div>

    <section class="saved" aria-labelledby="saved-heading">
      <h2 id="saved-heading">Saved reports</h2>
      {#if saved.length}
        <table class="list">
          <thead><tr><th>Name</th><th>Measure</th><th>Period</th><th><span class="muted">Actions</span></th></tr></thead>
          <tbody>
            {#each saved as r (r.id)}
              <tr aria-current={r.id === editing ? "true" : undefined}>
                <td>{r.spec.name}</td>
                <td>{measureName(r)}</td>
                <td>{periodLabel(r.spec.period)}</td>
                <td class="actions">
                  <Button onclick={() => openReport(r)}>Open</Button>
                  <Button data-variant="destructive" onclick={() => remove(r)}>Delete</Button>
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      {:else}
        <p class="muted">None yet. Build one above and save it.</p>
      {/if}
    </section>
  {/if}
</div>

<style>
  .wide {
    max-width: 90rem;
  }

  .toolbar {
    margin-bottom: calc(var(--ui-spacing) * 3);
  }

  .builder {
    display: grid;
    grid-template-columns: minmax(18rem, 24rem) 1fr;
    gap: calc(var(--ui-spacing) * 4);
    align-items: start;
  }

  @media (max-width: 60rem) {
    .builder {
      grid-template-columns: 1fr;
    }
  }

  .stack-tight {
    display: flex;
    flex-direction: column;
    gap: calc(var(--ui-spacing) * 1.5);
  }

  .label {
    font-weight: 600;
    font-size: 0.875rem;
  }

  .preview {
    min-width: 0;
  }

  .preview-head {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: calc(var(--ui-spacing) * 3);
  }

  .preview-head h2 {
    margin: 0;
  }

  .problem {
    margin: 0;
    padding: calc(var(--ui-spacing) * 2) calc(var(--ui-spacing) * 3);
    border-left: 3px solid var(--brass);
    background: var(--brass-tint);
  }

  .saved {
    margin-top: calc(var(--ui-spacing) * 8);
  }

  .actions {
    display: flex;
    gap: calc(var(--ui-spacing) * 2);
    justify-content: flex-end;
  }
</style>
